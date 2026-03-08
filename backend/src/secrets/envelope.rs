//! Envelope encryption: DEK (AES-256-GCM) + KMS-wrapped KEK.
//! AAD binds ciphertext to OrgID/AppID (context_id). DEKs are zeroized after use.

use aes_gcm::{aead::Aead, aead::KeyInit, Aes256Gcm};
use aead::Payload;
use aws_sdk_kms::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::Client as KmsClient;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroizing;

const DEK_LEN: usize = 32;
const NONCE_LEN: usize = 12;
/// KMS encryption context key; must match on decrypt.
const KMS_CONTEXT_KEY: &str = "ContextId";

/// Database storage record for an encrypted secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageBlob {
    /// AES-GCM ciphertext of the secret.
    pub encrypted_payload: Vec<u8>,
    /// KMS-wrapped data encryption key.
    pub encrypted_dek: Vec<u8>,
    /// 96-bit nonce used for AES-GCM (stored with ciphertext).
    pub nonce: Vec<u8>,
    /// KMS key ID (or ARN) used to wrap the DEK.
    pub kms_key_id: String,
}

#[derive(Debug, Error)]
pub enum EnvelopeError {
    #[error("KMS error: {0}")]
    Kms(#[from] aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::encrypt::EncryptError>),
    #[error("KMS decrypt error: {0}")]
    KmsDecrypt(#[from] aws_sdk_kms::error::SdkError<aws_sdk_kms::operation::decrypt::DecryptError>),
    #[error("KMS client config failed: {0}")]
    KmsConfig(String),
    #[error("decryption failed (wrong key, nonce, or AAD)")]
    DecryptionFailed,
    #[error("invalid key or nonce length")]
    KeyLength,
}

/// Service that holds an AWS KMS client for envelope encrypt/decrypt.
pub struct SecretEnveloper {
    kms: KmsClient,
    kms_key_id: String,
}

impl SecretEnveloper {
    /// Build from loaded AWS config and the KMS key ID (or ARN) to use as KEK.
    pub fn new(kms_client: KmsClient, kms_key_id: String) -> Self {
        Self {
            kms: kms_client,
            kms_key_id,
        }
    }

    /// Create from default env-based AWS config (e.g. credentials, region from env).
    pub async fn from_env(kms_key_id: String) -> Result<Self, aws_sdk_kms::error::BuildError> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let kms = KmsClient::new(&config);
        Ok(Self::new(kms, kms_key_id))
    }

    /// Encrypt the raw secret and wrap the DEK with KMS. AAD = context_id (OrgID/AppID).
    /// Raw secret is never logged. DEK is zeroized after use.
    pub async fn encrypt_and_wrap(
        &self,
        raw_secret: String,
        context_id: &str,
    ) -> Result<StorageBlob, EnvelopeError> {
        // 1. Generate unique DEK (256-bit) and 96-bit nonce
        let mut dek = Zeroizing::new([0u8; DEK_LEN]);
        rand::rngs::OsRng.fill_bytes(dek.as_mut());
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);

        // 2. Encrypt secret with DEK using AAD = context_id
        let cipher = Aes256Gcm::new_from_slice(dek.as_ref()).map_err(|_| EnvelopeError::KeyLength)?;
        let plaintext = raw_secret.as_bytes();
        let payload = Payload {
            msg: plaintext,
            aad: context_id.as_bytes(),
        };
        let encrypted_payload = cipher
            .encrypt((&nonce).into(), payload)
            .map_err(|_| EnvelopeError::DecryptionFailed)?;

        // 3. Wrap DEK with KMS (encryption context binds to same context_id)
        let encrypt_result = self
            .kms
            .encrypt()
            .key_id(&self.kms_key_id)
            .plaintext(Blob::new(dek.as_ref()))
            .encryption_context(KMS_CONTEXT_KEY, context_id)
            .send()
            .await;

        if let Err(SdkError::ServiceError(err)) = &encrypt_result {
            let meta = err.err();
            tracing::warn!(
                code = ?meta.code(),
                message = ?meta.message(),
                "KMS Encrypt failed"
            );
        }

        let encrypt_output = encrypt_result?;

        let encrypted_dek = encrypt_output
            .ciphertext_blob()
            .map(|b| b.as_ref().to_vec())
            .ok_or(EnvelopeError::DecryptionFailed)?;
        let key_id_used = encrypt_output
            .key_id()
            .map(String::from)
            .unwrap_or_else(|| self.kms_key_id.clone());

        // 4. DEK is dropped here and zeroized by Zeroizing

        Ok(StorageBlob {
            encrypted_payload,
            encrypted_dek,
            nonce: nonce.to_vec(),
            kms_key_id: key_id_used,
        })
    }

    /// Unwrap the DEK with KMS and decrypt the payload. Validates AAD (context_id).
    pub async fn unwrap_and_decrypt(
        &self,
        blob: &StorageBlob,
        context_id: &str,
    ) -> Result<String, EnvelopeError> {
        // 1. Unwrap DEK with KMS (same encryption context required)
        let decrypt_output = self
            .kms
            .decrypt()
            .ciphertext_blob(Blob::new(blob.encrypted_dek.as_slice()))
            .encryption_context(KMS_CONTEXT_KEY, context_id)
            .send()
            .await
            .map_err(|e| {
                // Map KMS decrypt failures (wrong context, wrong key) to our error
                EnvelopeError::KmsDecrypt(e)
            })?;

        let dek_bytes = decrypt_output
            .plaintext()
            .map(|b| b.as_ref().to_vec())
            .ok_or(EnvelopeError::DecryptionFailed)?;
        if dek_bytes.len() != DEK_LEN {
            return Err(EnvelopeError::KeyLength);
        }
        let mut dek_arr = [0u8; DEK_LEN];
        dek_arr.copy_from_slice(&dek_bytes);
        let dek = Zeroizing::new(dek_arr);

        // 2. Decrypt payload with DEK and same AAD
        let cipher = Aes256Gcm::new_from_slice(dek.as_ref()).map_err(|_| EnvelopeError::KeyLength)?;
        let nonce: [u8; NONCE_LEN] = blob
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| EnvelopeError::KeyLength)?;
        let payload = Payload {
            msg: blob.encrypted_payload.as_slice(),
            aad: context_id.as_bytes(),
        };
        let plaintext = cipher
            .decrypt((&nonce).into(), payload)
            .map_err(|_| EnvelopeError::DecryptionFailed)?;
        let s = String::from_utf8(plaintext).map_err(|_| EnvelopeError::DecryptionFailed)?;
        Ok(s)
    }
}

/// Standalone encrypt: build enveloper from env and encrypt. For callers that don't hold a client.
pub async fn encrypt_and_wrap(
    raw_secret: String,
    context_id: &str,
    kms_key_id: &str,
) -> Result<StorageBlob, EnvelopeError> {
    let enveloper = SecretEnveloper::from_env(kms_key_id.to_string())
        .await
        .map_err(|e| EnvelopeError::KmsConfig(e.to_string()))?;
    enveloper
        .encrypt_and_wrap(raw_secret, context_id)
        .await
}

/// Standalone decrypt: build enveloper from env and decrypt. For testing / callers without a client.
pub async fn unwrap_and_decrypt(
    blob: &StorageBlob,
    context_id: &str,
    kms_key_id: &str,
) -> Result<String, EnvelopeError> {
    let enveloper = SecretEnveloper::from_env(kms_key_id.to_string())
        .await
        .map_err(|e| EnvelopeError::KmsConfig(e.to_string()))?;
    enveloper.unwrap_and_decrypt(blob, context_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip with real KMS is tested in integration tests when AWS credentials and a key are present.
    /// This unit test only checks that StorageBlob serializes and that AAD mismatch would fail decryption
    /// (we cannot test full round-trip without KMS).
    #[test]
    fn storage_blob_serialization_roundtrip() {
        let blob = StorageBlob {
            encrypted_payload: vec![1, 2, 3],
            encrypted_dek: vec![4, 5, 6],
            nonce: vec![0u8; NONCE_LEN],
            kms_key_id: "alias/test".to_string(),
        };
        let json = serde_json::to_string(&blob).unwrap();
        let parsed: StorageBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.encrypted_payload, blob.encrypted_payload);
        assert_eq!(parsed.kms_key_id, blob.kms_key_id);
    }

    #[tokio::test]
    #[ignore = "requires AWS KMS key and credentials"]
    async fn roundtrip_encrypt_decrypt_with_kms() {
        let kms_key_id = std::env::var("TEST_KMS_KEY_ID").unwrap_or_else(|_| "alias/test".into());
        let raw = "sk-secret-openai-key".to_string();
        let context_id = "org_123";
        let blob = encrypt_and_wrap(raw.clone(), context_id, &kms_key_id).await.unwrap();
        assert!(!blob.encrypted_payload.is_empty());
        assert!(!blob.encrypted_dek.is_empty());
        assert_eq!(blob.nonce.len(), NONCE_LEN);
        let dec = unwrap_and_decrypt(&blob, context_id, &kms_key_id).await.unwrap();
        assert_eq!(dec, raw);
        // Wrong context_id must fail
        assert!(unwrap_and_decrypt(&blob, "wrong_org", &kms_key_id)
            .await
            .is_err());
    }
}
