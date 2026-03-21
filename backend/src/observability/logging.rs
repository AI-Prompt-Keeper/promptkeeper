//! JSON logging subscriber configuration.

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Install global JSON logging (timestamps, level, span fields). Ignores duplicate init.
pub fn init_json_logging() {
    let _ = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "promptkeeper=info,tower_http=info,axum=warn".into()
            }),
        )
        .with(
            fmt::layer()
                .json()
                .with_target(false)
                .with_current_span(true),
        )
        .try_init();
}
