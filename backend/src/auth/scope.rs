//! Scope for Prompt Keeper client API keys (`api_tokens.scope`).

/// Stored in PostgreSQL as `mgt` or `exe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiKeyScope {
    /// Full read-write-execute: store prompts/keys, execute, mint execution keys.
    Mgt,
    /// Read+execute: `POST /v1/execute` and `GET /v1/list-prompts` (titles only).
    Exe,
}

impl ApiKeyScope {
    pub fn as_str(self) -> &'static str {
        match self {
            ApiKeyScope::Mgt => "mgt",
            ApiKeyScope::Exe => "exe",
        }
    }

    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "mgt" => Some(ApiKeyScope::Mgt),
            "exe" => Some(ApiKeyScope::Exe),
            _ => None,
        }
    }
}
