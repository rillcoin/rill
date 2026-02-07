//! AES-256-GCM wallet file encryption.
//!
//! Phase 1 uses BLAKE3 for password-based key derivation. This is fast but
//! not memory-hard, making it less resistant to brute-force attacks than
//! argon2. Production wallets should upgrade to argon2id in Phase 2.
//!
//! # Wire format
//! ```text
//! salt (32 bytes) || nonce (12 bytes) || ciphertext + auth_tag
//! ```

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::error::WalletError;

/// BLAKE3 KDF context for password-derived encryption keys.
const PASSWORD_KDF_CONTEXT: &str = "rill-wallet-password-kdf-v1";

/// Salt length in bytes.
const SALT_LEN: usize = 32;

/// AES-GCM nonce length in bytes.
const NONCE_LEN: usize = 12;

/// Minimum encrypted payload size (salt + nonce + auth tag).
const MIN_ENCRYPTED_LEN: usize = SALT_LEN + NONCE_LEN + 16;

/// Derive a 256-bit encryption key from a password and salt using BLAKE3.
///
/// Phase 1: BLAKE3 derive_key is fast but not memory-hard.
/// TODO: Replace with argon2id for production wallet files.
pub fn derive_key(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut ikm = Vec::with_capacity(password.len() + salt.len());
    ikm.extend_from_slice(password);
    ikm.extend_from_slice(salt);
    blake3::derive_key(PASSWORD_KDF_CONTEXT, &ikm)
}

/// Encrypt plaintext with a password using AES-256-GCM.
///
/// Generates a random 32-byte salt and 12-byte nonce. Returns
/// `salt || nonce || ciphertext+tag`.
pub fn encrypt(plaintext: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    use rand::RngCore;
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;

    let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data that was encrypted with [`encrypt`].
///
/// Parses the salt and nonce from the header, derives the key from the
/// password, and decrypts the ciphertext. Returns [`WalletError::InvalidPassword`]
/// if the password is wrong (authentication tag mismatch).
pub fn decrypt(encrypted: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    if encrypted.len() < MIN_ENCRYPTED_LEN {
        return Err(WalletError::CorruptedFile(format!(
            "encrypted data too short: {} < {MIN_ENCRYPTED_LEN}",
            encrypted.len()
        )));
    }

    let salt = &encrypted[..SALT_LEN];
    let nonce_bytes = &encrypted[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &encrypted[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::Decryption(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| WalletError::InvalidPassword)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let password = b"correct horse battery staple";
        let plaintext = b"secret wallet data";

        let encrypted = encrypt(plaintext, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_decrypt_empty_data() {
        let password = b"password";
        let plaintext = b"";

        let encrypted = encrypt(plaintext, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext.to_vec());
    }

    #[test]
    fn encrypt_decrypt_large_data() {
        let password = b"password";
        let plaintext = vec![0xABu8; 10_000];

        let encrypted = encrypt(&plaintext, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_password_fails() {
        let password = b"correct";
        let wrong = b"wrong";
        let plaintext = b"secret";

        let encrypted = encrypt(plaintext, password).unwrap();
        let err = decrypt(&encrypted, wrong).unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn truncated_data_fails() {
        let err = decrypt(&[0u8; 10], b"password").unwrap_err();
        assert!(matches!(err, WalletError::CorruptedFile(_)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let password = b"password";
        let plaintext = b"secret data";

        let mut encrypted = encrypt(plaintext, password).unwrap();
        // Flip a byte in the ciphertext portion
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;

        let err = decrypt(&encrypted, password).unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn tampered_salt_fails() {
        let password = b"password";
        let plaintext = b"secret";

        let mut encrypted = encrypt(plaintext, password).unwrap();
        // Flip a byte in the salt
        encrypted[0] ^= 0xFF;

        let err = decrypt(&encrypted, password).unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn tampered_nonce_fails() {
        let password = b"password";
        let plaintext = b"secret";

        let mut encrypted = encrypt(plaintext, password).unwrap();
        // Flip a byte in the nonce
        encrypted[SALT_LEN] ^= 0xFF;

        let err = decrypt(&encrypted, password).unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn derive_key_deterministic() {
        let key1 = derive_key(b"password", b"salt");
        let key2 = derive_key(b"password", b"salt");
        assert_eq!(key1, key2);
    }

    #[test]
    fn derive_key_different_passwords() {
        let key1 = derive_key(b"password1", b"salt");
        let key2 = derive_key(b"password2", b"salt");
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_key_different_salts() {
        let key1 = derive_key(b"password", b"salt1");
        let key2 = derive_key(b"password", b"salt2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn encrypted_has_correct_overhead() {
        let password = b"password";
        let plaintext = b"hello";

        let encrypted = encrypt(plaintext, password).unwrap();
        // salt(32) + nonce(12) + plaintext(5) + tag(16) = 65
        assert_eq!(encrypted.len(), SALT_LEN + NONCE_LEN + plaintext.len() + 16);
    }
}
