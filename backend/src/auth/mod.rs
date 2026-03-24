//! Auth: crypto (Argon2, AES-GCM), types (User, Session, Workspace), token validation.

pub mod api_token;
pub mod crypto;
pub mod keygen;
pub mod scope;
pub mod session;
pub mod types;

pub use scope::ApiKeyScope;
