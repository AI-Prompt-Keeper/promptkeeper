//! Execute pipeline: LangChain-based LLM calls with streaming.
//! Loads provider API keys from api_keys; uses prompt's default provider when request omits provider.
//! Validates provider is supported and enabled; falls back to backup providers when primary is disabled.
//! Supports OpenAI, Anthropic, and Google Gemini (via REST API).

use crate::db::{
    format_unsupported_provider_message, get_provider_status, get_supported_providers_list,
    load_provider_api_key, FunctionMeta, FunctionStoreTrait, ProviderStatus,
};
use crate::routes::request::ExecuteRequest;
use crate::templates::render_prompt;
use axum::response::sse::Event;
use futures_util::stream::{Stream, StreamExt};
use langchain_rust::language_models::llm::LLM;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

/// Unified chunk type for all providers (OpenAI/Anthropic via LangChain, Gemini via REST).
struct StreamChunk {
    content: String,
}

/// Execute state: function store, DB pool, and KMS enveloper for API key decryption.
#[derive(Clone)]
pub struct ExecuteState {
    pub functions: Arc<dyn FunctionStoreTrait>,
    pub db: sqlx::PgPool,
    /// Required for loading provider keys from api_keys.
    pub enveloper: Option<Arc<crate::secrets::SecretEnveloper>>,
}

/// Build ExecuteState with an in-memory store (for tests).
pub fn execute_state_with_memory_store() -> ExecuteState {
    let functions = Arc::new(crate::db::FunctionStore::default());
    functions.insert(
        "default".to_string(),
        FunctionMeta {
            primary_provider: "openai".to_string(),
            backup_providers: vec!["anthropic".to_string()],
            response_format: Some("json".to_string()),
            prompt_template: "Hello, {{name}}!".to_string(),
            provider_config: std::collections::HashMap::new(),
            preferred_model: None,
        },
    );
    ExecuteState {
        functions,
        db: sqlx::PgPool::connect_lazy("postgres://localhost/promptkeeper")
            .expect("lazy pg pool"),
        enveloper: None,
    }
}

impl Default for ExecuteState {
    fn default() -> Self {
        execute_state_with_memory_store()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("function not found: {0}")]
    FunctionNotFound(String),
    #[error("provider key not found: {provider}. Store via POST /v1/keys.")]
    NoProviderKey { provider: String },
    #[error("KMS not configured; cannot decrypt provider keys.")]
    NoEnveloper,
    #[error("render: {0}")]
    Render(String),
    #[error("provider {provider} error: {message}")]
    ProviderError {
        provider: String,
        message: String,
        details: Option<String>,
    },
    #[error("provider '{0}' is not supported")]
    UnsupportedProvider(String),
    /// Provider is in catalog but disabled. Caller may try fallback.
    #[error("provider '{0}' is disabled")]
    ProviderDisabled(String),
    #[error("{0}")]
    Other(String),
}

/// Resolve provider: request's provider if non-empty, else prompt's primary_provider.
fn resolve_provider<'a>(req: &'a ExecuteRequest, meta: &'a FunctionMeta) -> &'a str {
    if let Some(ref p) = req.provider {
        let t = p.trim();
        if !t.is_empty() {
            return t;
        }
    }
    meta.primary_provider.as_str()
}

/// Resolve model: request override > prompt version preferred_model > provider_config.
/// Returns None when nothing specified — provider uses its default.
fn resolve_model(
    req_model: Option<&str>,
    meta: &FunctionMeta,
    provider: &str,
) -> Option<String> {
    let req = req_model
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let preferred = meta
        .preferred_model
        .as_deref()
        .filter(|s| !s.is_empty());
    let from_config = meta
        .provider_config
        .get(provider)
        .and_then(|c| c.get("model").and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty());

    req.or(preferred)
        .or(from_config)
        .map(|s| s.to_string())
}

/// Build ordered list of providers to try: primary first, then backups. Deduped, lowercase.
fn providers_to_try(primary: &str, backups: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let p = primary.trim().to_lowercase();
    if !p.is_empty() && seen.insert(p.clone()) {
        out.push(p);
    }
    for b in backups {
        let b = b.trim().to_lowercase();
        if !b.is_empty() && seen.insert(b.clone()) {
            out.push(b);
        }
    }
    out
}

/// Execute with a specific provider. Validates provider is supported and enabled, loads key,
/// resolves model, and calls the LLM. Caller can invoke this for each candidate provider in turn.
/// model_override: from execute request; takes precedence over meta.preferred_model and provider_config.
#[tracing::instrument(skip(state, meta, rendered))]
pub async fn execute_with_provider(
    state: &ExecuteState,
    meta: &FunctionMeta,
    rendered: &str,
    context_id: &str,
    user_id: Uuid,
    workspace_id: Uuid,
    provider: &str,
    model_override: Option<&str>,
) -> Result<
    Pin<Box<dyn Stream<Item = Result<Event, axum::Error>> + Send>>,
    ExecuteError,
> {
    let provider = provider.trim().to_lowercase();

    let status = get_provider_status(&state.db, &provider)
        .await
        .map_err(|e| ExecuteError::Other(e))?;
    match status {
        ProviderStatus::NotSupported => {
            let list = get_supported_providers_list(&state.db)
                .await
                .unwrap_or_default();
            return Err(ExecuteError::UnsupportedProvider(
                format_unsupported_provider_message(&provider, &list),
            ));
        }
        ProviderStatus::Disabled => return Err(ExecuteError::ProviderDisabled(provider)),
        ProviderStatus::Available => {}
    }

    if provider != "openai" && provider != "anthropic" && provider != "gemini" {
        let list = get_supported_providers_list(&state.db)
            .await
            .unwrap_or_default();
        return Err(ExecuteError::UnsupportedProvider(
            format_unsupported_provider_message(&provider, &list),
        ));
    }

    let enveloper = state
        .enveloper
        .as_ref()
        .ok_or(ExecuteError::NoEnveloper)?;

    let api_key = load_provider_api_key(&state.db, enveloper, user_id, workspace_id, &provider)
        .await
        .map_err(|e| match &e {
            crate::db::ApiKeyLoadError::NotFound { provider: p } => ExecuteError::NoProviderKey {
                provider: p.clone(),
            },
            _ => ExecuteError::Other(e.to_string()),
        })?;

    let model = resolve_model(model_override, meta, &provider);

    let messages = vec![langchain_rust::schemas::messages::Message::new_human_message(
        rendered.to_string(),
    )];

    let stream: Pin<Box<dyn Stream<Item = Result<StreamChunk, ExecuteError>> + Send>> =
        match provider.as_str() {
            "openai" => {
                let config = langchain_rust::llm::OpenAIConfig::default().with_api_key(api_key);
                let mut openai = langchain_rust::llm::openai::OpenAI::default().with_config(config);
                if let Some(ref m) = model {
                    openai = openai.with_model(m.clone());
                }
                let s = openai.stream(&messages).await.map_err(|e| {
                    let msg = e.to_string();
                    ExecuteError::ProviderError {
                        provider: provider.clone(),
                        message: msg,
                        details: Some(format!("{:?}", e)),
                    }
                })?;
                let p = provider.clone();
                Box::pin(s.map(move |r| {
                    r.map(|d| StreamChunk {
                        content: d.content,
                    })
                    .map_err(|e| ExecuteError::ProviderError {
                        provider: p.clone(),
                        message: e.to_string(),
                        details: Some(format!("{:?}", e)),
                    })
                }))
            }
            "anthropic" => {
                let mut claude = langchain_rust::llm::Claude::default().with_api_key(api_key);
                if let Some(ref m) = model {
                    claude = claude.with_model(m.clone());
                }
                let s = claude.stream(&messages).await.map_err(|e| {
                    let msg = e.to_string();
                    ExecuteError::ProviderError {
                        provider: provider.clone(),
                        message: msg,
                        details: Some(format!("{:?}", e)),
                    }
                })?;
                let p = provider.clone();
                Box::pin(s.map(move |r| {
                    r.map(|d| StreamChunk {
                        content: d.content,
                    })
                    .map_err(|e| ExecuteError::ProviderError {
                        provider: p.clone(),
                        message: e.to_string(),
                        details: Some(format!("{:?}", e)),
                    })
                }))
            }
            "gemini" => {
                let gemini_stream = stream_gemini(rendered, &api_key, model.as_deref(), &provider)?;
                Box::pin(gemini_stream)
            }
            _ => {
                let list = get_supported_providers_list(&state.db)
                    .await
                    .unwrap_or_default();
                return Err(ExecuteError::UnsupportedProvider(
                    format_unsupported_provider_message(&provider, &list),
                ));
            }
        };

    let provider_owned = provider.clone();
    let s = stream.map(move |chunk_result| {
        match chunk_result {
            Ok(stream_data) => {
                let content = stream_data.content;
                if content.is_empty() {
                    Ok(Event::default().data(""))
                } else {
                    let ev = serde_json::json!({
                        "content": content,
                        "provider": provider_owned
                    });
                    Event::default()
                        .json_data(ev)
                        .map_err(|e| axum::Error::from(e))
                }
            }
            Err(e) => {
                let ev = serde_json::json!({
                    "error": e.to_string(),
                    "details": format!("{:?}", e),
                    "provider": provider_owned
                });
                Event::default()
                    .json_data(ev)
                    .map_err(|e| axum::Error::from(e))
            }
        }
    });

    Ok(Box::pin(s))
}

/// Stream completion from Google Gemini API (Generative Language API).
fn stream_gemini(
    prompt: &str,
    api_key: &str,
    model: Option<&str>,
    provider: &str,
) -> Result<
    Pin<Box<dyn Stream<Item = Result<StreamChunk, ExecuteError>> + Send>>,
    ExecuteError,
> {
    let model = model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("gemini-2.0-flash");
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
        model
    );
    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| ExecuteError::Other(e.to_string()))?;

    let req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .body(body.to_string());

    let provider_owned = provider.to_string();
    let stream = async_stream::stream! {
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                yield Err(ExecuteError::ProviderError {
                    provider: provider_owned.clone(),
                    message: e.to_string(),
                    details: None,
                });
                return;
            }
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            yield Err(ExecuteError::ProviderError {
                provider: provider_owned.clone(),
                message: format!("{}: {}", status, body),
                details: None,
            });
            return;
        }
        let mut bytes_stream = resp.bytes_stream();
        let mut buf = Vec::new();
        use futures_util::StreamExt as _;
        while let Some(chunk) = bytes_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(ExecuteError::ProviderError {
                        provider: provider_owned.clone(),
                        message: e.to_string(),
                        details: None,
                    });
                    return;
                }
            };
            buf.extend_from_slice(&chunk);
            while let Some(i) = buf.iter().position(|&b| b == b'\n') {
                let line = buf.drain(..=i).collect::<Vec<_>>();
                let line = String::from_utf8_lossy(&line);
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" || data.is_empty() {
                        continue;
                    }
                    let parsed: serde_json::Value = match serde_json::from_str(data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let text = parsed
                        .get("candidates")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("content"))
                        .and_then(|c| c.get("parts"))
                        .and_then(|p| p.get(0))
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    if !text.is_empty() {
                        yield Ok(StreamChunk {
                            content: text.to_string(),
                        });
                    }
                }
            }
        }
    };

    Ok(Box::pin(stream))
}

/// Execute: resolve function, render prompt, try primary then backup providers until one succeeds.
#[tracing::instrument(skip(state, req))]
pub async fn execute_request(
    state: ExecuteState,
    req: ExecuteRequest,
    context_id: &str,
    user_id: Uuid,
    workspace_id: Uuid,
) -> Result<
    Pin<Box<dyn Stream<Item = Result<Event, axum::Error>> + Send>>,
    ExecuteError,
> {
    let meta = state
        .functions
        .get_with_context(&req.function_id, context_id)
        .ok_or_else(|| ExecuteError::FunctionNotFound(req.function_id.clone()))?;

    let rendered = render_prompt(&meta.prompt_template, &req.variables)
        .map_err(|e| ExecuteError::Render(e.to_string()))?;

    let primary = resolve_provider(&req, &meta);
    let providers = providers_to_try(primary, &meta.backup_providers);

    let model_override = req.model.as_deref();

    let mut last_disabled_err = None;
    for provider in providers {
        match execute_with_provider(
            &state,
            &meta,
            &rendered,
            context_id,
            user_id,
            workspace_id,
            &provider,
            model_override,
        )
        .await
        {
            Ok(stream) => return Ok(stream),
            Err(ExecuteError::ProviderDisabled(ref p)) => {
                last_disabled_err = Some(ExecuteError::ProviderDisabled(p.clone()));
            }
            Err(e) => return Err(e),
        }
    }

    Err(match last_disabled_err {
        Some(e) => e,
        None => {
            let list = get_supported_providers_list(&state.db)
                .await
                .unwrap_or_default();
            ExecuteError::UnsupportedProvider(format_unsupported_provider_message(
                primary, &list,
            ))
        }
    })
}

/// Helper to convert ExecuteError into a single SSE event for error response.
/// Returns event with "error" and "details" so our response is an error.
pub fn execute_error_to_event(err: &ExecuteError) -> Event {
    let (error, details) = match err {
        ExecuteError::ProviderError {
            provider: _,
            message,
            details: d,
        } => (message.clone(), d.clone()),
        _ => (err.to_string(), None),
    };
    let payload = serde_json::json!({
        "error": error,
        "details": details
    });
    Event::default()
        .json_data(payload)
        .unwrap_or_else(|_| Event::default().data("{\"error\":\"internal\"}"))
}
