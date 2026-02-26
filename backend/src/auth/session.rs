//! Session validation: Bearer token → hash lookup. Tokens never stored; only SHA-256 hex.

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Validated session from Bearer token. Use as extractor on protected routes.
#[derive(Clone, Debug)]
pub struct ValidSession {
    pub user_id: Uuid,
}

/// Hash token for storage/lookup. Matches login and api_tokens pattern.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate Bearer token against sessions table. Returns user_id if valid.
pub async fn validate_session_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let token_hash = hash_token(token);
    let row = sqlx::query(
        "SELECT user_id FROM sessions WHERE token_hash = $1 AND expires_at > now()",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|r| r.try_get("user_id").ok()))
}

#[async_trait]
impl<S> FromRequestParts<S> for ValidSession
where
    S: Send + Sync,
    PgPool: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "error": "missing Authorization header" })),
                )
            })?;

        let token = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "error": "invalid Authorization header" })),
                )
            })?;

        let pool = PgPool::from_ref(state);
        let user_id = validate_session_token(&pool, token)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "session validation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": "authentication failed" })),
                )
            })?;

        user_id
            .map(|id| Ok(ValidSession { user_id: id }))
            .unwrap_or_else(|| {
                Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "error": "invalid or expired token" })),
                ))
            })
    }
}
