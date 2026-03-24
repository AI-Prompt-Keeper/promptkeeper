//! Cryptographically secure Prompt Keeper client API keys with prefixes and checksum.
//!
//! Format: `{prefix}{64_hex_secret}_{4_hex_checksum}` where checksum is the first 4 hex chars
//! of SHA256(prefix || hex_secret) (ASCII bytes). Minimum 32 bytes entropy in the secret.

use rand::RngCore;
use sha2::{Digest, Sha256};

/// Management key prefix (full RWX: prompts, keys, execute).
pub const PREFIX_MGT_LIVE: &str = "pk_mgt_live_";
/// Execution-only key prefix (POST /v1/execute only).
pub const PREFIX_EXE_LIVE: &str = "pk_exe_live_";

fn compute_checksum(prefix: &str, hex_body: &str) -> String {
    let mut h = Sha256::new();
    h.update(prefix.as_bytes());
    h.update(hex_body.as_bytes());
    let digest = hex::encode(h.finalize());
    digest[..4].to_string()
}

fn generate_with_prefix(prefix: &'static str) -> String {
    let mut secret = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut secret);
    let hex_body = hex::encode(secret);
    let checksum = compute_checksum(prefix, &hex_body);
    format!("{}{}_{}", prefix, hex_body, checksum)
}

/// Generate a new management API key (`pk_mgt_live_...`).
pub fn generate_management_key() -> String {
    generate_with_prefix(PREFIX_MGT_LIVE)
}

/// Generate a new execution-only API key (`pk_exe_live_...`).
pub fn generate_execution_key() -> String {
    generate_with_prefix(PREFIX_EXE_LIVE)
}

/// Validate structural checksum for `pk_mgt_live_` / `pk_exe_live_` keys.
pub fn validate_scoped_key_checksum(token: &str) -> bool {
    let prefix = if token.starts_with(PREFIX_MGT_LIVE) {
        PREFIX_MGT_LIVE
    } else if token.starts_with(PREFIX_EXE_LIVE) {
        PREFIX_EXE_LIVE
    } else {
        return false;
    };
    let rest = match token.get(prefix.len()..) {
        Some(r) => r,
        None => return false,
    };
    let idx = match rest.rfind('_') {
        Some(i) => i,
        None => return false,
    };
    let hex_body = &rest[..idx];
    let checksum = &rest[idx + 1..];
    if hex_body.len() != 64 || !hex_body.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    if checksum.len() != 4 || !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    checksum == compute_checksum(prefix, hex_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn management_key_roundtrip_checksum() {
        let k = generate_management_key();
        assert!(k.starts_with(PREFIX_MGT_LIVE));
        assert!(validate_scoped_key_checksum(&k));
    }

    #[test]
    fn execution_key_roundtrip_checksum() {
        let k = generate_execution_key();
        assert!(k.starts_with(PREFIX_EXE_LIVE));
        assert!(validate_scoped_key_checksum(&k));
    }

    #[test]
    fn tampered_checksum_rejected() {
        let mut k = generate_management_key();
        let last = k.pop().unwrap();
        let wrong = if last == '0' { '1' } else { '0' };
        k.push(wrong);
        assert!(!validate_scoped_key_checksum(&k));
    }
}
