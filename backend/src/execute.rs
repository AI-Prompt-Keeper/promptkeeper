//! Execute pipeline: 30s timeout, waterfall failover, circuit breaker, structural JSON validation.

use crate::db::{FunctionMeta, FunctionStoreTrait};
use crate::providers::{call_provider, is_failover_status, ProviderError, ProviderResponse};
use crate::routes::request::ExecuteRequest;
use crate::templates::render_prompt;
use axum::response::sse::Event;
use bytes::Bytes;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{instrument, warn};

/// Total time budget for the whole request (primary + failover + retries). Must not exceed 30s.
const TOTAL_TIMEOUT_SECS: u64 = 30;
/// Per-provider call timeout so several attempts fit in 30s.
const PER_CALL_TIMEOUT: Duration = Duration::from_secs(8);
/// Circuit breaker: mark unhealthy after this many failures in the window.
const FAILURE_THRESHOLD: u32 = 3;
/// Circuit breaker: failure count window.
const FAILURE_WINDOW: Duration = Duration::from_secs(60);
/// Circuit breaker: stay unhealthy for this long.
const UNHEALTHY_DURATION: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
struct CircuitBreakerState {
    /// Timestamps of recent failures (within FAILURE_WINDOW).
    failures: Vec<Instant>,
    /// When the provider can be tried again.
    unhealthy_until: Option<Instant>,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            failures: Vec::new(),
            unhealthy_until: None,
        }
    }
}

impl CircuitBreakerState {
    fn record_failure(&mut self, at: Instant) {
        self.failures.push(at);
        let cutoff = at.checked_sub(FAILURE_WINDOW).unwrap_or(at);
        self.failures.retain(|&t| t >= cutoff);
        if self.failures.len() >= FAILURE_THRESHOLD as usize {
            self.unhealthy_until = Some(at + UNHEALTHY_DURATION);
        }
    }

    fn is_healthy(&self, at: Instant) -> bool {
        if let Some(until) = self.unhealthy_until {
            if at < until {
                return false;
            }
        }
        true
    }
}

/// Shared circuit breaker state per provider id.
pub struct CircuitBreakerRegistry {
    state: RwLock<std::collections::HashMap<String, CircuitBreakerState>>,
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(std::collections::HashMap::new()),
        }
    }

    async fn is_healthy(&self, provider_id: &str, at: Instant) -> bool {
        let guard = self.state.read().await;
        guard
            .get(provider_id)
            .map(|s| s.is_healthy(at))
            .unwrap_or(true)
    }

    async fn record_failure(&self, provider_id: &str, at: Instant) {
        let mut m = self.state.write().await;
        m.entry(provider_id.to_string())
            .or_default()
            .record_failure(at);
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state for the execute endpoint.
#[derive(Clone)]
pub struct ExecuteState {
    pub client: reqwest::Client,
    pub functions: Arc<dyn FunctionStoreTrait>,
    pub circuit_breaker: Arc<CircuitBreakerRegistry>,
}

/// Build ExecuteState with an in-memory store (for tests). Seeds a default function.
pub fn execute_state_with_memory_store() -> ExecuteState {
    let functions = Arc::new(crate::db::FunctionStore::default());
    functions.insert(
        "default".to_string(),
        FunctionMeta {
            primary_provider: "openai".to_string(),
            backup_providers: vec!["anthropic".to_string(), "llama_local".to_string()],
            response_format: Some("json".to_string()),
            prompt_template: "Hello, {{name}}!".to_string(),
            provider_config: Default::default(),
        },
    );
    ExecuteState {
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(TOTAL_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|e| panic!("reqwest client: {}", e)),
        functions,
        circuit_breaker: Arc::new(CircuitBreakerRegistry::default()),
    }
}

impl Default for ExecuteState {
    fn default() -> Self {
        execute_state_with_memory_store()
    }
}

/// Build ordered list of providers to try: preferred first (if specified and healthy), then primary, then backups.
async fn providers_to_try(
    meta: &FunctionMeta,
    preferred_provider: Option<&str>,
    now: Instant,
    cb: &CircuitBreakerRegistry,
) -> Vec<String> {
    let mut out = Vec::new();

    // Collect all configured providers (primary + backups).
    let mut configured = vec![meta.primary_provider.clone()];
    for b in &meta.backup_providers {
        if !configured.contains(b) {
            configured.push(b.clone());
        }
    }

    // If client specified a preferred provider and it's in the list and healthy, try it first.
    if let Some(pref) = preferred_provider {
        let pref_trimmed = pref.trim();
        if !pref_trimmed.is_empty() {
            if let Some(p) = configured.iter().find(|c| c.eq_ignore_ascii_case(pref_trimmed)) {
                if cb.is_healthy(p, now).await {
                    out.push(p.clone());
                }
            }
        }
    }

    // Add primary if not already in out.
    if cb.is_healthy(&meta.primary_provider, now).await && !out.contains(&meta.primary_provider) {
        out.push(meta.primary_provider.clone());
    }
    for b in &meta.backup_providers {
        if cb.is_healthy(b, now).await && !out.contains(b) {
            out.push(b.clone());
        }
    }
    out
}

/// Structural validator: if format is "json", check that body is valid JSON. Returns true if valid or not required.
fn validate_json_if_requested(body: &[u8], response_format: Option<&str>) -> bool {
    let Some("json") = response_format else { return true };
    serde_json::from_slice::<serde_json::Value>(body).is_ok()
}

/// Build LLM request body (OpenAI-style). In production you might vary by provider.
fn build_llm_body(meta: &FunctionMeta, provider_id: &str, rendered_prompt: &str) -> Vec<u8> {
    let model = meta
        .provider_config
        .get(provider_id)
        .and_then(|c| c.get("model").and_then(|v| v.as_str()))
        .unwrap_or(default_model(provider_id));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": rendered_prompt}]
    });
    serde_json::to_vec(&body).unwrap_or_default()
}

fn default_model(provider_id: &str) -> &'static str {
    match provider_id {
        "openai" => "gpt-4",
        "anthropic" => "claude-3-sonnet-20240229",
        "llama" | "llama_local" => "llama3",
        _ => "gpt-4",
    }
}

/// Single attempt: call one provider and return response or error. Records circuit breaker on failure.
async fn try_provider(
    state: &ExecuteState,
    provider_id: &str,
    meta: &FunctionMeta,
    body: &[u8],
    now: Instant,
) -> Result<ProviderResponse, ProviderError> {
    let config = meta
        .provider_config
        .get(provider_id)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match call_provider(
        &state.client,
        provider_id,
        &config,
        body,
        PER_CALL_TIMEOUT,
    )
    .await
    {
        Ok(r) => Ok(r),
        Err(e) => {
            state.circuit_breaker.record_failure(provider_id, now).await;
            Err(e)
        }
    }
}

/// Execute with 30s cap, waterfall, circuit breaker, and one JSON validation retry.
/// context_id: workspace scope for function lookup (from auth); empty = global fallback.
#[instrument(skip(state, req))]
pub async fn execute_request(
    state: ExecuteState,
    req: ExecuteRequest,
    context_id: &str,
) -> Result<Vec<Event>, anyhow::Error> {
    let deadline = Instant::now() + Duration::from_secs(TOTAL_TIMEOUT_SECS);
    let meta = state
        .functions
        .get_with_context(&req.function_id, context_id)
        .ok_or_else(|| anyhow::anyhow!("function not found: {}", req.function_id))?;

    let rendered_prompt = render_prompt(&meta.prompt_template, &req.variables)?;
    let want_json = meta.response_format.as_deref() == Some("json");

    let mut last_status = 0u16;
    let mut last_body = Bytes::new();
    let mut tried = Vec::<String>::new();
    let mut json_retry_used = false;

    let providers = providers_to_try(
        &meta,
        req.provider.as_deref(),
        Instant::now(),
        state.circuit_breaker.as_ref(),
    )
    .await;

    if providers.is_empty() {
        anyhow::bail!("no healthy providers available");
    }

    for provider_id in &providers {
        if Instant::now() >= deadline {
            break;
        }
        let body = build_llm_body(&meta, provider_id, &rendered_prompt);
        let now = Instant::now();

        let resp = match try_provider(&state, provider_id, &meta, &body, now).await {
            Ok(r) => r,
            Err(e) => {
                warn!(provider = %provider_id, err = %e, "provider call failed");
                state.circuit_breaker.record_failure(provider_id, now).await;
                continue;
            }
        };

        if is_failover_status(resp.status) {
            state.circuit_breaker.record_failure(provider_id, Instant::now()).await;
            last_status = resp.status;
            last_body = resp.body;
            tried.push(provider_id.clone());
            continue;
        }

        last_status = resp.status;
        last_body = resp.body.clone();

        if want_json && !validate_json_if_requested(&last_body, meta.response_format.as_deref()) {
            if !json_retry_used {
                json_retry_used = true;
                tried.push(provider_id.clone());
                continue;
            }
        }

        let event = Event::default()
            .json_data(serde_json::json!({
                "content": String::from_utf8_lossy(last_body.as_ref()),
                "status": last_status,
                "provider": provider_id
            }))
            .map_err(|e| anyhow::anyhow!("sse json: {}", e))?;
        return Ok(vec![event]);
    }

    if last_status != 0 {
        let event = Event::default()
            .json_data(serde_json::json!({
                "content": String::from_utf8_lossy(last_body.as_ref()),
                "error": "all providers failed or timed out",
                "status": last_status,
                "tried": tried
            }))
            .map_err(|e| anyhow::anyhow!("sse json: {}", e))?;
        return Ok(vec![event]);
    }

    anyhow::bail!("no healthy providers or all attempts failed")
}
