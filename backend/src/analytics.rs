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
}

