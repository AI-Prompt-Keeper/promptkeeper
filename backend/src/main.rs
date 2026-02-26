//! Prompt Keeper — high-performance LLM proxy.
//!
//! Core routing engine with SSE streaming, Handlebars templating, envelope encryption (Put), and observability.
//! Serves API under /v1 and static frontend from current dir (for local testing).

use promptkeeper::routes::app_router;
use promptkeeper::secrets::SecretEnveloper;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "promptkeeper=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

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
    let app = app_router(secrets, db)
        .await?
        .fallback_service(ServeDir::new(static_dir));

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
