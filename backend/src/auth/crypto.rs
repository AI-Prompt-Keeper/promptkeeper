//! Crypto: Argon2 password hashing, AES-256-GCM for API keys and MFA secrets.
//! Keys are never logged; plaintext API keys never touch the database.

use aes_gcm::{aead::Aead, aead::KeyInit, Aes256Gcm};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;
use thiserror::Error;

const NONCE_LEN: usize = 12;
#[allow(dead_code)]
const ARGON2_MEMORY: u32 = 19_456;
#[allow(dead_code)]
const ARGON2_ITERATIONS: u32 = 2;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("hashing failed")]
    Hash,
    #[error("encryption/decryption failed")]
    Aes,
    #[error("invalid key length")]
    KeyLength,
}

/// Hash password with Argon2id (OWASP-recommended).
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| CryptoError::Hash)
}

/// Verify password against Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError> {
    let parsed = PasswordHash::new(hash).map_err(|_| CryptoError::Hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Encrypt plaintext with AES-256-GCM. Returns (ciphertext, nonce).
/// Key must be 32 bytes (256 bits). Use a key derived from env in production.
pub fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::KeyLength);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::KeyLength)?;
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(&nonce.into(), plaintext)
        .map_err(|_| CryptoError::Aes)?;
    Ok((ciphertext, nonce.to_vec()))
}

/// Decrypt ciphertext with AES-256-GCM.
pub fn decrypt(key: &[u8], ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != 32 {
        return Err(CryptoError::KeyLength);
    }
    if nonce.len() != NONCE_LEN {
        return Err(CryptoError::KeyLength);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::KeyLength)?;
    let nonce_arr: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| CryptoError::KeyLength)?;
    cipher
        .decrypt((&nonce_arr).into(), ciphertext)
        .map_err(|_| CryptoError::Aes)
}

/// Load 32-byte encryption key from env ENCRYPTION_KEY (base64).
/// If unset, returns a fixed dev key and logs a warning.
pub fn encryption_key_from_env() -> [u8; 32] {
    const DEV_KEY: [u8; 32] = [0x0a; 32]; // dev only
    match std::env::var("ENCRYPTION_KEY") {
        Ok(s) => {
            let decoded = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                s.trim().as_bytes(),
            );
            match decoded {
                Ok(b) if b.len() == 32 => {
                    let mut out = [0u8; 32];
                    out.copy_from_slice(&b);
                    out
                }
                _ => {
                    tracing::warn!("ENCRYPTION_KEY invalid (need 32 bytes base64); using dev key");
                    DEV_KEY
                }
            }
        }
        Err(_) => {
            tracing::warn!("ENCRYPTION_KEY not set; using dev key (do not use in production)");
            DEV_KEY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let key = [0u8; 32];
        let pt = b"sk-secret-key";
        let (ct, nonce) = encrypt(&key, pt).unwrap();
        let dec = decrypt(&key, &ct, &nonce).unwrap();
        assert_eq!(dec.as_slice(), pt);
    }

    #[test]
    fn password_hash_verify() {
        let h = hash_password("hello").unwrap();
        assert!(verify_password("hello", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }
}
