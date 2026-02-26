//! API token validation: Bearer pk_... → api_tokens lookup. Returns (user_id, workspace_id).
//! Also supports session tokens (login) with workspace fallback to user's first workspace.

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::session::{hash_token, validate_session_token};

/// Auth context for execute and other protected endpoints. Resolves to (user_id, workspace_id).
#[derive(Clone, Debug)]
pub struct Auth {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
}

/// Validate pk_ token against api_tokens. Returns (user_id, workspace_id) if valid.
async fn validate_api_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
    if !token.starts_with("pk_") {
        return Ok(None);
    }
    let token_hash = hash_token(token);
    let row = sqlx::query(
        "SELECT user_id, workspace_id FROM api_tokens WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|r| {
        let uid: Uuid = r.try_get("user_id").ok()?;
        let wid: Uuid = r.try_get("workspace_id").ok()?;
        Some((uid, wid))
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

        // Try API token first (pk_...)
        if let Some((user_id, workspace_id)) = validate_api_token(&pool, token).await.map_err(|e| {
            tracing::error!(error = ?e, "api token validation failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "authentication failed" })),
            )
        })? {
            return Ok(Auth {
                user_id,
                workspace_id,
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
            });
        }

        Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "invalid or expired token" })),
        ))
    }
}
