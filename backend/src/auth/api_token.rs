//! API token validation: Bearer `pk_mgt_live_...` / `pk_exe_live_...` → api_tokens lookup.
//! Also supports session tokens (login) with workspace fallback to user's first workspace.
//!
//! Execution-only keys (`pk_exe_live_...`, scope exe) may call `POST /v1/execute` and
//! `GET /v1/list-prompts`; other mutating requests return 403.

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, Method, StatusCode},
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::keygen::{
    self, PREFIX_EXE_LIVE, PREFIX_MGT_LIVE,
};
use super::scope::ApiKeyScope;
use super::session::{hash_token, validate_session_token};

/// Auth context for execute and other protected endpoints.
#[derive(Clone, Debug)]
pub struct Auth {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    /// `None` when authenticated with a **session** token (full access).
    /// `Some(Mgt)` / `Some(Exe)` when authenticated with a Prompt Keeper API key.
    pub api_key_scope: Option<ApiKeyScope>,
}

/// True if the caller used an execution-only API key.
pub fn is_execution_only_scope(scope: Option<ApiKeyScope>) -> bool {
    matches!(scope, Some(ApiKeyScope::Exe))
}

/// Whether an execution-only key may perform this HTTP request.
fn execution_key_allows_request(method: &Method, path: &str) -> bool {
    if method == Method::POST && path == "/v1/execute" {
        return true;
    }
    if method == Method::GET && path == "/v1/list-prompts" {
        return true;
    }
    false
}

fn reject_execution_key(method: &Method, path: &str) -> Option<(StatusCode, axum::Json<serde_json::Value>)> {
    if !execution_key_allows_request(method, path) {
        Some((
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "execution-only API key cannot access this endpoint"
            })),
        ))
    } else {
        None
    }
}

/// Validate scoped client API token against api_tokens. Returns (user_id, workspace_id, scope) if valid.
async fn validate_api_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<(Uuid, Uuid, ApiKeyScope)>, sqlx::Error> {
    if !token.starts_with(PREFIX_MGT_LIVE) && !token.starts_with(PREFIX_EXE_LIVE) {
        return Ok(None);
    }
    if !keygen::validate_scoped_key_checksum(token) {
        return Ok(None);
    }

    let token_hash = hash_token(token);
    let row = sqlx::query(
        "SELECT user_id, workspace_id, scope FROM api_tokens WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| {
        let uid: Uuid = r.try_get("user_id").ok()?;
        let wid: Uuid = r.try_get("workspace_id").ok()?;
        let scope_s: String = r.try_get("scope").ok()?;
        let scope = ApiKeyScope::from_db(&scope_s)?;

        // Consistency: token shape must match stored scope
        if token.starts_with(PREFIX_EXE_LIVE) && scope != ApiKeyScope::Exe {
            return None;
        }
        if token.starts_with(PREFIX_MGT_LIVE) && scope != ApiKeyScope::Mgt {
            return None;
        }

        Some((uid, wid, scope))
    }))
}

/// Get user's first workspace (for session auth which has no workspace binding).
async fn default_workspace_for_user(pool: &PgPool, user_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, Uuid>(
        "SELECT workspace_id FROM workspace_members WHERE user_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
    PgPool: FromRef<S>,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let method = parts.method.clone();
        let path = parts.uri.path().to_string();

        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                parts
                    .headers
                    .get("X-API-Key")
                    .and_then(|v| v.to_str().ok())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            })
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "error": "missing Authorization or X-API-Key header" })),
                )
            })?;

        let pool = PgPool::from_ref(state);

        // Try client API token first (pk_mgt_live_ / pk_exe_live_)
        if let Some((user_id, workspace_id, scope)) = validate_api_token(&pool, token).await.map_err(|e| {
            tracing::error!(error = ?e, "api token validation failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "authentication failed" })),
            )
        })? {
            if let Some(rejection) = reject_execution_key(&method, &path) {
                if is_execution_only_scope(Some(scope)) {
                    return Err(rejection);
                }
            }
            return Ok(Auth {
                user_id,
                workspace_id,
                api_key_scope: Some(scope),
            });
        }

        // Fall back to session token (64 hex chars)
        if let Some(user_id) = validate_session_token(&pool, token).await.map_err(|e| {
            tracing::error!(error = ?e, "session validation failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "authentication failed" })),
            )
        })? {
            let workspace_id = default_workspace_for_user(&pool, user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "failed to get default workspace");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(serde_json::json!({ "error": "authentication failed" })),
                    )
                })?
                .ok_or_else(|| {
                    (
                        StatusCode::FORBIDDEN,
                        axum::Json(serde_json::json!({ "error": "no workspace assigned" })),
                    )
                })?;
            return Ok(Auth {
                user_id,
                workspace_id,
                api_key_scope: None,
            });
        }

        Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "invalid or expired token" })),
        ))
    }
}
