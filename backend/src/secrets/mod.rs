//! Secure AI Gateway "Put" logic: envelope encryption with AWS KMS.
//!
//! Raw secrets (API keys, prompt templates) are encrypted with a unique DEK (AES-256-GCM),
//! the DEK is wrapped by KMS, and the resulting blob is stored. Raw secrets are never logged.

mod envelope;

pub use envelope::{encrypt_and_wrap, unwrap_and_decrypt, EnvelopeError, SecretEnveloper, StorageBlob};
