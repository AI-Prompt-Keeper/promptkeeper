//! LLM provider HTTP client: forwards requests to OpenAI, Anthropic, or local (e.g. Llama).
//! Streaming SSE: pipe provider stream to client with minimal buffering.
//! Logs provider status and latency; does not log prompt content.

use axum::response::sse::Event;
use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::{Duration, Instant};
use tracing::instrument;

/// Result of a single provider call. Caller uses status to decide failover.
#[derive(Debug)]
pub struct ProviderResponse {
    pub status: u16,
    pub body: Bytes,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("timeout after {:?}", _0)]
    Timeout(Duration),
}

/// Returns true if the status indicates we should try a backup (waterfall).
pub fn is_failover_status(status: u16) -> bool {
    status == 429 || (status >= 500 && status < 600)
}

/// Call one LLM provider. Uses a short timeout so multiple attempts fit in 30s total.
#[instrument(skip(client, body))]
pub async fn call_provider(
    client: &Client,
    provider_id: &str,
    config: &serde_json::Value,
    body: &[u8],
    request_timeout: Duration,
) -> Result<ProviderResponse, ProviderError> {
    let url = config
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| default_url(provider_id));
    let req = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body.to_vec());
    let resp = tokio::time::timeout(request_timeout, req.send()).await
        .map_err(|_| ProviderError::Timeout(request_timeout))??;
    let status = resp.status().as_u16();
    let body = resp.bytes().await?;
    Ok(ProviderResponse { status, body })
}

fn default_url(provider_id: &str) -> &'static str {
    match provider_id {
        "openai" => "https://api.openai.com/v1/chat/completions",
        "anthropic" => "https://api.anthropic.com/v1/messages",
        "llama_local" | "llama" => "http://localhost:11434/v1/chat/completions",
        _ => "https://api.openai.com/v1/chat/completions",
    }
}

// --- Streaming SSE path for POST /v1/execute ---

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Anthropic,
}

impl Provider {
    pub fn from_id(id: &str) -> Self {
        match id {
            "anthropic" => Provider::Anthropic,
            _ => Provider::OpenAI,
        }
    }
}

/// Stream completion from the configured provider; returns SSE events (minimal buffering).
pub async fn stream_completion(
    provider: Provider,
    api_key: &str,
    prompt: &str,
) -> anyhow::Result<impl futures_util::Stream<Item = Event> + Send + 'static> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    let (url, body, content_type) = match provider {
        Provider::OpenAI => (
            "https://api.openai.com/v1/chat/completions".to_string(),
            serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": prompt}],
                "stream": true
            })
            .to_string(),
            "application/json",
        ),
        Provider::Anthropic => (
            "https://api.anthropic.com/v1/messages".to_string(),
            serde_json::json!({
                "model": "claude-3-5-sonnet-20241022",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": prompt}],
                "stream": true
            })
            .to_string(),
            "application/json",
        ),
    };

    let req = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", content_type)
        .body(body);

    let start = Instant::now();
    let res = req.send().await?;
    let status = res.status();
    let latency_ms = start.elapsed().as_millis();

    tracing::info!(
        provider = ?provider,
        status = %status.as_u16(),
        latency_ms = %latency_ms,
        "provider response"
    );

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        anyhow::bail!("provider error {}: {}", status, err_body);
    }

    let stream = res.bytes_stream();
    let stream = line_buffer_sse(stream);
    Ok(stream)
}

/// Forward provider bytes as SSE events: line-buffer and emit one Event per "data: ..." line.
fn line_buffer_sse(
    stream: impl futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl futures_util::Stream<Item = Event> + Send + 'static {
    let buffer = String::new();
    let stream = Box::pin(stream);
    futures_util::stream::unfold((stream, buffer), |(mut stream, mut buffer)| async move {
        loop {
            if let Some(idx) = buffer.find('\n') {
                let line = buffer[..idx].trim().to_string();
                buffer = buffer[idx + 1..].to_string();
                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ") {
                    return Some((Event::default().data(data), (stream, buffer)));
                }
                continue;
            }
            match stream.next().await {
                Some(Ok(chunk)) => {
                    if let Ok(s) = std::str::from_utf8(&chunk) {
                        buffer.push_str(s);
                    }
                }
                Some(Err(_)) => return None,
                None => return None,
            }
        }
    })
}
