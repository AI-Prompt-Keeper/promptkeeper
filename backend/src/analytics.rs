//! Async PostHog analytics reporting via a dedicated background thread.
//! Request handlers only enqueue events; network I/O happens off the request path.

use chrono::Utc;
use serde_json::json;
use std::sync::mpsc::{self, Sender};

const POSTHOG_CAPTURE_URL: &str = "https://us.i.posthog.com/capture/";

#[derive(Debug)]
struct AnalyticsEvent {
    name: String,
    properties: serde_json::Value,
}

#[derive(Clone)]
pub struct AnalyticsReporter {
    tx: Option<Sender<AnalyticsEvent>>,
}

impl AnalyticsReporter {
    pub fn from_env() -> Self {
        let token = match std::env::var("POSTHOG_TOKEN") {
            Ok(v) => v.trim().to_string(),
            Err(_) => String::new(),
        };
        if token.is_empty() {
            return Self { tx: None };
        }

        let (tx, rx) = mpsc::channel::<AnalyticsEvent>();

        std::thread::Builder::new()
            .name("posthog-analytics-worker".to_string())
            .spawn(move || {
                let agent = ureq::Agent::new_with_defaults();
                for evt in rx {
                    let payload = json!({
                        "api_key": token,
                        "event": evt.name,
                        "properties": evt.properties,
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    let _ = agent
                        .post(POSTHOG_CAPTURE_URL)
                        .header("Content-Type", "application/json")
                        .send(payload.to_string());
                }
            })
            .expect("failed to spawn analytics worker thread");

        Self { tx: Some(tx) }
    }

    fn enqueue(&self, name: &str, properties: serde_json::Value) {
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
            json!({
                "distinct_id": user_id.to_string(),
                "endpoint": endpoint,
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "latency_ms": latency_ms,
            }),
        );
    }

    pub fn track_proxy_error(
        &self,
        endpoint: &str,
        surface: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
        error: &str,
        latency_ms: u128,
    ) {
        self.enqueue(
            "proxy_error",
            json!({
                "distinct_id": user_id.to_string(),
                "endpoint": endpoint,
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "error": error,
                "latency_ms": latency_ms,
            }),
        );
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
            json!({
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
            json!({
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
            json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "provider": provider,
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
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
            json!({
                "distinct_id": user_id.to_string(),
                "surface": "unknown",
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/execute",
            }),
        );
    }

    /// POST /v1/execute-raw body was not valid JSON (after auth).
    pub fn track_execute_raw_request_parse_error(
        &self,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) {
        self.enqueue(
            "execute_raw_request_parse_error",
            json!({
                "distinct_id": user_id.to_string(),
                "surface": "unknown",
                "workspace_id": workspace_id.to_string(),
                "endpoint": "/v1/execute-raw",
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
            json!({
                "distinct_id": user_id.to_string(),
                "surface": surface,
                "workspace_id": workspace_id.to_string(),
                "endpoint": endpoint,
                "reason": "kms_not_configured",
            }),
        );
    }
}

