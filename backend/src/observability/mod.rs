//! Alpha observability: request IDs, JSON logs, Prometheus metrics, health probes.

pub mod health;
pub mod logging;
pub mod metrics;
pub mod middleware;
pub mod request_id;

pub use health::{HealthJson, ReadinessState};
pub use middleware::observability_middleware;
pub use metrics_exporter_prometheus::PrometheusHandle;
pub use request_id::{ObservabilityFields, ObservabilityShared, RequestId, X_REQUEST_ID};

use metrics_exporter_prometheus::PrometheusBuilder;

/// Install JSON logging + Prometheus recorder with histogram buckets for request duration (ms).
pub fn init_observability() -> anyhow::Result<PrometheusHandle> {
    logging::init_json_logging();

    // Spec: 0.05, 0.1, … interpreted as milliseconds (50ms, 100ms, …) for proxy latency.
    let buckets_ms = [
        50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0,
    ];
    let handle = PrometheusBuilder::new()
        .set_buckets(&buckets_ms)
        .map_err(|e| anyhow::anyhow!(e))?
        .install_recorder()
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(handle)
}
