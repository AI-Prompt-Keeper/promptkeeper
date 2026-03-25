//! Prompt Keeper — high-performance LLM proxy.
//!
//! Core routing engine with SSE streaming, Handlebars templating, envelope encryption (Put), and observability.
//! Serves API under /v1 and static frontend from current dir (for local testing).

use promptkeeper::observability::init_observability;
use promptkeeper::routes::app_router;
use promptkeeper::secrets::SecretEnveloper;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let prometheus_handle = init_observability()?;
    let upkeep = prometheus_handle.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            upkeep.run_upkeep();
        }
    });

    // Database pool (PostgreSQL).
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for registration/auth");
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let secrets = match std::env::var("KMS_KEY_ID") {
        Ok(kms_key_id) => match SecretEnveloper::from_env(kms_key_id.clone()).await {
            Ok(enveloper) => {
                tracing::info!("KMS envelope encryption enabled (POST /v1/keys, POST /v1/prompts)");
                Some(Arc::new(enveloper))
            }
            Err(e) => {
                tracing::warn!(err = %e, "KMS_KEY_ID set but KMS client failed; POST /v1/keys and POST /v1/prompts disabled");
                None
            }
        },
        Err(_) => {
            tracing::debug!("KMS_KEY_ID not set; POST /v1/keys and POST /v1/prompts disabled");
            None
        }
    };

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| ".".to_string());
    let app = app_router(secrets, db, prometheus_handle)
        .await?
        .fallback_service(ServeDir::new(static_dir));

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
