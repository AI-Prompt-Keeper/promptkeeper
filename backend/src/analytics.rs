//! Async PostHog analytics reporting via a dedicated background thread.
//! Request handlers only enqueue events; network I/O happens off the request path.
//!
//! Uses the official [`posthog-rs`](https://posthog.com/docs/libraries/rust) client (blocking)
//! on the worker thread.

use serde_json::Value;
use std::sync::mpsc::{self, Sender};

use posthog_rs::{client, Client, Event};

#[derive(Debug)]
struct AnalyticsEvent {
    name: String,
    properties: Value,
}

#[derive(Clone)]
pub struct AnalyticsReporter {
    tx: Option<Sender<AnalyticsEvent>>,
}

impl AnalyticsReporter {
    /// Reads `POSTHOG_TOKEN` (project API key). If unset or empty, analytics is a no-op.
    /// Optional `POSTHOG_HOST` overrides the ingestion URL (default US: `https://us.i.posthog.com`).
    /// Use `https://eu.i.posthog.com` for EU projects.
    pub fn from_env() -> Self {
        let token = match std::env::var("POSTHOG_TOKEN") {
            Ok(v) => v.trim().to_string(),
            Err(_) => String::new(),
        };
        if token.is_empty() {
            return Self { tx: None };
        }

        let ph_client = build_posthog_client(&token);

        let (tx, rx) = mpsc::channel::<AnalyticsEvent>();

        std::thread::Builder::new()
            .name("posthog-analytics-worker".to_string())
            .spawn(move || {
                for evt in rx {
                    capture_with_client(&ph_client, &evt.name, &evt.properties);
                }
            })
            .expect("failed to spawn analytics worker thread");

        Self { tx: Some(tx) }
    }

    fn enqueue(&self, name: &str, properties: Value) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AnalyticsEvent {
                name: name.to_string(),
                properties,
            });
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
}

fn build_posthog_client(token: &str) -> Client {
    let host = std::env::var("POSTHOG_HOST")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match host.as_deref() {
        Some(h) => client((token, h)),
        None => client(token),
    }
}

fn capture_with_client(ph_client: &Client, name: &str, properties: &Value) {
    let Some(obj) = properties.as_object() else {
        tracing::warn!(event = %name, "analytics properties must be a JSON object");
        return;
    };
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

    if let Err(e) = ph_client.capture(event) {
        tracing::warn!(error = %e, event = %name, "posthog capture failed");
    }
}
