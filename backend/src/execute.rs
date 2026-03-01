//! Execute pipeline: LangChain-based LLM calls with streaming.
//! Loads provider API keys from api_keys; uses prompt's default provider when request omits provider.
//! Validates provider is supported and enabled; falls back to backup providers when primary is disabled.

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

    if provider != "openai" && provider != "anthropic" {
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

    let stream = match provider.as_str() {
        "openai" => {
            let config = langchain_rust::llm::OpenAIConfig::default().with_api_key(api_key);
            let mut openai = langchain_rust::llm::openai::OpenAI::default().with_config(config);
            if let Some(ref m) = model {
                openai = openai.with_model(m.clone());
            }
            openai.stream(&messages).await
        }
        "anthropic" => {
            let mut claude = langchain_rust::llm::Claude::default().with_api_key(api_key);
            if let Some(ref m) = model {
                claude = claude.with_model(m.clone());
            }
            claude.stream(&messages).await
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

    let stream = stream.map_err(|e| {
        let msg = e.to_string();
        let details = format!("{:?}", e);
        ExecuteError::ProviderError {
            provider: provider.clone(),
            message: msg,
            details: Some(details),
        }
    })?;

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
