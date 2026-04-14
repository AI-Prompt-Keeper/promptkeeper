//! Async PostHog analytics reporting via a dedicated background thread.
//! Request handlers only enqueue events; network I/O happens off the request path.
//!
//! Uses the official [`posthog-rs`](https://posthog.com/docs/libraries/rust) client (blocking)
//! on the worker thread.
//!
//! Debug logs never emit `distinct_id`, `workspace_id`, or other identifiers—only a whitelist of
//! operational fields (endpoint, surface, latency, stable codes, etc.). Full payloads are still
//! sent to PostHog only.

use serde_json::Value;
use std::sync::mpsc::{self, Sender};

use posthog_rs::{client, Client, Error as PosthogError, Event};

/// Property keys allowed in application logs (no user/workspace identifiers).
const LOG_SAFE_PROPERTY_KEYS: &[&str] = &[
    "endpoint",
    "surface",
    "provider",
    "error_code",
    "reason",
    "latency_ms",
    "added_latency_ms",
    "count",
];

/// Subset of event properties safe for logs (GDPR-friendly: no distinct_id / workspace_id / PII).
fn sanitize_properties_for_log(properties: &Value) -> Value {
    let Some(obj) = properties.as_object() else {
        return Value::Object(serde_json::Map::new());
    };
    let mut m = serde_json::Map::new();
    for &key in LOG_SAFE_PROPERTY_KEYS {
        if let Some(v) = obj.get(key) {
            m.insert(key.to_string(), v.clone());
        }
    }
    Value::Object(m)
}

fn safe_fields_json(properties: &Value) -> String {
    serde_json::to_string(&sanitize_properties_for_log(properties))
        .unwrap_or_else(|e| format!("{{\"<serialize>\":\"{e}\"}}"))
}

#[derive(Debug)]
struct AnalyticsEvent {
    name: String,
    properties: Value,
}

#[derive(Clone)]
pub struct AnalyticsReporter {
    tx: Option<Sender<AnalyticsEvent>>,
    /// When `tx` is `None`, why PostHog was not started (for logging on each dropped event).
    disabled_reason: Option<&'static str>,
}

impl AnalyticsReporter {
    /// Reads `POSTHOG_TOKEN` (project API key). If unset or empty, analytics is a no-op.
    /// Optional `POSTHOG_HOST` overrides the ingestion URL (default US: `https://us.i.posthog.com`).
    /// Use `https://eu.i.posthog.com` for EU projects.
    pub fn from_env() -> Self {
        tracing::debug!(target: "promptkeeper::analytics", "analytics setup: begin");

        let (token, disabled_reason): (String, Option<&'static str>) =
            match std::env::var("POSTHOG_TOKEN") {
                Err(_) => {
                    tracing::debug!(
                        target: "promptkeeper::analytics",
                        "analytics setup: POSTHOG_TOKEN env var is not set"
                    );
                    (String::new(), Some("POSTHOG_TOKEN is not set"))
                }
                Ok(v) => {
                    let t = v.trim().to_string();
                    if t.is_empty() {
                        tracing::debug!(
                            target: "promptkeeper::analytics",
                            "analytics setup: POSTHOG_TOKEN is empty after trim"
                        );
                        (t, Some("POSTHOG_TOKEN is empty or whitespace only"))
                    } else {
                        tracing::debug!(
                            target: "promptkeeper::analytics",
                            token_len = t.len(),
                            "analytics setup: POSTHOG_TOKEN present (length only; value not logged)"
                        );
                        (t, None)
                    }
                }
            };

        if let Some(reason) = disabled_reason {
            tracing::debug!(
                target: "promptkeeper::analytics",
                reason = reason,
                "analytics setup: PostHog disabled — no client or worker"
            );
            return Self {
                tx: None,
                disabled_reason: Some(reason),
            };
        }

        // `posthog-rs` without `async-client` uses `reqwest::blocking::Client`. That must not be
        // constructed or dropped on Tokio's async runtime threads — it can panic with:
        // "Cannot drop a runtime in a context where blocking is not allowed".
        // `from_env()` runs from `app_router().await` on the main runtime, so build the client only
        // inside this dedicated OS thread.
        tracing::debug!(
            target: "promptkeeper::analytics",
            "analytics setup: spawning PostHog worker (client will be built on worker thread)"
        );

        let (tx, rx) = mpsc::channel::<AnalyticsEvent>();

        std::thread::Builder::new()
            .name("posthog-analytics-worker".to_string())
            .spawn(move || {
                tracing::debug!(
                    target: "promptkeeper::analytics",
                    "posthog worker: thread started"
                );
                tracing::debug!(
                    target: "promptkeeper::analytics",
                    "posthog worker: building blocking PostHog client (off async runtime)"
                );
                let ph_client = build_posthog_client(&token);
                for evt in rx {
                    let safe_fields = safe_fields_json(&evt.properties);
                    tracing::debug!(
                        target: "promptkeeper::analytics",
                        event = %evt.name,
                        safe_fields = %safe_fields,
                        "posthog worker: dequeued event, capturing"
                    );
                    capture_with_client(&ph_client, &evt.name, &evt.properties);
                }
                tracing::debug!(
                    target: "promptkeeper::analytics",
                    "posthog worker: channel closed, thread exiting"
                );
            })
            .expect("failed to spawn analytics worker thread");

        tracing::debug!(
            target: "promptkeeper::analytics",
            "analytics setup: complete (worker thread running)"
        );

        Self {
            tx: Some(tx),
            disabled_reason: None,
        }
    }

    fn enqueue(&self, name: &str, properties: Value) {
        let safe_fields = safe_fields_json(&properties);
        if let Some(tx) = &self.tx {
            tracing::debug!(
                target: "promptkeeper::analytics",
                event = %name,
                safe_fields = %safe_fields,
                "analytics enqueue: sending event to worker channel"
            );
            if let Err(e) = tx.send(AnalyticsEvent {
                name: name.to_string(),
                properties,
            }) {
                tracing::warn!(
                    reason = "posthog worker queue closed",
                    event = %name,
                    error = %e,
                    "posthog is not available"
                );
            }
        } else if let Some(reason) = self.disabled_reason {
            tracing::debug!(
                target: "promptkeeper::analytics",
                reason = reason,
                event = %name,
                safe_fields = %safe_fields,
                "analytics enqueue: skipped (PostHog disabled)"
            );
            tracing::warn!(
                reason = reason,
                event = %name,
                "posthog is not available"
            );
        }
    }

    pub fn track_proxy_success(
        &self,
        endpoint: &str,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        latency_ms: u128,
    ) {
        self.enqueue(
            "proxy_success",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "endpoint": endpoint,
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "latency_ms": latency_ms,
            }),
        );
    }

    /// Third-party-safe: `error_code` and optional `provider` only — never raw upstream messages.
    pub fn track_proxy_error(
        &self,
        endpoint: &str,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        error_code: &str,
        provider: Option<&str>,
        latency_ms: u128,
    ) {
        let mut props = serde_json::json!({
            "distinct_id": user_id.to_string(),
            "endpoint": endpoint,
            "surface": surface,
            "workspace_id": workspace_id.to_string(),
            "error_code": error_code,
            "latency_ms": latency_ms,
        });
        if let Some(p) = provider.filter(|s| !s.is_empty()) {
            props["provider"] = serde_json::json!(p);
        }
        self.enqueue("proxy_error", props);
    }

    pub fn track_added_latency(
        &self,
        endpoint: &str,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        added_latency_ms: u128,
    ) {
        self.enqueue(
            "proxy_added_latency",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "endpoint": endpoint,
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "added_latency_ms": added_latency_ms,
            }),
        );
    }

    /// User stored a provider API key (POST /v1/keys success).
    pub fn track_key_stored(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        provider: &str,
    ) {
        self.enqueue(
            "key_stored",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "provider": provider,
            }),
        );
    }

    /// User stored a prompt template (POST /v1/prompts success). `provider` is `"unknown"` when omitted.
    pub fn track_prompt_stored(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        provider: &str,
    ) {
        self.enqueue(
            "prompt_stored",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "provider": provider,
            }),
        );
    }

    /// GET /v1/list-prompts succeeded (titles only).
    pub fn track_prompts_listed(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        count: usize,
        latency_ms: u128,
    ) {
        self.enqueue(
            "prompts_listed",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/list-prompts",
                "count": count,
                "latency_ms": latency_ms,
            }),
        );
    }

    /// GET /v1/list-prompts failed after auth (e.g. DB error).
    pub fn track_prompts_list_failed(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        reason: &str,
        latency_ms: u128,
    ) {
        self.enqueue(
            "prompts_list_failed",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/list-prompts",
                "reason": reason,
                "latency_ms": latency_ms,
            }),
        );
    }

    /// User attempted to store a key for a provider that is not in the supported catalog.
    pub fn track_key_store_provider_not_supported(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        provider: &str,
    ) {
        self.enqueue(
            "key_store_provider_not_supported",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "provider": provider,
            }),
        );
    }

    // --- One event per HTTP API route (auth, parse failures, etc.) ---

    /// POST /v1/auth/register succeeded.
    pub fn track_user_registered(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) {
        self.enqueue(
            "user_registered",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/auth/register",
            }),
        );
    }

    /// POST /v1/auth/register rejected (`reason` is a stable code, no PII).
    pub fn track_register_failed(&self, surface: &str, reason: &str) {
        self.enqueue(
            "register_failed",
            serde_json::json!({
                "distinct_id": "anonymous",
                "surface": surface,
                "endpoint": "/v1/auth/register",
                "reason": reason,
            }),
        );
    }

    /// POST /v1/auth/login succeeded (session issued).
    pub fn track_user_logged_in(&self, surface: &str, user_id: uuid::Uuid) {
        self.enqueue(
            "user_logged_in",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "endpoint": "/v1/auth/login",
            }),
        );
    }

    /// POST /v1/auth/login failed (`reason` is generic to avoid user enumeration).
    pub fn track_login_failed(&self, surface: &str, reason: &str) {
        self.enqueue(
            "login_failed",
            serde_json::json!({
                "distinct_id": "anonymous",
                "surface": surface,
                "endpoint": "/v1/auth/login",
                "reason": reason,
            }),
        );
    }

    /// POST /v1/execute body was not valid JSON (after auth).
    pub fn track_execute_request_parse_error(
        &self,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) {
        self.enqueue(
            "execute_request_parse_error",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": "unknown",
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/execute",
            }),
        );
    }

    /// POST /v1/keys or /v1/prompts when KMS/Put is not configured (503).
    pub fn track_put_endpoint_unavailable(
        &self,
        endpoint: &str,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) {
        self.enqueue(
            "put_endpoint_unavailable",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": endpoint,
                "reason": "kms_not_configured",
            }),
        );
    }

    /// POST /v1/workspaces succeeded.
    pub fn track_workspace_created(
        &self,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) {
        self.enqueue(
            "workspace_created",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/workspaces",
            }),
        );
    }

    /// DELETE /v1/workspaces/:id succeeded.
    pub fn track_workspace_deleted(&self, user_id: uuid::Uuid, workspace_id: uuid::Uuid) {
        self.enqueue(
            "workspace_deleted",
            serde_json::json!({
                "distinct_id": user_id.to_string(),
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/workspaces",
            }),
        );
    }
}

fn build_posthog_client(token: &str) -> Client {
    tracing::debug!(
        target: "promptkeeper::analytics",
        "posthog client build: reading POSTHOG_HOST"
    );
    let host = std::env::var("POSTHOG_HOST")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match host.as_deref() {
        Some(h) => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                posthog_host = %h,
                "posthog client build: using explicit POSTHOG_HOST"
            );
            let c = client((token, h));
            tracing::debug!(
                target: "promptkeeper::analytics",
                "posthog client build: client constructed (explicit host)"
            );
            c
        }
        None => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                default_ingestion = "https://us.i.posthog.com",
                "posthog client build: POSTHOG_HOST unset — SDK default US ingestion"
            );
            let c = client(token);
            tracing::debug!(
                target: "promptkeeper::analytics",
                "posthog client build: client constructed (default host)"
            );
            c
        }
    }
}

fn capture_with_client(ph_client: &Client, name: &str, properties: &Value) {
    let Some(obj) = properties.as_object() else {
        tracing::warn!(event = %name, "analytics properties must be a JSON object");
        return;
    };
    let safe_fields = safe_fields_json(properties);
    tracing::debug!(
        target: "promptkeeper::analytics",
        event = %name,
        safe_fields = %safe_fields,
        "analytics capture: sending to PostHog"
    );
    let distinct_id = obj
        .get("distinct_id")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous")
        .to_string();

    let mut event = Event::new(name.to_string(), distinct_id);
    for (k, v) in obj {
        if k == "distinct_id" {
            continue;
        }
        if let Err(e) = event.insert_prop(k, v.clone()) {
            tracing::warn!(error = %e, key = %k, event = %name, "analytics insert_prop failed");
        }
    }

    let capture_result = ph_client.capture(event);
    match &capture_result {
        Ok(()) => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                event = %name,
                "analytics capture: PostHog accepted event (SDK Ok — HTTP 2xx; response body not exposed by posthog-rs)"
            );
        }
        Err(e) => {
            log_posthog_error_debug(e);
            tracing::warn!(error = %e, event = %name, "posthog capture failed");
        }
    }
}

fn log_posthog_error_debug(err: &PosthogError) {
    match err {
        PosthogError::BadRequest(body) => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                ?err,
                response_body = %body,
                "posthog HTTP response: 400/413 bad request"
            );
        }
        PosthogError::ServerError { status, message } => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                ?err,
                status = *status,
                response_body = %message,
                "posthog HTTP response: 5xx server error"
            );
        }
        PosthogError::RateLimit => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                ?err,
                "posthog HTTP response: 429 rate limited"
            );
        }
        PosthogError::Connection(msg) => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                ?err,
                detail = %msg,
                "posthog HTTP/network error"
            );
        }
        PosthogError::Serialization(msg) => {
            tracing::debug!(
                target: "promptkeeper::analytics",
                ?err,
                detail = %msg,
                "posthog serialization error"
            );
        }
        _ => {
            tracing::debug!(target: "promptkeeper::analytics", ?err, "posthog error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_properties_for_log_omits_identifiers() {
        let v = serde_json::json!({
            "distinct_id": "user-uuid",
            "workspace_id": "ws-uuid",
            "endpoint": "/v1/execute",
            "latency_ms": 42,
            "surface": "web",
        });
        let s = sanitize_properties_for_log(&v);
        let obj = s.as_object().expect("object");
        assert!(!obj.contains_key("distinct_id"));
        assert!(!obj.contains_key("workspace_id"));
        assert_eq!(obj.get("endpoint").and_then(|x| x.as_str()), Some("/v1/execute"));
        assert_eq!(obj.get("latency_ms").and_then(|x| x.as_u64()), Some(42));
        assert_eq!(obj.get("surface").and_then(|x| x.as_str()), Some("web"));
    }
}
