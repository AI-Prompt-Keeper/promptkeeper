//! Function metadata store: primary/backup providers and response format.
//! DB-backed with in-memory cache; every write persists to PostgreSQL.

use std::collections::HashMap;
use std::sync::RwLock;

use sqlx::Row;

/// Per-function metadata for routing and validation.
#[derive(Debug, Clone)]
pub struct FunctionMeta {
    /// Primary provider id (e.g. "openai", "anthropic").
    pub primary_provider: String,
    /// Backup provider ids in order of preference (e.g. ["anthropic", "llama_local"]).
    pub backup_providers: Vec<String>,
    /// If Some("json"), response will be validated as JSON and may trigger one retry if invalid.
    pub response_format: Option<String>,
    /// Prompt template (e.g. Handlebars); rendered with variables.
    pub prompt_template: String,
    /// Provider-specific config (API base URL, model, etc.).
    pub provider_config: HashMap<String, serde_json::Value>,
}

impl Default for FunctionMeta {
    fn default() -> Self {
        Self {
            primary_provider: "openai".to_string(),
            backup_providers: vec!["anthropic".to_string()],
            response_format: None,
            prompt_template: String::new(),
            provider_config: HashMap::new(),
        }
    }
}

/// Trait for function lookup. Both in-memory and DB-backed stores implement this.
pub trait FunctionStoreTrait: Send + Sync {
    fn get(&self, function_id: &str) -> Option<FunctionMeta>;
    /// Lookup with workspace scope. Default uses empty context (global).
    fn get_with_context(&self, function_id: &str, _context_id: &str) -> Option<FunctionMeta> {
        self.get(function_id)
    }
}

impl FunctionStoreTrait for FunctionStore {
    fn get(&self, function_id: &str) -> Option<FunctionMeta> {
        self.functions.read().ok()?.get(function_id).cloned()
    }
}

impl FunctionStoreTrait for DbFunctionStore {
    fn get(&self, function_id: &str) -> Option<FunctionMeta> {
        self.get_with_context(function_id, "")
    }

    fn get_with_context(&self, function_id: &str, context_id: &str) -> Option<FunctionMeta> {
        DbFunctionStore::get_with_context(self, function_id, context_id)
    }
}

/// Cache key: (function_name, context_id). Empty context_id = global/default.
fn cache_key(function_id: &str, context_id: &str) -> (String, String) {
    (function_id.to_string(), context_id.to_string())
}

/// DB-backed function store with in-memory cache. Loads at startup; persists every change.
pub struct DbFunctionStore {
    pool: sqlx::PgPool,
    /// Optional enveloper for decrypting encrypted prompt_versions.
    enveloper: Option<std::sync::Arc<crate::secrets::SecretEnveloper>>,
    /// Cache: (function_name, context_id) -> FunctionMeta
    cache: RwLock<HashMap<(String, String), FunctionMeta>>,
}

impl DbFunctionStore {
    pub fn new(
        pool: sqlx::PgPool,
        enveloper: Option<std::sync::Arc<crate::secrets::SecretEnveloper>>,
    ) -> Self {
        Self {
            pool,
            enveloper,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Load all production deployments from DB into cache. Call at startup.
    pub async fn load_from_db(&self) -> Result<(), LoadError> {
        let rows = sqlx::query(
            r#"
            SELECT f.name AS function_name, f.primary_provider, f.backup_providers,
                   f.response_format, f.provider_config,
                   COALESCE(pv.template_text, '') AS template_text,
                   pv.encrypted_payload, pv.encrypted_dek, pv.nonce, pv.kms_key_id,
                   d.context_id
            FROM deployments d
            JOIN functions f ON f.id = d.function_id
            JOIN prompt_versions pv ON pv.id = d.version_id
            WHERE d.tag = 'production'
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LoadError::Db(e.to_string()))?;

        let mut cache = self
            .cache
            .write()
            .map_err(|e| LoadError::Lock(e.to_string()))?;

        for row in rows {
            let function_name: String = row.try_get("function_name").map_err(LoadError::Row)?;
            let context_id: String = row.try_get("context_id").unwrap_or_default();
            let primary_provider: String = row.try_get("primary_provider").unwrap_or_else(|_| "openai".into());
            let backup_providers: Vec<String> = row
                .try_get::<serde_json::Value, _>("backup_providers")
                .ok()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let response_format: Option<String> = row.try_get("response_format").ok();
            let provider_config: HashMap<String, serde_json::Value> = row
                .try_get::<serde_json::Value, _>("provider_config")
                .ok()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            let template_text: Option<String> = row.try_get("template_text").ok();
            let encrypted_payload: Option<Vec<u8>> = row.try_get("encrypted_payload").ok();
            let encrypted_dek: Option<Vec<u8>> = row.try_get("encrypted_dek").ok();
            let nonce: Option<Vec<u8>> = row.try_get("nonce").ok();
            let kms_key_id: Option<String> = row.try_get("kms_key_id").ok();

            let prompt_template = if let Some(t) = template_text.filter(|s: &String| !s.is_empty()) {
                t
            } else if encrypted_payload.is_some()
                && encrypted_dek.is_some()
                && nonce.is_some()
                && kms_key_id.is_some()
            {
                let enveloper = self.enveloper.as_ref().ok_or_else(|| {
                    LoadError::Decrypt("encrypted prompt but no KMS enveloper configured".into())
                })?;
                let blob = crate::secrets::StorageBlob {
                    encrypted_payload: encrypted_payload.unwrap(),
                    encrypted_dek: encrypted_dek.unwrap(),
                    nonce: nonce.unwrap(),
                    kms_key_id: kms_key_id.unwrap(),
                };
                enveloper
                    .unwrap_and_decrypt(&blob, &context_id)
                    .await
                    .map_err(|e| LoadError::Decrypt(e.to_string()))?
            } else {
                String::new()
            };

            let meta = FunctionMeta {
                primary_provider,
                backup_providers,
                response_format,
                prompt_template,
                provider_config,
            };
            cache.insert(cache_key(&function_name, &context_id), meta);
        }

        Ok(())
    }

    /// Get function meta by name. Uses context_id="" for global/default deployment.
    pub fn get(&self, function_id: &str) -> Option<FunctionMeta> {
        self.get_with_context(function_id, "")
    }

    /// Get function meta by name and context_id (e.g. workspace id).
    pub fn get_with_context(&self, function_id: &str, context_id: &str) -> Option<FunctionMeta> {
        let cache = self.cache.read().ok()?;
        // Try exact context first, then fallback to global ("")
        cache
            .get(&cache_key(function_id, context_id))
            .or_else(|| cache.get(&cache_key(function_id, "")))
            .cloned()
    }

    /// Insert into cache only (for in-memory seeding). Prefer persist_prompt for real data.
    pub fn insert(&self, function_id: String, meta: FunctionMeta) {
        let _ = self
            .cache
            .write()
            .map(|mut m| m.insert(cache_key(&function_id, ""), meta));
    }

    /// Seed a default function if none exist. Used for local dev / tests when DB is empty.
    pub fn seed_default_if_empty(&self) {
        if self.get("default").is_none() {
            self.insert(
                "default".to_string(),
                FunctionMeta {
                    primary_provider: "openai".to_string(),
                    backup_providers: vec!["anthropic".to_string(), "llama_local".to_string()],
                    response_format: Some("json".to_string()),
                    prompt_template: "Hello, {{name}}!".to_string(),
                    provider_config: Default::default(),
                },
            );
        }
    }

    /// Reload one deployment from DB into cache. Call after Put persists a new prompt.
    pub async fn refresh_deployment(&self, function_name: &str, context_id: &str) -> Result<(), RefreshError> {
        let row = sqlx::query(
            r#"
            SELECT f.name AS function_name, f.primary_provider, f.backup_providers,
                   f.response_format, f.provider_config,
                   pv.template_text, pv.encrypted_payload, pv.encrypted_dek, pv.nonce, pv.kms_key_id
            FROM deployments d
            JOIN functions f ON f.id = d.function_id
            JOIN prompt_versions pv ON pv.id = d.version_id
            WHERE d.tag = 'production' AND f.name = $1 AND d.context_id = $2
            "#,
        )
        .bind(function_name)
        .bind(context_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RefreshError::Db(e.to_string()))?;

        let Some(row) = row else {
            // No deployment for this key; remove from cache
            let mut cache = self
                .cache
                .write()
                .map_err(|e| RefreshError::Lock(e.to_string()))?;
            cache.remove(&cache_key(function_name, context_id));
            return Ok(());
        };

        let primary_provider: String = row.try_get("primary_provider").unwrap_or_else(|_| "openai".into());
        let backup_providers: Vec<String> = row
            .try_get::<serde_json::Value, _>("backup_providers")
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let response_format: Option<String> = row.try_get("response_format").ok();
        let provider_config: HashMap<String, serde_json::Value> = row
            .try_get::<serde_json::Value, _>("provider_config")
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let template_text: Option<String> = row.try_get("template_text").ok();
        let encrypted_payload: Option<Vec<u8>> = row.try_get("encrypted_payload").ok();
        let encrypted_dek: Option<Vec<u8>> = row.try_get("encrypted_dek").ok();
        let nonce: Option<Vec<u8>> = row.try_get("nonce").ok();
        let kms_key_id: Option<String> = row.try_get("kms_key_id").ok();

        let prompt_template = if let Some(t) = template_text.filter(|s: &String| !s.is_empty()) {
            t
        } else if let (Some(ep), Some(ed), Some(n), Some(kid)) =
            (encrypted_payload, encrypted_dek, nonce, kms_key_id)
        {
            let enveloper = self.enveloper.as_ref().ok_or(RefreshError::NoEnveloper)?;
            let blob = crate::secrets::StorageBlob {
                encrypted_payload: ep,
                encrypted_dek: ed,
                nonce: n,
                kms_key_id: kid,
            };
            enveloper
                .unwrap_and_decrypt(&blob, context_id)
                .await
                .map_err(|e| RefreshError::Decrypt(e.to_string()))?
        } else {
            String::new()
        };

        let meta = FunctionMeta {
            primary_provider,
            backup_providers,
            response_format,
            prompt_template,
            provider_config,
        };

        let mut cache = self
            .cache
            .write()
            .map_err(|e| RefreshError::Lock(e.to_string()))?;
        cache.insert(cache_key(function_name, context_id), meta);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("db: {0}")]
    Db(String),
    #[error("row: {0}")]
    Row(#[from] sqlx::Error),
    #[error("lock: {0}")]
    Lock(String),
    #[error("decrypt: {0}")]
    Decrypt(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("db: {0}")]
    Db(String),
    #[error("lock: {0}")]
    Lock(String),
    #[error("no enveloper for encrypted prompt")]
    NoEnveloper,
    #[error("decrypt: {0}")]
    Decrypt(String),
}

/// In-memory store (used when no DB or for tests). Implements same get/insert interface.
pub struct FunctionStore {
    functions: RwLock<HashMap<String, FunctionMeta>>,
}

impl FunctionStore {
    pub fn new() -> Self {
        Self {
            functions: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, function_id: &str) -> Option<FunctionMeta> {
        self.functions.read().ok()?.get(function_id).cloned()
    }

    pub fn insert(&self, function_id: String, meta: FunctionMeta) {
        let _ = self.functions.write().map(|mut m| m.insert(function_id, meta));
    }
}

impl Default for FunctionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Config returned by mock DB for the execute pipeline (prompt template + provider keys).
#[derive(Clone, Debug)]
pub struct FunctionConfig {
    pub prompt_template: String,
    pub provider_id: String,
    pub api_key: String,
}

/// Mock DB: returns default config for any function_id. Replace with PostgreSQL when ready.
#[derive(Clone, Default)]
pub struct MockDb;

impl MockDb {
    pub async fn get_function_config(&self, _function_id: &str) -> anyhow::Result<FunctionConfig> {
        tokio::task::yield_now().await;
        Ok(FunctionConfig {
            prompt_template: "Hello, {{name}}! You asked: {{query}}.".to_string(),
            provider_id: "openai".to_string(),
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "mock-key".to_string()),
        })
    }
}
