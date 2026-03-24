//! HTTP routes for the LLM proxy and secure Put (envelope encryption).

use axum::{
    body::Bytes,
    extract::{Extension, FromRef, Query, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use futures_util::Stream;
use rand::RngCore;
use sqlx::Row;
use std::sync::Arc;
use tracing::Instrument;

use crate::auth::api_token::Auth;
use crate::auth::crypto::{hash_password, verify_password};
use crate::auth::keygen;
use crate::auth::session::hash_token;
use crate::auth::ApiKeyScope;
use crate::analytics::AnalyticsReporter;
use crate::db::DbFunctionStore;
use crate::execute::{ExecuteState};
use crate::observability::{
    self, observability_middleware, HealthJson, ReadinessState,
};
use crate::observability::request_id::ObservabilityShared;
use crate::put::{PutFunctionService, PutKeyRequestBody, PutPromptRequestBody, PutStorageResponse, secret_fingerprint};
use metrics_exporter_prometheus::PrometheusHandle;
use crate::routes::request::ExecuteRequest;
use crate::secrets::{EnvelopeError, SecretEnveloper};
use sha2::{Digest, Sha256};

fn default_surface() -> String {
    "unknown".to_string()
}

pub mod request {
    use serde::Deserialize;
    use std::collections::HashMap;

    fn default_surface() -> String {
        "unknown".to_string()
    }

    /// POST /v1/execute body: function_id + variables map.
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ExecuteRequest {
        pub function_id: String,
        #[serde(default)]
        pub variables: HashMap<String, serde_json::Value>,
        /// Optional: prefer this provider (e.g. "openai", "anthropic", "gemini") for this request.
        pub provider: Option<String>,
        /// Optional: model override. Takes precedence over prompt version default.
        /// If omitted, Anthropic defaults to `claude-sonnet-4-6` (server-side default).
        pub model: Option<String>,
        /// Optional client-facing interface tag, e.g. "cli", "android", "web". Defaults to "unknown".
        #[serde(default = "default_surface")]
        pub surface: String,
    }

}

/// GET /v1/list-prompts query parameters.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ListPromptsQuery {
    /// Client surface tag for analytics (e.g. `"cli"`, `"android"`). Default: `"unknown"`.
    #[serde(default = "default_surface")]
    surface: String,
}

/// Response: stored prompt **titles** (function names) only.
#[derive(Debug, serde::Serialize)]
struct ListPromptsResponse {
    /// Production deployment names for this workspace and global (`context_id` `''`), sorted.
    titles: Vec<String>,
}

/// In-memory rate limiter: one register per IP per window (e.g. 3 minutes).
#[derive(Default)]
pub struct RegisterRateLimiter {
    last_by_ip: RwLock<HashMap<String, Instant>>,
}

const REGISTER_RATE_WINDOW: Duration = Duration::from_secs(3 * 60); // 3 minutes

impl RegisterRateLimiter {
    /// Returns true if the request is allowed, false if rate limited. Records the attempt when allowed.
    pub fn check_and_record(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut map = self.last_by_ip.write().expect("register rate limiter lock");
        // Prune expired entries to avoid unbounded growth
        map.retain(|_, t| now.saturating_duration_since(*t) <= REGISTER_RATE_WINDOW);
        if let Some(&last) = map.get(ip) {
            if now.saturating_duration_since(last) < REGISTER_RATE_WINDOW {
                return false;
            }
        }
        map.insert(ip.to_string(), now);
        true
    }
}

// --- Proof-of-Work for register ---

/// Number of leading zero bits required in SHA256(nonce || valid_until || solution).
const REGISTER_POW_DIFFICULTY: u32 = 4;
/// Challenge validity window (client must submit solution before this expires).
const REGISTER_POW_VALIDITY: chrono::Duration = chrono::Duration::minutes(5);

/// Response for GET /v1/auth/register-challenge.
#[derive(Debug, serde::Serialize)]
struct RegisterChallengeResponse {
    /// Hex-encoded random nonce (16 bytes). Client must use this exact value when computing solution.
    pub nonce: String,
    /// Required leading zero bits in the PoW hash.
    pub difficulty: u32,
    /// ISO8601 timestamp. Solution is accepted only if the server time is before this.
    pub valid_until: String,
}

/// Count leading zero bits in a 32-byte hash (big-endian).
fn leading_zero_bits(hash: &[u8; 32]) -> u32 {
    let mut bits = 0u32;
    for &b in hash {
        if b == 0 {
            bits += 8;
        } else {
            bits += b.leading_zeros();
            break;
        }
    }
    bits
}

/// Verify PoW: SHA256(nonce_hex_decoded || valid_until_utf8 || solution_utf8) has >= difficulty leading zero bits.
/// Returns an error message if invalid.
fn verify_register_pow(
    nonce_hex: &str,
    valid_until: &str,
    solution: &str,
    difficulty: u32,
) -> Result<(), String> {
    let nonce_bytes = hex::decode(nonce_hex.trim()).map_err(|e| format!("invalid nonce hex: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&nonce_bytes);
    hasher.update(valid_until.as_bytes());
    hasher.update(solution.as_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    let bits = leading_zero_bits(&hash);
    if bits < difficulty {
        return Err(format!("insufficient proof-of-work: got {} leading zero bits, need {}", bits, difficulty));
    }
    Ok(())
}

/// GET /v1/auth/register-challenge — returns nonce, difficulty, valid_until for client to compute PoW.
async fn register_challenge_handler() -> Json<RegisterChallengeResponse> {
    let mut nonce = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let valid_until = Utc::now() + REGISTER_POW_VALIDITY;
    Json(RegisterChallengeResponse {
        nonce: hex::encode(nonce),
        difficulty: REGISTER_POW_DIFFICULTY,
        valid_until: valid_until.to_rfc3339(),
    })
}

/// Client IP from X-Forwarded-For (first client) or X-Real-IP. None if neither is present (request is not rate limited).
fn client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(v) = headers.get("x-forwarded-for") {
        if let Ok(s) = v.to_str() {
            let first = s.split(',').next().map(str::trim).unwrap_or("").to_string();
            if !first.is_empty() {
                return Some(first);
            }
        }
    }
    if let Some(v) = headers.get("x-real-ip") {
        if let Ok(s) = v.to_str() {
            let s = s.trim();
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Application state: execute pipeline + optional envelope encryption and Put service.
#[derive(Clone)]
pub struct AppState {
    pub execute: ExecuteState,
    /// Present when KMS is configured (KMS_KEY_ID env); enables POST /v1/keys and POST /v1/prompts.
    pub secrets: Option<Arc<SecretEnveloper>>,
    /// Put function service (requires secrets).
    pub put_service: Option<Arc<PutFunctionService>>,
    /// Shared Postgres connection pool.
    pub db: sqlx::PgPool,
    /// Rate limiter for register: 1 IP per 3 minutes.
    pub register_rate_limiter: Arc<RegisterRateLimiter>,
    /// Async analytics reporter (PostHog) with a dedicated worker thread.
    pub analytics: AnalyticsReporter,
    /// Prometheus scrape handle (`GET /metrics`).
    pub prometheus_handle: PrometheusHandle,
    /// Startup readiness (DB function catalog loaded).
    pub readiness: Arc<ReadinessState>,
}

impl FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}


/// GET /health — for Docker and load balancer healthchecks.
async fn health_handler() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// Build the app router with shared state. Pass `secrets` when KMS is configured.
pub async fn app_router(
    secrets: Option<Arc<SecretEnveloper>>,
    db: sqlx::PgPool,
    prometheus_handle: PrometheusHandle,
) -> Result<Router<()>, crate::db::LoadError> {
    let function_store = Arc::new(DbFunctionStore::new(db.clone(), secrets.clone()));
    let config_loaded = function_store.load_from_db().await.is_ok();
    if !config_loaded {
        tracing::warn!("load_from_db failed (schema 001 may not be applied); using empty function store");
    }
    function_store.seed_default_if_empty();

    let put_service = secrets.as_ref().map(|e| {
        Arc::new(
            PutFunctionService::new(Arc::clone(e), db.clone())
                .with_function_store(Arc::clone(&function_store)),
        )
    });

    let execute = ExecuteState {
        functions: function_store,
        db: db.clone(),
        enveloper: secrets.clone(),
    };

    let register_rate_limiter = Arc::new(RegisterRateLimiter::default());
    let analytics = AnalyticsReporter::from_env();
    let readiness = Arc::new(ReadinessState { config_loaded });
    Ok(Router::new()
        .route("/health", get(health_handler))
        .route("/health/live", get(health_live_route))
        .route("/health/ready", get(health_ready_route))
        .route("/metrics", get(metrics_route))
        .route("/v1/execute", post(execute_handler))
        .route("/v1/list-prompts", get(list_prompts_handler))
        .route("/v1/auth/register-challenge", get(register_challenge_handler))
        .route("/v1/auth/register", post(register_handler))
        .route("/v1/auth/api-tokens", post(create_api_token_handler))
        .route("/v1/auth/login", post(login_handler))
        .route("/v1/keys", post(put_key_handler))
        .route("/v1/prompts", post(put_prompt_handler))
        .layer(middleware::from_fn(observability_middleware))
        .with_state(AppState {
            execute,
            secrets,
            put_service,
            db,
            register_rate_limiter,
            analytics,
            prometheus_handle,
            readiness,
        }))
}

/// GET /metrics — Prometheus text exposition.
async fn metrics_route(State(state): State<AppState>) -> (StatusCode, [(axum::http::HeaderName, &'static str); 1], String) {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.prometheus_handle.render(),
    )
}

/// GET /health/live — liveness.
async fn health_live_route() -> (StatusCode, Json<HealthJson>) {
    (
        StatusCode::OK,
        Json(HealthJson { status: "ok" }),
    )
}

/// GET /health/ready — DB + catalog load OK.
async fn health_ready_route(State(state): State<AppState>) -> (StatusCode, Json<HealthJson>) {
    if observability::health::is_ready(&state.db, &state.readiness).await {
        (
            StatusCode::OK,
            Json(HealthJson { status: "ok" }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthJson { status: "not_ready" }),
        )
    }
}

/// Request body for user registration.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    #[serde(default = "default_surface")]
    pub surface: String,
}

/// Response body for successful registration (no password fields).
#[derive(Debug, serde::Serialize)]
struct RegisterResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Default workspace created at signup.
    pub default_workspace_id: uuid::Uuid,
    /// API key for the default workspace. Returned only once; store securely.
    pub api_key: String,
    /// Always `"mgt"`: management key (full access).
    pub api_key_scope: &'static str,
}

/// POST /v1/auth/register: create user, default workspace, workspace_members, and API key.
/// Rate limited: 1 request per IP per 3 minutes.
async fn register_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, Json<serde_json::Value>)> {
    let surface = body.surface.clone();
    if let Some(client_ip) = client_ip_from_headers(&headers) {
        if !state.register_rate_limiter.check_and_record(&client_ip) {
            state.analytics.track_register_failed(&surface, "rate_limited");
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "registration rate limited: one attempt per IP per 3 minutes"
                })),
            ));
        }
    }

    // Proof-of-work verification (required for register).
    let pow_nonce = headers
        .get("x-pow-nonce")
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    let pow_solution = headers
        .get("x-pow-solution")
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    let pow_valid_until = headers
        .get("x-pow-valid-until")
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    let (pow_nonce, pow_solution, pow_valid_until) = match (pow_nonce, pow_solution, pow_valid_until) {
        (Some(n), Some(s), Some(v)) if !n.is_empty() && !s.is_empty() && !v.is_empty() => (n, s, v),
        _ => {
            state.analytics.track_register_failed(&surface, "pow_headers_missing");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "proof-of-work required: request must include headers X-Pow-Nonce, X-Pow-Solution, X-Pow-Valid-Until (get challenge from GET /v1/auth/register-challenge)"
                })),
            ));
        }
    };
    let valid_until_dt = match chrono::DateTime::parse_from_rfc3339(pow_valid_until) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            state.analytics.track_register_failed(&surface, "pow_valid_until_invalid");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid X-Pow-Valid-Until: must be ISO8601 (e.g. 2025-02-06T12:00:00Z)"
                })),
            ));
        }
    };
    if Utc::now() > valid_until_dt {
        state.analytics.track_register_failed(&surface, "pow_expired");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "proof-of-work challenge expired; request a new challenge from GET /v1/auth/register-challenge"
            })),
        ));
    }
    if let Err(e) = verify_register_pow(pow_nonce, pow_valid_until, pow_solution, REGISTER_POW_DIFFICULTY) {
        state.analytics.track_register_failed(&surface, "pow_invalid");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("invalid proof-of-work: {}", e)
            })),
        ));
    }

    // Basic validation – keep raw password in memory as short as possible.
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') {
        state.analytics.track_register_failed(&surface, "invalid_email");
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "invalid email" }))));
    }
    if body.password.len() < 12 {
        state.analytics.track_register_failed(&surface, "password_too_short");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "password must be at least 12 characters" })),
        ));
    }

    // Hash password with Argon2id (see auth::crypto). No logging of raw password.
    let password = body.password;
    let hashed = hash_password(&password).map_err(|_| {
        state.analytics.track_register_failed(&surface, "hashing_failed");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "hashing failed" })))
    })?;

    // Drop raw password ASAP.
    drop(password);

    let name = body.name.clone();

    // Run all inserts in a transaction.
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to begin transaction");
        state.analytics.track_register_failed(&surface, "transaction_begin_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    // 1. Insert user
    let row = sqlx::query(
        "INSERT INTO users (email, password_hash, name) \
         VALUES ($1, $2, $3) \
         RETURNING id, email, name, created_at",
    )
    .bind(&email)
    .bind(&hashed)
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().map(|c| c == "23505").unwrap_or(false) {
                state.analytics.track_register_failed(&surface, "email_already_registered");
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "error": "email already registered" })),
                );
            }
        }
        tracing::error!(error = ?e, "failed to insert user");
        state.analytics.track_register_failed(&surface, "insert_user_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    let id: uuid::Uuid = row.try_get("id").unwrap();
    let email: String = row.try_get("email").unwrap();
    let name: Option<String> = row.try_get("name").unwrap();
    let created_at: DateTime<Utc> = row.try_get("created_at").unwrap();

    // 2. Create default workspace (slug must be unique; use user_id to avoid collisions)
    let workspace_slug = format!("{}-personal", id);
    let workspace_row = sqlx::query(
        "INSERT INTO workspaces (name, slug) VALUES ($1, $2) RETURNING id",
    )
    .bind("Personal")
    .bind(&workspace_slug)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create workspace");
        state.analytics.track_register_failed(&surface, "create_workspace_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    let workspace_id: uuid::Uuid = workspace_row.try_get("id").unwrap();

    // 3. Add user as owner in workspace_members
    sqlx::query(
        "INSERT INTO workspace_members (workspace_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(workspace_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to add workspace member");
        state.analytics.track_register_failed(&surface, "add_workspace_member_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    // 4. Generate management API key and store hash (plaintext returned once)
    let api_key = keygen::generate_management_key();
    let token_hash = hash_token(&api_key);

    sqlx::query(
        "INSERT INTO api_tokens (user_id, workspace_id, token_hash, label, scope) VALUES ($1, $2, $3, $4, 'mgt')",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(&token_hash)
    .bind("Default")
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create API token");
        state.analytics.track_register_failed(&surface, "create_api_token_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit registration");
        state.analytics.track_register_failed(&surface, "commit_failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    state.analytics.track_user_registered(&surface, id, workspace_id);

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id,
            email,
            name,
            created_at,
            default_workspace_id: workspace_id,
            api_key,
            api_key_scope: "mgt",
        }),
    ))
}

fn default_exe_token_label() -> String {
    "Execution".to_string()
}

/// Request body for POST /v1/auth/api-tokens (mint execution-only key).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateApiTokenRequest {
    /// Human-readable label (e.g. "Mobile app"). Defaults to `"Execution"`.
    #[serde(default = "default_exe_token_label")]
    pub label: String,
    #[serde(default = "default_surface")]
    pub surface: String,
}

/// Response: new execution-only API key (shown once).
#[derive(Debug, serde::Serialize)]
struct CreateApiTokenResponse {
    pub api_key: String,
    /// Always `exe` for keys created via this endpoint.
    pub scope: &'static str,
    pub label: String,
}

/// POST /v1/auth/api-tokens — mint an execution-only key (`pk_exe_live_...`).
/// Requires a **management** API key or a **session** token (not an execution-only key).
async fn create_api_token_handler(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateApiTokenRequest>,
) -> Result<(StatusCode, Json<CreateApiTokenResponse>), (StatusCode, Json<serde_json::Value>)> {
    if matches!(auth.api_key_scope, Some(ApiKeyScope::Exe)) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "execution-only API key cannot create tokens" })),
        ));
    }
    let trimmed = body.label.trim();
    let label = if trimmed.is_empty() {
        default_exe_token_label()
    } else {
        trimmed.to_string()
    };
    let api_key = keygen::generate_execution_key();
    let token_hash = hash_token(&api_key);
    sqlx::query(
        "INSERT INTO api_tokens (user_id, workspace_id, token_hash, label, scope) VALUES ($1, $2, $3, $4, 'exe')",
    )
    .bind(auth.user_id)
    .bind(auth.workspace_id)
    .bind(&token_hash)
    .bind(&label)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert execution API token");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create API token" })),
        )
    })?;
    let _ = body.surface;
    Ok((
        StatusCode::CREATED,
        Json(CreateApiTokenResponse {
            api_key,
            scope: "exe",
            label,
        }),
    ))
}

/// Request body for login.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(default = "default_surface")]
    pub surface: String,
}

/// Response body for successful login (token + user; no password).
#[derive(Debug, serde::Serialize)]
struct LoginResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub user: LoginUser,
}

#[derive(Debug, serde::Serialize)]
struct LoginUser {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: Option<String>,
}

/// POST /v1/auth/login: verify password, create session, return token.
/// Generic "invalid email or password" on failure to avoid user enumeration.
async fn login_handler(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let surface = body.surface.clone();
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') {
        state.analytics.track_login_failed(&surface, "invalid_credentials");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid email or password" })),
        ));
    }

    // Look up user by email; only password users (password_hash not null).
    let row = sqlx::query(
        "SELECT id, email, name, password_hash FROM users WHERE email = $1 AND password_hash IS NOT NULL",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "login db error");
        state.analytics.track_login_failed(&surface, "db_error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "login failed" })),
        )
    })?;

    let row = match row {
        Some(r) => r,
        None => {
            state.analytics.track_login_failed(&surface, "invalid_credentials");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid email or password" })),
            ))
        }
    };

    let user_id: uuid::Uuid = row.try_get("id").unwrap();
    let user_email: String = row.try_get("email").unwrap();
    let user_name: Option<String> = row.try_get("name").unwrap();
    let password_hash: String = row.try_get("password_hash").unwrap();

    // Verify password; drop raw password from memory ASAP.
    let password = body.password;
    let ok = verify_password(&password, &password_hash).unwrap_or(false);
    drop(password);

    if !ok {
        state.analytics.track_login_failed(&surface, "invalid_credentials");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid email or password" })),
        ));
    }

    // Generate secure session token (256-bit). Store only hash; plaintext returned once.
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = hex::encode(bytes);

    let token_hash = {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Session expires in 7 days.
    let expires_at = Utc::now() + chrono::Duration::days(7);

    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to create session");
            state.analytics.track_login_failed(&surface, "db_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "login failed" })),
            )
        })?;

    state.analytics.track_user_logged_in(&surface, user_id);

    Ok(Json(LoginResponse {
        token,
        expires_at,
        user: LoginUser {
            id: user_id,
            email: user_email,
            name: user_name,
        },
    }))
}

/// GET /v1/list-prompts — list stored prompt titles (production deployments) for the auth workspace
/// plus global (`context_id` empty). Management or execution API key, or session token.
async fn list_prompts_handler(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListPromptsQuery>,
) -> Result<Json<ListPromptsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start = std::time::Instant::now();
    let surface = query.surface.clone();
    let context_id = auth.workspace_id.to_string();

    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT f.name
        FROM deployments d
        JOIN functions f ON f.id = d.function_id
        WHERE d.tag = 'production'
          AND (d.context_id = $1 OR d.context_id = '')
        ORDER BY f.name
        "#,
    )
    .bind(&context_id)
    .fetch_all(&state.db)
    .await;

    let titles = match rows {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = ?e, "list_prompts query failed");
            state.analytics.track_prompts_list_failed(
                &surface,
                auth.user_id,
                auth.workspace_id,
                "db_error",
                start.elapsed().as_millis(),
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to list prompts" })),
            ));
        }
    };

    state.analytics.track_prompts_listed(
        &surface,
        auth.user_id,
        auth.workspace_id,
        titles.len(),
        start.elapsed().as_millis(),
    );

    Ok(Json(ListPromptsResponse { titles }))
}

/// POST /v1/execute: parse body, run execute via LangChain, stream SSE back.
/// Requires Authorization: Bearer <api_token> or X-API-Key: <api_token> (scoped `pk_mgt_live_` / `pk_exe_live_` key or session token).
async fn execute_handler(
    State(state): State<AppState>,
    auth: Auth,
    Extension(obs): Extension<ObservabilityShared>,
    body: Bytes,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>> + Send + 'static> {
    let execute_state = state.execute;
    let analytics = state.analytics;
    let start = Instant::now();

    type SseStream = Pin<Box<dyn Stream<Item = Result<Event, axum::Error>> + Send + 'static>>;

    let req: ExecuteRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(err = %e, "execute request body parse failed");
            crate::observability::metrics::apply_execute_failure(
                &obs,
                &crate::execute::ExecuteError::Other(format!("json: {e}")),
            );
            analytics.track_execute_request_parse_error(auth.user_id, auth.workspace_id);
            let event = Event::default()
                .json_data(serde_json::json!({ "error": e.to_string() }))
                .unwrap();
            let s: SseStream = Box::pin(futures_util::stream::iter(vec![Ok(event)]));
            return Sse::new(s);
        }
    };

    let function_id = req.function_id.clone();
    let surface = req.surface.clone();
    crate::observability::metrics::set_proxy_fields(
        &obs,
        &function_id,
        req.provider.clone(),
        req.model.clone(),
    );
    let span = tracing::info_span!("execute", function_id = %function_id);

    let context_id = auth.workspace_id.to_string();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        crate::execute::execute_request(
            execute_state,
            req,
            &context_id,
            auth.user_id,
            auth.workspace_id,
        )
        .instrument(span),
    )
    .await;

    let event_stream: SseStream = match result {
            Ok(Ok(stream)) => {
                let latency_ms = start.elapsed().as_millis();
                tracing::info!(
                    function_id = %function_id,
                    latency_ms = %latency_ms,
                    "execute stream ready"
                );
                analytics.track_proxy_success(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    latency_ms,
                );
                analytics.track_added_latency(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    latency_ms,
                );
                Box::pin(stream)
            }
            Ok(Err(e)) => {
                tracing::warn!(err = %e, "execute failed");
                crate::observability::metrics::apply_execute_failure(&obs, &e);
                let latency_ms = start.elapsed().as_millis();
                analytics.track_proxy_error(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    &e.to_string(),
                    latency_ms,
                );
                analytics.track_added_latency(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    latency_ms,
                );
                let event = crate::execute::execute_error_to_event(&e);
                Box::pin(futures_util::stream::iter(vec![Ok(event)]))
            }
            Err(_) => {
                tracing::warn!("execute exceeded 60s timeout");
                crate::observability::metrics::apply_execute_failure(
                    &obs,
                    &crate::execute::ExecuteError::Other(
                        "execute exceeded 60s client timeout".into(),
                    ),
                );
                let latency_ms = start.elapsed().as_millis();
                analytics.track_proxy_error(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    "execute exceeded 60s client timeout",
                    latency_ms,
                );
                analytics.track_added_latency(
                    "/v1/execute",
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    latency_ms,
                );
                let event = crate::execute::execute_error_to_event(
                    &crate::execute::ExecuteError::Other(
                        "execute exceeded 60s client timeout".into(),
                    ),
                );
                Box::pin(futures_util::stream::iter(vec![Ok(event)]))
            }
        };

    Sse::new(event_stream)
}

fn map_put_error(e: crate::put::PutServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let (status, msg): (_, String) = match &e {
        crate::put::PutServiceError::Envelope(EnvelopeError::Kms(err)) => {
            tracing::warn!(err = %err, "KMS encrypt failed");
            (axum::http::StatusCode::BAD_GATEWAY, "KMS connection or config failed".into())
        }
        crate::put::PutServiceError::Envelope(EnvelopeError::KmsConfig(err)) => {
            tracing::warn!(err = %err, "KMS config failed");
            (axum::http::StatusCode::BAD_GATEWAY, "KMS connection or config failed".into())
        }
        crate::put::PutServiceError::Envelope(EnvelopeError::KmsDecrypt(e)) => {
            tracing::warn!(err = %e, "KMS decrypt failed");
            (axum::http::StatusCode::BAD_GATEWAY, "KMS decrypt failed".into())
        }
        crate::put::PutServiceError::Envelope(_) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "encryption failed".into())
        }
        crate::put::PutServiceError::Validation(_) => (axum::http::StatusCode::BAD_REQUEST, e.to_string()),
        crate::put::PutServiceError::Db(_) => {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "storage failed".into())
        }
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// POST /v1/keys: store a provider API key. Requires raw_secret and provider.
/// Location: /v1/keys. Requires auth.
async fn put_key_handler(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<PutKeyRequestBody>,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let surface = body.surface.clone();
    let service = state.put_service.as_ref().ok_or_else(|| {
        state.analytics.track_put_endpoint_unavailable(
            "/v1/keys",
            &surface,
            auth.user_id,
            auth.workspace_id,
        );
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "secrets not configured (KMS_KEY_ID required)" })),
        )
    })?;

    let context_id = auth.workspace_id.to_string();
    let fingerprint = secret_fingerprint(body.raw_secret.as_str());
    let provider_for_analytics = body.provider.trim().to_string();

    let result = service
        .store_key(
            &body.provider,
            body.raw_secret.as_str(),
            &context_id,
            auth.user_id,
            auth.workspace_id,
        )
        .await;

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            if e.is_provider_not_supported_for_key() {
                state.analytics.track_key_store_provider_not_supported(
                    &surface,
                    auth.user_id,
                    auth.workspace_id,
                    &provider_for_analytics,
                );
            }
            return Err(map_put_error(e));
        }
    };

    state.analytics.track_key_stored(
        &surface,
        auth.user_id,
        auth.workspace_id,
        &provider_for_analytics,
    );

    let response = PutStorageResponse {
        version_id: result.version_id,
        created_at: result.created_at,
        kms_key_arn: result.kms_key_arn.clone(),
        fingerprint,
    };
    drop(body);
    Ok((
        axum::http::StatusCode::CREATED,
        [(axum::http::header::LOCATION, axum::http::header::HeaderValue::from_static("/v1/keys"))],
        Json(response),
    ))
}

/// POST /v1/prompts: store a prompt template. Requires name and raw_secret; provider optional.
/// Location: /v1/functions/{name}/versions/{version_id}. Requires auth.
async fn put_prompt_handler(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<PutPromptRequestBody>,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let surface = body.surface.clone();
    let service = state.put_service.as_ref().ok_or_else(|| {
        state.analytics.track_put_endpoint_unavailable(
            "/v1/prompts",
            &surface,
            auth.user_id,
            auth.workspace_id,
        );
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "secrets not configured (KMS_KEY_ID required)" })),
        )
    })?;

    let context_id = auth.workspace_id.to_string();
    let fingerprint = secret_fingerprint(body.raw_secret.as_str());
    let provider_label = body
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown")
        .to_string();

    let result = service
        .store_prompt(
            &body.name,
            body.raw_secret.as_str(),
            &context_id,
            body.provider.as_deref(),
            body.preferred_model.as_deref(),
        )
        .await
        .map_err(map_put_error)?;

    state.analytics.track_prompt_stored(
        &surface,
        auth.user_id,
        auth.workspace_id,
        &provider_label,
    );

    let response = PutStorageResponse {
        version_id: result.version_id,
        created_at: result.created_at,
        kms_key_arn: result.kms_key_arn.clone(),
        fingerprint,
    };
    let location = format!("/v1/functions/{}/versions/{}", body.name.trim(), result.version_id);
    let location_header = axum::http::header::HeaderValue::try_from(location)
        .unwrap_or_else(|_| axum::http::header::HeaderValue::from_static("/"));
    drop(body);
    Ok((
        axum::http::StatusCode::CREATED,
        [(axum::http::header::LOCATION, location_header)],
        Json(response),
    ))
}
