//! HTTP routes for the LLM proxy and secure Put (envelope encryption).

use axum::{
    body::Bytes,
    extract::{FromRef, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use futures_util::Stream;
use rand::RngCore;
use sqlx::Row;
use std::sync::Arc;
use std::time::Instant;
use tracing::Instrument;

use crate::auth::api_token::Auth;
use crate::auth::crypto::{hash_password, verify_password};
use crate::db::DbFunctionStore;
use crate::execute::{execute_request, ExecuteState};
use crate::put::{PutFunctionService, PutKeyRequestBody, PutPromptRequestBody, PutStorageResponse, secret_fingerprint};
use crate::routes::request::ExecuteRequest;
use crate::secrets::{EnvelopeError, SecretEnveloper};
use sha2::{Digest, Sha256};

pub mod request {
    use serde::Deserialize;
    use std::collections::HashMap;

    /// POST /v1/execute body: function_id + variables map.
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ExecuteRequest {
        pub function_id: String,
        #[serde(default)]
        pub variables: HashMap<String, serde_json::Value>,
        /// Optional: prefer this provider (e.g. "openai", "anthropic") for this request.
        pub provider: Option<String>,
    }

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
}

impl FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            execute: ExecuteState::default(),
            secrets: None,
            put_service: None,
            db: sqlx::PgPool::connect_lazy("postgres://localhost/promptkeeper").expect("lazy pg pool"),
        }
    }
}

/// GET /health — for Docker and load balancer healthchecks.
async fn health_handler() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// Build the app router with shared state. Pass `secrets` when KMS is configured.
pub async fn app_router(secrets: Option<Arc<SecretEnveloper>>, db: sqlx::PgPool) -> Result<Router<()>, crate::db::LoadError> {
    let function_store = Arc::new(DbFunctionStore::new(db.clone(), secrets.clone()));
    if let Err(e) = function_store.load_from_db().await {
        tracing::warn!(err = %e, "load_from_db failed (schema 001 may not be applied); using empty function store");
    }
    function_store.seed_default_if_empty();

    let put_service = secrets.as_ref().map(|e| {
        Arc::new(
            PutFunctionService::new(Arc::clone(e), db.clone())
                .with_function_store(Arc::clone(&function_store)),
        )
    });

    let execute = ExecuteState {
        client: ExecuteState::default().client,
        functions: function_store,
        circuit_breaker: ExecuteState::default().circuit_breaker,
    };

    Ok(Router::new()
        .route("/health", get(health_handler))
        .route("/v1/execute", post(execute_handler))
        .route("/v1/auth/register", post(register_handler))
        .route("/v1/auth/login", post(login_handler))
        .route("/v1/keys", post(put_key_handler))
        .route("/v1/prompts", post(put_prompt_handler))
        .with_state(AppState {
            execute,
            secrets,
            put_service,
            db,
        }))
}

/// Request body for user registration.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
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
}

/// POST /v1/auth/register: create user, default workspace, workspace_members, and API key.
async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, Json<serde_json::Value>)> {
    // Basic validation – keep raw password in memory as short as possible.
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "invalid email" }))));
    }
    if body.password.len() < 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "password must be at least 12 characters" })),
        ));
    }

    // Hash password with Argon2id (see auth::crypto). No logging of raw password.
    let password = body.password;
    let hashed = hash_password(&password).map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "hashing failed" })))
    })?;

    // Drop raw password ASAP.
    drop(password);

    let name = body.name.clone();

    // Run all inserts in a transaction.
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to begin transaction");
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
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({ "error": "email already registered" })),
                );
            }
        }
        tracing::error!(error = ?e, "failed to insert user");
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    // 4. Generate API key and store hash (plaintext returned once)
    let mut token_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut token_bytes);
    let api_key = format!("pk_{}", hex::encode(token_bytes));

    let token_hash = {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hex::encode(hasher.finalize())
    };

    sqlx::query(
        "INSERT INTO api_tokens (user_id, workspace_id, token_hash, label) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(&token_hash)
    .bind("Default")
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create API token");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit registration");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create user" })),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id,
            email,
            name,
            created_at,
            default_workspace_id: workspace_id,
            api_key,
        }),
    ))
}

/// Request body for login.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    pub email: String,
    pub password: String,
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
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') {
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "login failed" })),
        )
    })?;

    let row = match row {
        Some(r) => r,
        None => {
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "login failed" })),
            )
        })?;

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

/// POST /v1/execute: parse body with minimal allocation, run execute, stream SSE back.
/// Requires Authorization: Bearer <api_token> or X-API-Key: <api_token> (pk_... or session token).
async fn execute_handler(
    State(state): State<AppState>,
    auth: Auth,
    body: Bytes,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>> + Send + 'static> {
    let state = state.execute;
    let start = Instant::now();

    // Zero-copy parse: single pass over body bytes.
    let req: ExecuteRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(err = %e, "execute request body parse failed");
            let event = Event::default()
                .json_data(serde_json::json!({ "error": e.to_string() }))
                .unwrap();
            return Sse::new(futures_util::stream::iter(vec![Ok(event)]));
        }
    };

    let function_id = req.function_id.clone();
    let span = tracing::info_span!("execute", function_id = %function_id);

    let context_id = auth.workspace_id.to_string();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        execute_request(state, req, &context_id).instrument(span),
    )
    .await;
    let result = match result {
        Ok(inner) => inner,
        Err(_) => {
            tracing::warn!("execute exceeded 30s client timeout");
            let event = Event::default()
                .json_data(serde_json::json!({ "error": "execute exceeded 30s client timeout" }))
                .unwrap();
            return Sse::new(futures_util::stream::iter(vec![Ok(event)]));
        }
    };
    let latency_ms = start.elapsed().as_millis();
    tracing::info!(
        function_id = %function_id,
        latency_ms = %latency_ms,
        "execute stream ready"
    );

    let events: Vec<Result<Event, axum::Error>> = match result {
        Ok(evs) => evs.into_iter().map(Ok).collect(),
        Err(e) => {
            tracing::warn!(err = %e, "execute failed");
            let event = Event::default()
                .json_data(serde_json::json!({ "error": e.to_string() }))
                .unwrap();
            vec![Ok(event)]
        }
    };

    Sse::new(futures_util::stream::iter(events))
}

fn map_put_error(e: crate::put::PutServiceError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let (status, msg): (_, String) = match &e {
        crate::put::PutServiceError::Envelope(EnvelopeError::Kms(_) | EnvelopeError::KmsConfig(_)) => {
            (axum::http::StatusCode::BAD_GATEWAY, "KMS connection or config failed".into())
        }
        crate::put::PutServiceError::Envelope(EnvelopeError::KmsDecrypt(_)) => {
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
    let service = state.put_service.as_ref().ok_or_else(|| {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "secrets not configured (KMS_KEY_ID required)" })),
        )
    })?;

    let context_id = auth.workspace_id.to_string();
    let fingerprint = secret_fingerprint(body.raw_secret.as_str());
    let result = service
        .store_key(
            &body.provider,
            body.raw_secret.as_str(),
            &context_id,
            auth.user_id,
            auth.workspace_id,
        )
        .await
        .map_err(map_put_error)?;

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
    let service = state.put_service.as_ref().ok_or_else(|| {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "secrets not configured (KMS_KEY_ID required)" })),
        )
    })?;

    let context_id = auth.workspace_id.to_string();
    let fingerprint = secret_fingerprint(body.raw_secret.as_str());
    let result = service
        .store_prompt(
            &body.name,
            body.raw_secret.as_str(),
            &context_id,
            body.provider.as_deref(),
        )
        .await
        .map_err(map_put_error)?;

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
