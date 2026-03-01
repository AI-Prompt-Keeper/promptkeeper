//! PutFunctionService: encrypt + store. Keys → api_keys; Prompts → prompt_versions + deployments.
//! Separate store_key and store_prompt methods.

use chrono::Utc;
use sqlx::Row;

use crate::db::{
    format_provider_error_with_list, get_provider_status, get_supported_providers_list,
    DbFunctionStore, ProviderStatus,
};
use crate::put::StorageResult;
use crate::secrets::{EnvelopeError, SecretEnveloper};

#[derive(Debug, thiserror::Error)]
pub enum PutServiceError {
    #[error("envelope encryption failed: {0}")]
    Envelope(#[from] EnvelopeError),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("database: {0}")]
    Db(String),
}

/// Processes PUT logic: encrypt with envelope, persist to DB, return metadata.
/// Caller (handler) is responsible for validation; this layer does encrypt + store.
pub struct PutFunctionService {
    enveloper: std::sync::Arc<SecretEnveloper>,
    db: sqlx::PgPool,
    /// Optional: refresh cache after persist (when execute uses DbFunctionStore).
    function_store: Option<std::sync::Arc<DbFunctionStore>>,
}

impl PutFunctionService {
    pub fn new(enveloper: std::sync::Arc<SecretEnveloper>, db: sqlx::PgPool) -> Self {
        Self {
            enveloper,
            db,
            function_store: None,
        }
    }

    /// Set the function store to refresh after persisting a prompt. Enables execute to see new prompts immediately.
    pub fn with_function_store(mut self, store: std::sync::Arc<DbFunctionStore>) -> Self {
        self.function_store = Some(store);
        self
    }

    /// Store provider API key. Encrypts with envelope; persists to api_keys.
    /// Validates provider is supported and enabled before storing.
    pub async fn store_key(
        &self,
        provider: &str,
        raw_secret: &str,
        context_id: &str,
        user_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> Result<StorageResult, PutServiceError> {
        let provider = provider.trim();
        if provider.is_empty() {
            return Err(PutServiceError::Validation("provider is required".into()));
        }
        let status = get_provider_status(&self.db, provider)
            .await
            .map_err(|e| PutServiceError::Db(e))?;
        match status {
            ProviderStatus::NotSupported => {
                let list = get_supported_providers_list(&self.db)
                    .await
                    .unwrap_or_default();
                return Err(PutServiceError::Validation(
                    format_provider_error_with_list(provider, &list, "not supported"),
                ));
            }
            ProviderStatus::Disabled => {
                let list = get_supported_providers_list(&self.db)
                    .await
                    .unwrap_or_default();
                return Err(PutServiceError::Validation(
                    format_provider_error_with_list(provider, &list, "not enabled"),
                ));
            }
            ProviderStatus::Available => {}
        }
        self.persist_api_key(&user_id, &workspace_id, provider, raw_secret, context_id)
            .await
    }

    /// Store prompt template for a named function. Persists to prompt_versions + deployment.
    /// Optional provider sets primary_provider when creating a new function.
    /// Optional preferred_model resolves to models.id and sets prompt_versions.preferred_model_id.
    pub async fn store_prompt(
        &self,
        name: &str,
        raw_secret: &str,
        context_id: &str,
        provider: Option<&str>,
        preferred_model: Option<&str>,
    ) -> Result<StorageResult, PutServiceError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(PutServiceError::Validation("name is required".into()));
        }
        self.persist_prompt(name, raw_secret, context_id, provider, preferred_model)
            .await
    }

    /// Persist prompt: ensure function exists, insert prompt_version (encrypted), upsert deployment.
    /// primary_provider defaults to 'openai' when creating a new function; optional provider overrides.
    /// preferred_model: resolved (provider + name) -> models.id, inserted if missing.
    async fn persist_prompt(
        &self,
        function_id: &str,
        raw_secret: &str,
        context_id: &str,
        primary_provider: Option<&str>,
        preferred_model: Option<&str>,
    ) -> Result<StorageResult, PutServiceError> {
        let blob = self
            .enveloper
            .encrypt_and_wrap(raw_secret.to_string(), context_id)
            .await?;

        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|e| PutServiceError::Db(e.to_string()))?;

        // 1. Ensure function exists (by name)
        let function_db_id: i64 = {
            let exists = sqlx::query_scalar::<_, i64>("SELECT id FROM functions WHERE name = $1")
                .bind(function_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| PutServiceError::Db(e.to_string()))?;

            match exists {
                Some(id) => id,
                None => {
                    let prov_name = primary_provider
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("openai");
                    let provider_id: i64 = sqlx::query_scalar(
                        "SELECT id FROM supported_providers WHERE provider = $1",
                    )
                    .bind(prov_name)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| PutServiceError::Db(e.to_string()))?
                    .ok_or_else(|| {
                        PutServiceError::Validation(format!(
                            "provider '{}' not in supported_providers; add it first",
                            prov_name
                        ))
                    })?;
                    let r = sqlx::query(
                        "INSERT INTO functions (name, primary_provider_id, response_format, provider_config) \
                         VALUES ($1, $2, NULL, '{}'::jsonb) RETURNING id",
                    )
                    .bind(function_id)
                    .bind(provider_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| PutServiceError::Db(e.to_string()))?;
                    r.try_get("id").map_err(|e| PutServiceError::Db(e.to_string()))?
                }
            }
        };

        // 1b. Resolve preferred_model -> models.id (provider = function's primary)
        let preferred_model_id: Option<i64> = {
            let prov: String = sqlx::query_scalar(
                "SELECT sp.provider FROM functions f JOIN supported_providers sp ON sp.id = f.primary_provider_id WHERE f.id = $1",
            )
            .bind(function_db_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| PutServiceError::Db(e.to_string()))?
            .unwrap_or_else(|| primary_provider.map(str::trim).filter(|s| !s.is_empty()).unwrap_or("openai").to_string());
            let model_name = preferred_model.map(str::trim).filter(|s| !s.is_empty());
            match model_name {
                Some(name) => {
                    let provider_id: i64 = sqlx::query_scalar(
                        "SELECT id FROM supported_providers WHERE provider = $1",
                    )
                    .bind(&prov)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| PutServiceError::Db(e.to_string()))?
                    .ok_or_else(|| {
                        PutServiceError::Validation(format!(
                            "provider '{}' not in supported_providers",
                            prov
                        ))
                    })?;
                    let id: Option<i64> = sqlx::query_scalar(
                        r#"
                        INSERT INTO models (provider_id, name) VALUES ($1, $2)
                        ON CONFLICT (provider_id, name) DO UPDATE SET name = EXCLUDED.name
                        RETURNING id
                        "#,
                    )
                    .bind(provider_id)
                    .bind(name)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| PutServiceError::Db(e.to_string()))?;
                    id
                }
                None => None,
            }
        };

        // 2. Insert prompt_version (immutable; encrypted)
        let version_row = sqlx::query(
            r#"
            INSERT INTO prompt_versions (function_id, preferred_model_id, template_text, model_config, provider_settings, encrypted_payload, encrypted_dek, nonce, kms_key_id, context_id)
            VALUES ($1, $2, NULL, '{}'::jsonb, '{}'::jsonb, $3, $4, $5, $6, $7)
            RETURNING id, created_at
            "#,
        )
        .bind(function_db_id)
        .bind(preferred_model_id)
        .bind(&blob.encrypted_payload)
        .bind(&blob.encrypted_dek)
        .bind(&blob.nonce)
        .bind(&blob.kms_key_id)
        .bind(context_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| PutServiceError::Db(e.to_string()))?;

        let version_id: i64 = version_row.try_get("id").map_err(|e| PutServiceError::Db(e.to_string()))?;
        let created_at: chrono::DateTime<Utc> =
            version_row.try_get("created_at").map_err(|e| PutServiceError::Db(e.to_string()))?;

        // 3. Upsert deployment (production tag)
        sqlx::query(
            r#"
            INSERT INTO deployments (function_id, version_id, tag, context_id)
            VALUES ($1, $2, 'production', $3)
            ON CONFLICT (function_id, context_id, tag) DO UPDATE SET version_id = EXCLUDED.version_id
            "#,
        )
        .bind(function_db_id)
        .bind(version_id)
        .bind(context_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| PutServiceError::Db(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| PutServiceError::Db(e.to_string()))?;

        // 4. Refresh cache so execute sees the new prompt immediately
        if let Some(ref store) = self.function_store {
            if let Err(e) = store.refresh_deployment(function_id, context_id).await {
                tracing::warn!(%function_id, %context_id, err = %e, "failed to refresh function store cache after persist");
            }
        }

        Ok(StorageResult {
            version_id,
            created_at,
            kms_key_arn: blob.kms_key_id,
        })
    }

    /// Persist provider API key to api_keys. Encrypts with envelope; context_id = AAD.
    async fn persist_api_key(
        &self,
        user_id: &uuid::Uuid,
        workspace_id: &uuid::Uuid,
        provider: &str,
        raw_secret: &str,
        context_id: &str,
    ) -> Result<StorageResult, PutServiceError> {
        // Validate user has access to workspace
        let has_access = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM workspace_members WHERE user_id = $1 AND workspace_id = $2)",
        )
        .bind(user_id)
        .bind(workspace_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| PutServiceError::Db(e.to_string()))?;

        if !has_access {
            return Err(PutServiceError::Validation(
                "workspace access denied".into(),
            ));
        }

        let blob = self
            .enveloper
            .encrypt_and_wrap(raw_secret.to_string(), context_id)
            .await?;

        let label = format!("{} key", provider);

        // Upsert: replace existing key for same (user, workspace, provider)
        let updated = sqlx::query(
            r#"
            UPDATE api_keys
            SET key_encrypted = $1, nonce = $2, encrypted_dek = $3, kms_key_id = $4, label = $5
            WHERE user_id = $6 AND workspace_id = $7 AND provider = $8
            "#,
        )
        .bind(&blob.encrypted_payload)
        .bind(&blob.nonce)
        .bind(&blob.encrypted_dek)
        .bind(&blob.kms_key_id)
        .bind(&label)
        .bind(user_id)
        .bind(workspace_id)
        .bind(provider)
        .execute(&self.db)
        .await
        .map_err(|e| PutServiceError::Db(e.to_string()))?;

        let created_at = if updated.rows_affected() > 0 {
            chrono::Utc::now()
        } else {
            let row = sqlx::query(
                r#"
                INSERT INTO api_keys (user_id, workspace_id, label, key_encrypted, nonce, encrypted_dek, kms_key_id, provider)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING created_at
                "#,
            )
            .bind(user_id)
            .bind(workspace_id)
            .bind(&label)
            .bind(&blob.encrypted_payload)
            .bind(&blob.nonce)
            .bind(&blob.encrypted_dek)
            .bind(&blob.kms_key_id)
            .bind(provider)
            .fetch_one(&self.db)
            .await
            .map_err(|e| PutServiceError::Db(e.to_string()))?;
            row.try_get("created_at").map_err(|e| PutServiceError::Db(e.to_string()))?
        };

        Ok(StorageResult {
            version_id: 0,
            created_at,
            kms_key_arn: blob.kms_key_id,
        })
    }
}
