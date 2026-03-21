//! Liveness and readiness logic (HTTP handlers live in `routes.rs` to avoid crate cycles).

use serde::Serialize;

#[derive(Serialize)]
pub struct HealthJson {
    pub status: &'static str,
}

/// Readiness: function catalog loaded from DB at startup.
#[derive(Clone)]
pub struct ReadinessState {
    pub config_loaded: bool,
}

/// Returns true when DB responds and config was loaded at router build.
pub async fn is_ready(db: &sqlx::PgPool, readiness: &ReadinessState) -> bool {
    if !readiness.config_loaded {
        return false;
    }
    sqlx::query("SELECT 1").fetch_one(db).await.is_ok()
}
