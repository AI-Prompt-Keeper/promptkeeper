//! Separate endpoints for storing API keys and prompts. Envelope encryption (KMS + DEK).
//! Raw secrets never logged; fingerprint only in response; zeroized before send.
//! Keys → api_keys table; Prompts → prompt_versions + deployments (functions table).

mod service;

pub use service::{PutFunctionService, PutServiceError};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// Request body for POST /v1/keys. Stores provider API key (openai, anthropic, gemini, etc.).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutKeyRequestBody {
    /// Raw API key. Never logged; zeroized after use.
    #[serde(deserialize_with = "deserialize_secret")]
    pub raw_secret: Zeroizing<String>,
    /// Provider (e.g. "openai", "anthropic"). Required.
    pub provider: String,
}

/// Request body for POST /v1/prompts. Stores prompt template for a named function.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutPromptRequestBody {
    /// Prompt/function name (e.g. "customer_support"). Required.
    pub name: String,
    /// Raw prompt template (e.g. Handlebars). Never logged; zeroized after use.
    #[serde(deserialize_with = "deserialize_secret")]
    pub raw_secret: Zeroizing<String>,
    /// Optional: default provider for this prompt (e.g. "openai", "anthropic", "gemini").
    pub provider: Option<String>,
    /// Optional: preferred model for this version (e.g. "gpt-4o", "claude-3-5-sonnet-20240620").
    pub preferred_model: Option<String>,
}

fn deserialize_secret<'de, D>(d: D) -> Result<Zeroizing<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(Zeroizing::new(s))
}

/// Metadata returned from storage layer after persisting encrypted secret.
#[derive(Clone, Debug)]
pub struct StorageResult {
    pub version_id: i64,
    pub created_at: DateTime<Utc>,
    pub kms_key_arn: String,
}

/// Response body for PUT keys and prompts. No raw_secret; sensitive fields never serialized.
#[derive(Debug, Serialize)]
pub struct PutStorageResponse {
    pub version_id: i64,
    pub created_at: DateTime<Utc>,
    pub kms_key_arn: String,
    /// Truncated SHA-256 of the raw secret (for verification only; not stored in DB).
    pub fingerprint: String,
}

/// Truncated SHA-256 of input: first 16 bytes as 32 hex chars. Safe to include in response.
pub fn secret_fingerprint(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16.min(digest.len())])
}
