//! Auth and workspace model types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct MfaSettings {
    pub enabled: bool,
    pub totp_secret_encrypted: Option<Vec<u8>>,
    pub totp_secret_nonce: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyMeta {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub label: String,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub user_id: Uuid,
    pub api_key_added_at: Option<chrono::DateTime<chrono::Utc>>,
    pub default_workspace_id: Option<Uuid>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
