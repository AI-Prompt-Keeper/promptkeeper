//! Integration tests for the Prompt Keeper API.
//!
//! Run all tests: `cargo test`
//! Run with DB (register/login): `cargo test -- --include-ignored` (requires DATABASE_URL and schema)

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use promptkeeper::routes::app_router;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// Database URL for tests. Uses DATABASE_URL env or falls back to local default.
fn test_db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/promptkeeper".into())
}

/// Create a test app with no KMS (secrets disabled). Uses lazy pool that won't connect until queried.
async fn test_app_no_kms() -> axum::Router {
    let pool = sqlx::PgPool::connect_lazy(&test_db_url()).expect("lazy pg pool");
    app_router(None, pool).await.expect("app_router")
}

/// Create a test app. For tests that need KMS, pass Some(enveloper); otherwise None.
async fn test_app_with_pool(pool: sqlx::PgPool) -> axum::Router {
    app_router(None, pool).await.expect("app_router")
}

/// Create app with function "test_fn_disabled" (primary=test_provider_disabled, no backups).
/// Call before creating app so load_from_db picks it up. Uses context_id='' for global deployment.
async fn setup_function_disabled_provider_no_fallback(pool: &sqlx::PgPool) {
    ensure_test_provider_disabled(pool).await;

    let provider_id: i64 = sqlx::query_scalar(
        "SELECT id FROM supported_providers WHERE provider = 'test_provider_disabled'",
    )
    .fetch_one(pool)
    .await
    .expect("test_provider_disabled must exist");

    let function_id: i64 = sqlx::query_scalar(
        "INSERT INTO functions (name, primary_provider_id, response_format, provider_config) \
         VALUES ('test_fn_disabled', $1, NULL, '{}'::jsonb) \
         ON CONFLICT (name) DO UPDATE SET primary_provider_id = $1 RETURNING id",
    )
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("insert function");

    let version_id: i64 = sqlx::query_scalar(
        "INSERT INTO prompt_versions (function_id, template_text, context_id) \
         VALUES ($1, 'Hello {{name}}!', '') RETURNING id",
    )
    .bind(function_id)
    .fetch_one(pool)
    .await
    .expect("insert prompt_version");

    sqlx::query(
        "INSERT INTO deployments (function_id, version_id, tag, context_id) \
         VALUES ($1, $2, 'production', '') \
         ON CONFLICT (function_id, context_id, tag) DO UPDATE SET version_id = $2",
    )
    .bind(function_id)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert deployment");
}

/// Create app + registered user with api_key. For execute tests that need auth.
/// Pass true for setup_disabled_fn to create test_fn_disabled (primary=test_provider_disabled, no backups).
async fn test_app_with_api_key_inner(setup_disabled_fn: bool) -> (axum::Router, String) {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    if setup_disabled_fn {
        setup_function_disabled_provider_no_fallback(&pool).await;
    }
    let app = test_app_with_pool(pool).await;
    let email = format!(
        "execute-{}@example.com",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let reg_body = json!({
        "email": email,
        "password": "securePassword123",
        "name": "Execute Test"
    });
    let reg = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reg.status(), StatusCode::CREATED, "register must succeed for execute tests");
    let body_str = body_string(reg.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    let api_key = parsed
        .get("api_key")
        .and_then(|v| v.as_str())
        .expect("api_key in register response")
        .to_string();
    (app, api_key)
}

async fn test_app_with_api_key() -> (axum::Router, String) {
    test_app_with_api_key_inner(false).await
}

/// Helper to get response body as bytes.
async fn body_bytes(body: axum::body::Body) -> Vec<u8> {
    body.collect().await.unwrap().to_bytes().to_vec()
}

/// Helper to get response body as UTF-8 string.
async fn body_string(body: axum::body::Body) -> String {
    String::from_utf8_lossy(&body_bytes(body).await).into_owned()
}

/// Ensure test_provider_disabled exists in supported_providers (for provider validation tests).
async fn ensure_test_provider_disabled(pool: &sqlx::PgPool) {
    sqlx::query(
        "INSERT INTO supported_providers (provider, supported, enabled) VALUES ($1, true, false) \
         ON CONFLICT (provider) DO UPDATE SET supported = true, enabled = false",
    )
    .bind("test_provider_disabled")
    .execute(pool)
    .await
    .expect("ensure test_provider_disabled");
}

/// Parse first SSE data payload from stream body. Returns the JSON object if present.
fn parse_first_sse_data(body_str: &str) -> Option<serde_json::Value> {
    for line in body_str.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                return Some(v);
            }
        }
    }
    None
}

// =============================================================================
// 1. Health
// =============================================================================

#[tokio::test]
async fn health_returns_200_and_ok() {
    let app = test_app_no_kms().await;

    let response = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response.into_body()).await;
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = test_app_no_kms().await;

    let response = app
        .oneshot(Request::get("/v1/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn health_accepts_get_only() {
    let app = test_app_no_kms().await;

    let response = app
        .oneshot(Request::post("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// =============================================================================
// 2. Execute
// =============================================================================

#[tokio::test]
async fn execute_without_auth_returns_401() {
    let app = test_app_no_kms().await;
    let body = json!({
        "function_id": "default",
        "variables": {}
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "execute without auth must return 401"
    );
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert!(parsed.get("error").is_some(), "error field must be present");
}

#[tokio::test]
async fn execute_happy_path_returns_sse_stream() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "default",
        "variables": { "name": "Alice", "query": "What is 2+2?" },
        "provider": "anthropic"
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Execute returns 200 even on provider errors; errors are in SSE data
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").and_then(|v| v.to_str().ok());
    assert!(
        content_type.map(|c| c.contains("text/event-stream")).unwrap_or(false),
        "Expected text/event-stream, got {:?}",
        content_type
    );
}

#[tokio::test]
async fn execute_function_not_found_returns_error_in_sse() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "unknown_fn_xyz",
        "variables": {}
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE stream must contain parseable data event");
    let err = data.get("error").and_then(|v| v.as_str()).expect("SSE data must have 'error' field");
    assert_eq!(err, "function not found: unknown_fn_xyz", "Exact error message must include function id");
}

#[tokio::test]
async fn execute_invalid_json_returns_parse_error_in_sse() {
    let (app, api_key) = test_app_with_api_key().await;

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from("{ invalid json }"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE stream must contain parseable data event");
    let err = data.get("error").and_then(|v| v.as_str()).expect("SSE data must have 'error' field for parse failures");
    assert!(!err.is_empty(), "Error message must not be empty");
    assert!(data.get("error").is_some() && data.as_object().map_or(false, |o| o.len() == 1), "Parse error response must only contain 'error' field, got: {:?}", data);
}

#[tokio::test]
async fn execute_missing_function_id_returns_error_in_sse() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "variables": { "name": "Alice" }
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE stream must contain parseable data event");
    let err = data.get("error").and_then(|v| v.as_str()).expect("SSE data must have 'error' field");
    assert!(err.to_lowercase().contains("function_id") || err.to_lowercase().contains("missing"), "Error must mention missing function_id: {}", err);
}

#[tokio::test]
async fn execute_empty_function_id_returns_function_not_found() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "",
        "variables": {}
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE must contain data");
    let err = data.get("error").and_then(|v| v.as_str()).expect("Must have error");
    assert_eq!(err, "function not found: ", "Empty function_id must produce function not found");
}

#[tokio::test]
async fn execute_provider_disabled_no_fallback_returns_error() {
    let (app, api_key) = test_app_with_api_key_inner(true).await;
    let body = json!({
        "function_id": "test_fn_disabled",
        "variables": { "name": "Alice" }
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE must contain data event");
    let err = data.get("error").and_then(|v| v.as_str()).expect("Must have error field");
    assert!(
        err.contains("disabled") || err.to_lowercase().contains("not enabled"),
        "expected 'disabled' or 'not enabled' in error: {}",
        err
    );
}

#[tokio::test]
async fn execute_provider_unsupported_no_fallback_returns_error() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "default",
        "variables": { "name": "Alice" },
        "provider": "test_provider_unsupported"
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let data = parse_first_sse_data(&body_str).expect("SSE must contain data event");
    let err = data.get("error").and_then(|v| v.as_str()).expect("Must have error field");
    assert!(
        err.contains("not supported") || err.to_lowercase().contains("unsupported"),
        "expected 'not supported' in error: {}",
        err
    );
}

#[tokio::test]
async fn execute_empty_variables_defaults_to_empty_object() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "default"
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should not fail on parse; variables default to {}
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn execute_wrong_content_type_still_parses_json() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "function_id": "default",
        "variables": {}
    });

    let response = app
        .oneshot(
            Request::post("/v1/execute")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "text/plain")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Handler reads raw body; JSON may still parse
    assert_eq!(response.status(), StatusCode::OK);
}

// =============================================================================
// 3. Put keys (POST /v1/keys) and put prompts (POST /v1/prompts)
// =============================================================================

#[tokio::test]
async fn put_key_without_auth_returns_401() {
    let app = test_app_no_kms().await;
    let body = json!({ "raw_secret": "sk-xxx", "provider": "openai" });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn put_prompt_without_auth_returns_401() {
    let app = test_app_no_kms().await;
    let body = json!({ "name": "my_fn", "raw_secret": "Hello {{name}}!" });

    let response = app
        .oneshot(
            Request::post("/v1/prompts")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn put_key_without_kms_returns_503() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({ "raw_secret": "sk-xxx", "provider": "openai" });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "secrets not configured (KMS_KEY_ID required)");
}

#[tokio::test]
async fn put_prompt_without_kms_returns_503() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({ "name": "my_fn", "raw_secret": "Hello {{name}}!" });

    let response = app
        .oneshot(
            Request::post("/v1/prompts")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn put_prompt_without_name_returns_400_or_503() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({ "raw_secret": "Hello" });

    let response = app
        .oneshot(
            Request::post("/v1/prompts")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "expected 400 (missing name) or 503 (no KMS)"
    );
}

#[tokio::test]
async fn put_key_without_provider_returns_400_or_503() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({ "raw_secret": "sk-xxx" });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "expected 400 (missing provider) or 503 (no KMS)"
    );
}

#[tokio::test]
async fn put_key_deny_unknown_fields_returns_422() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({
        "raw_secret": "sk-xxx",
        "provider": "openai",
        "extra": "field"
    });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn put_key_disabled_provider_returns_error() {
    let (app, api_key) = test_app_with_api_key().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    ensure_test_provider_disabled(&pool).await;

    let body = json!({
        "raw_secret": "sk-xxx",
        "provider": "test_provider_disabled"
    });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!response.status().is_success(), "adding key for disabled provider must fail");
    if response.status() == StatusCode::BAD_REQUEST {
        let body_str = body_string(response.into_body()).await;
        let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        let err = parsed.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("not enabled") || err.to_lowercase().contains("disabled"), "expected 'not enabled' or 'disabled' in error: {}", err);
    }
}

#[tokio::test]
async fn put_key_unsupported_provider_returns_error() {
    let (app, api_key) = test_app_with_api_key().await;

    let body = json!({
        "raw_secret": "sk-xxx",
        "provider": "test_provider_unsupported"
    });

    let response = app
        .oneshot(
            Request::post("/v1/keys")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(!response.status().is_success(), "adding key for unsupported provider must fail");
    if response.status() == StatusCode::BAD_REQUEST {
        let body_str = body_string(response.into_body()).await;
        let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        let err = parsed.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(err.contains("not supported") || err.to_lowercase().contains("unsupported"), "expected 'not supported' in error: {}", err);
    }
}

#[tokio::test]
async fn put_prompt_missing_raw_secret_returns_422() {
    let (app, api_key) = test_app_with_api_key().await;
    let body = json!({ "name": "my_fn" });

    let response = app
        .oneshot(
            Request::post("/v1/prompts")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// =============================================================================
// 4. Register
// =============================================================================

#[tokio::test]
async fn register_happy_path_returns_201_and_user() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool).await;

    let email = format!("test-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let body = json!({
        "email": email,
        "password": "securePassword123",
        "name": "Test User"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(parsed.get("password").is_none(), "Response must never leak password");
    assert!(parsed.get("password_hash").is_none(), "Response must never leak password hash");
    let keys: Vec<_> = parsed.as_object().unwrap().keys().collect();
    assert!(!keys.iter().any(|k| k.contains("password")), "No password-related fields in response: {:?}", keys);

    assert!(parsed.get("id").is_some());
    assert_eq!(parsed.get("email").and_then(|v| v.as_str()).unwrap(), email);
    assert_eq!(parsed.get("name").and_then(|v| v.as_str()).unwrap(), "Test User");
    assert!(parsed.get("created_at").is_some());
    assert!(parsed.get("default_workspace_id").is_some());

    let api_key = parsed.get("api_key").and_then(|v| v.as_str()).expect("api_key must be present");
    assert!(api_key.starts_with("pk_"), "api_key must have pk_ prefix");
    let suffix = &api_key[3..];
    assert_eq!(suffix.len(), 64, "api_key must be pk_ + 64 hex chars");
    assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()), "api_key suffix must be hex: {}", suffix);
}

#[tokio::test]
async fn register_deny_unknown_fields_returns_422() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "user@example.com",
        "password": "securePassword123",
        "admin": true,
        "extra": "field"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "Unknown fields must be rejected to prevent privilege escalation");
}

#[tokio::test]
async fn register_missing_fields_returns_422() {
    let app = test_app_no_kms().await;
    let body = json!({});

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn register_empty_email_returns_400() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "",
        "password": "securePassword123",
        "name": "Alice"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&body_string(response.into_body()).await).unwrap().get("error").and_then(|v| v.as_str()).unwrap(), "invalid email");
}

#[tokio::test]
async fn register_invalid_email_returns_400() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "not-an-email",
        "password": "securePassword123",
        "name": "Alice"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "invalid email");
}

#[tokio::test]
async fn register_password_too_short_returns_400() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "user@example.com",
        "password": "short",
        "name": "Alice"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "password must be at least 12 characters");
}

#[tokio::test]
async fn register_password_11_chars_fails() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "user@example.com",
        "password": "12345678901",
        "name": "Alice"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_str = body_string(response.into_body()).await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body_str).unwrap().get("error").and_then(|v| v.as_str()).unwrap(),
        "password must be at least 12 characters"
    );
}

#[tokio::test]
async fn register_email_normalized_to_lowercase() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool).await;

    let email = format!("User{}@Example.COM", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let body = json!({
        "email": email,
        "password": "securePassword123",
        "name": "Alice"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED, "Registration must succeed to verify email normalization");
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    let returned_email = parsed.get("email").and_then(|v| v.as_str()).unwrap();
    assert_eq!(returned_email, email.to_lowercase(), "Email must be normalized to lowercase");
}

#[tokio::test]
async fn register_missing_name_still_succeeds() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool).await;

    let email = format!("no-name-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let body = json!({
        "email": email,
        "password": "securePassword123"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED, "Registration without name must succeed");
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert!(parsed.get("name").is_none() || parsed.get("name").as_ref().map(|v| v.is_null()).unwrap_or(false), "Name must be null or absent when not provided");
}

#[tokio::test]
async fn register_duplicate_email_returns_409() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool.clone()).await;

    let email = format!("dup-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let body = json!({
        "email": email,
        "password": "securePassword123",
        "name": "First"
    });

    let resp1 = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp1.status(), StatusCode::CREATED, "First registration must succeed to test duplicate; DB may be unavailable");

    let resp2 = app
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::CONFLICT);
    let body_str = body_string(resp2.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "email already registered");
}

// =============================================================================
// 5. Login
// =============================================================================

#[tokio::test]
async fn login_missing_fields_returns_422() {
    let app = test_app_no_kms().await;
    let body = json!({});

    let response = app
        .oneshot(
            Request::post("/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_invalid_email_format_returns_401() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "not-an-email",
        "password": "anypassword"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "invalid email or password");
}

#[tokio::test]
async fn login_nonexistent_user_returns_401() {
    let app = test_app_no_kms().await;
    let body = json!({
        "email": "nonexistent@example.com",
        "password": "securePassword123"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "invalid email or password", "Must use generic message to avoid user enumeration");
}

#[tokio::test]
async fn login_happy_path_returns_200_and_token() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool.clone()).await;

    let email = format!("login-test-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let reg_body = json!({
        "email": email,
        "password": "securePassword123",
        "name": "Login Test"
    });

    let reg = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(reg.status(), StatusCode::CREATED);

    let login_body = json!({
        "email": email,
        "password": "securePassword123"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert!(parsed.get("password").is_none(), "Login response must never leak password");
    let token = parsed.get("token").and_then(|v| v.as_str()).expect("token must be present");
    assert_eq!(token.len(), 64, "Session token must be 64 hex chars");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "Token must be hex");

    let expires_at = parsed.get("expires_at").and_then(|v| v.as_str()).expect("expires_at must be present");
    assert!(expires_at.contains('T') && expires_at.contains('Z'), "expires_at must be ISO 8601");

    let user = parsed.get("user").expect("user object must be present");
    assert_eq!(user.get("email").and_then(|v| v.as_str()).unwrap(), email);
    assert!(user.get("id").is_some());
    assert!(user.get("name").is_some());
    assert!(user.get("password").is_none(), "User object must not contain password");
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_db_url())
        .await
        .expect("connect to test db");
    let app = test_app_with_pool(pool.clone()).await;

    let email = format!("wrong-pw-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let reg_body = json!({
        "email": email,
        "password": "correctPassword123",
        "name": "User"
    });

    let reg = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(reg.status(), StatusCode::CREATED);

    let login_body = json!({
        "email": email,
        "password": "wrongPassword123"
    });

    let response = app
        .oneshot(
            Request::post("/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body_str = body_string(response.into_body()).await;
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.get("error").and_then(|v| v.as_str()).unwrap(), "invalid email or password");
}
