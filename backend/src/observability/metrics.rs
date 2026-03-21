//! Prometheus metrics: counters, histogram, gauge, provider error classification.

use crate::execute::ExecuteError;
use super::request_id::ObservabilityShared;

/// Stable label for logs/metrics (not user-facing messages).
pub fn execute_error_type_label(err: &ExecuteError) -> &'static str {
    match err {
        ExecuteError::FunctionNotFound(_) => "function_not_found",
        ExecuteError::NoProviderKey { .. } => "no_provider_key",
        ExecuteError::NoEnveloper => "no_enveloper",
        ExecuteError::Render(_) => "render",
        ExecuteError::ProviderError { .. } => "provider_error",
        ExecuteError::UnsupportedProvider(_) => "unsupported_provider",
        ExecuteError::ProviderDisabled(_) => "provider_disabled",
        ExecuteError::Other(_) => "other",
    }
}

pub fn set_proxy_fields(
    obs: &ObservabilityShared,
    function_id: &str,
    provider: Option<String>,
    model: Option<String>,
) {
    if let Ok(mut g) = obs.lock() {
        g.function_id = Some(function_id.to_string());
        g.provider = provider;
        g.model = model;
    }
}

pub fn apply_execute_failure(obs: &ObservabilityShared, err: &ExecuteError) {
    if let Ok(mut g) = obs.lock() {
        g.handler_status = Some("error".into());
        g.error_type = Some(execute_error_type_label(err).to_string());
    }
    record_provider_error_metric(err);
}

/// Classify upstream/provider messages for `prke_provider_errors_total`.
pub fn classify_provider_error_type(message: &str) -> &'static str {
    let m = message.to_lowercase();
    if m.contains("rate") && m.contains("limit") {
        return "rate_limit";
    }
    if m.contains("timeout") || m.contains("timed out") || m.contains("deadline") {
        return "timeout";
    }
    if m.contains("401")
        || m.contains("403")
        || m.contains("unauthorized")
        || m.contains("invalid api key")
        || m.contains("authentication")
    {
        return "auth_error";
    }
    if m.contains(" 500")
        || m.contains(" 502")
        || m.contains(" 503")
        || m.contains(" 504")
        || m.contains("internal server error")
        || m.contains("bad gateway")
        || (m.contains("gateway") && m.contains("error"))
    {
        return "upstream_5xx";
    }
    "unknown"
}

/// Map [`ExecuteError`] to provider + error_type for metrics (non-provider errors return None).
pub fn provider_error_for_execute(err: &ExecuteError) -> Option<(&str, &'static str)> {
    match err {
        ExecuteError::ProviderError { provider, message, .. } => {
            Some((provider.as_str(), classify_provider_error_type(message)))
        }
        _ => None,
    }
}

/// Increment `prke_provider_errors_total` when applicable.
pub fn record_provider_error_metric(err: &ExecuteError) {
    if let Some((provider, et)) = provider_error_for_execute(err) {
        metrics::counter!(
            "prke_provider_errors_total",
            "provider" => sanitize_label(provider),
            "error_type" => et,
        )
        .increment(1);
    }
}

/// Low-cardinality label sanitizer (avoid unbounded provider strings in edge cases).
fn sanitize_label(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "unknown".to_string()
    } else if t.len() > 64 {
        t.chars().take(64).collect()
    } else {
        t.to_string()
    }
}

pub fn label_or_unknown(opt: Option<&str>) -> String {
    match opt {
        None => "unknown".to_string(),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() {
                "unknown".to_string()
            } else if t.len() > 64 {
                t.chars().take(64).collect()
            } else {
                t.to_string()
            }
        }
    }
}
