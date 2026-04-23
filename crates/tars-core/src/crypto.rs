//! Encryption module for project secrets
//!
//! Uses AES-256-GCM for encryption with a master key stored in the OS keychain.
//! - macOS: Keychain
//! - Windows: Credential Manager
//! - Linux: Secret Service (GNOME Keyring, `KWallet`)
//!
//! Debug and test builds intentionally default to a deterministic local dev key
//! so local development does not constantly prompt for keychain access. Set
//! `TARS_USE_SYSTEM_KEYRING=1` to force debug builds back onto the OS keychain.

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use sha2::{Digest, Sha256};
use thiserror::Error;

const KEYRING_SERVICE: &str = "com.tars.desktop";
const KEYRING_USER: &str = "master-key";
const FORCE_SYSTEM_KEYRING_ENV: &str = "TARS_USE_SYSTEM_KEYRING";
const DEV_KEY_SEED: &str = "tars-debug-master-key:v1";

/// Crypto errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Invalid key format")]
    InvalidKey,
}

/// Get or create the master encryption key from the OS keychain.
///
/// On first call, generates a random 256-bit key and stores it.
/// Subsequent calls retrieve the stored key.
fn get_or_create_master_key() -> Result<[u8; 32], CryptoError> {
    if should_use_local_dev_key() {
        return Ok(local_dev_master_key());
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| CryptoError::Keyring(e.to_string()))?;

    // Try to get existing key
    match entry.get_password() {
        Ok(hex_key) => {
            let bytes = hex::decode(&hex_key).map_err(|_| CryptoError::InvalidKey)?;
            if bytes.len() != 32 {
                return Err(CryptoError::InvalidKey);
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(key)
        }
        Err(keyring::Error::NoEntry) => {
            // Generate new key
            let key = Aes256Gcm::generate_key(OsRng);
            let hex_key = hex::encode(key.as_slice());
            entry
                .set_password(&hex_key)
                .map_err(|e| CryptoError::Keyring(e.to_string()))?;
            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(key.as_slice());
            Ok(key_arr)
        }
        Err(e) => Err(CryptoError::Keyring(e.to_string())),
    }
}

fn should_use_local_dev_key() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }

    !env_flag_enabled(FORCE_SYSTEM_KEYRING_ENV)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn local_dev_master_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(KEYRING_SERVICE.as_bytes());
    hasher.update(b":");
    hasher.update(KEYRING_USER.as_bytes());
    hasher.update(b":");
    hasher.update(DEV_KEY_SEED.as_bytes());

    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

/// Encrypt a plaintext string.
///
/// Returns `(nonce_hex, ciphertext_hex)`.
///
/// # Errors
/// Returns an error if encryption fails or the keychain is unavailable.
pub fn encrypt(plaintext: &str) -> Result<(String, String), CryptoError> {
    let key_bytes = get_or_create_master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    Ok((hex::encode(nonce.as_slice()), hex::encode(&ciphertext)))
}

/// Decrypt a ciphertext using the stored nonce.
///
/// # Errors
/// Returns an error if decryption fails or the keychain is unavailable.
pub fn decrypt(nonce_hex: &str, ciphertext_hex: &str) -> Result<String, CryptoError> {
    let key_bytes = get_or_create_master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes =
        hex::decode(nonce_hex).map_err(|_| CryptoError::Decryption("Invalid nonce".into()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = hex::decode(ciphertext_hex)
        .map_err(|_| CryptoError::Decryption("Invalid ciphertext".into()))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| CryptoError::Decryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = "my-secret-api-key-12345";
        let (nonce, ciphertext) = encrypt(plaintext).expect("encryption failed");
        let decrypted = decrypt(&nonce, &ciphertext).expect("decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_debug_builds_default_to_local_dev_key() {
        assert!(should_use_local_dev_key());
    }
}
