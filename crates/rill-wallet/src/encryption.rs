//! AES-256-GCM wallet file encryption.
//!
//! Phase 2 uses argon2id for password-based key derivation. BLAKE3 (used in
//! Phase 1) is supported for backward compatibility.
//!
//! # Wire format
//! ```text
//! version (1 byte) || salt (32 bytes) || nonce (12 bytes) || ciphertext + auth_tag
//! ```
//!
//! Legacy Phase 1 format (no version byte):
//! ```text
//! salt (32 bytes) || nonce (12 bytes) || ciphertext + auth_tag
//! ```

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::error::WalletError;

/// Version byte for legacy BLAKE3 KDF.
const VERSION_BLAKE3: u8 = 0x01;

/// Version byte for argon2id KDF.
const VERSION_ARGON2ID: u8 = 0x02;

/// BLAKE3 KDF context for password-derived encryption keys.
const PASSWORD_KDF_CONTEXT: &str = "rill-wallet-password-kdf-v1";

/// Salt length in bytes.
const SALT_LEN: usize = 32;

/// AES-GCM nonce length in bytes.
const NONCE_LEN: usize = 12;

/// Minimum encrypted payload size for v2+ (version + salt + nonce + auth tag).
const MIN_ENCRYPTED_LEN: usize = 1 + SALT_LEN + NONCE_LEN + 16;

/// Minimum encrypted payload size for v1/legacy (salt + nonce + auth tag, no version byte).
const MIN_ENCRYPTED_LEN_V1: usize = SALT_LEN + NONCE_LEN + 16;

/// Argon2id parameters: 64 MB memory, 3 iterations, 1 lane.
const ARGON2_M_COST: u32 = 65536; // 64 MB in KiB
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 1;

/// Derive a 256-bit encryption key from a password and salt using BLAKE3.
///
/// Legacy function for v1 wallet decryption. Not memory-hard.
/// New wallets use argon2id.
fn derive_key_blake3(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut ikm = Vec::with_capacity(password.len() + salt.len());
    ikm.extend_from_slice(password);
    ikm.extend_from_slice(salt);
    blake3::derive_key(PASSWORD_KDF_CONTEXT, &ikm)
}

/// Derive a 256-bit key from password and salt using argon2id.
///
/// Uses 64 MB memory, 3 iterations, 1 lane.
fn derive_key_argon2id(password: &[u8], salt: &[u8]) -> Result<[u8; 32], WalletError> {
    use argon2::{Argon2, Algorithm, Version, Params};
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
        .map_err(|e| WalletError::Encryption(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2.hash_password_into(password, salt, &mut key)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;
    Ok(key)
}

/// Encrypt plaintext with a password using AES-256-GCM.
///
/// Generates a random 32-byte salt and 12-byte nonce. Returns
/// `version || salt || nonce || ciphertext+tag` using argon2id KDF.
pub fn encrypt(plaintext: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    use rand::RngCore;
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_key_argon2id(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;

    let mut result = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
    result.push(VERSION_ARGON2ID);
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt data that was encrypted with [`encrypt`].
///
/// Supports versioned format (argon2id or legacy BLAKE3) and legacy v1 format
/// (BLAKE3, no version byte). Returns [`WalletError::InvalidPassword`]
/// if the password is wrong (authentication tag mismatch).
pub fn decrypt(encrypted: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    if encrypted.len() < MIN_ENCRYPTED_LEN_V1 {
        return Err(WalletError::CorruptedFile(format!(
            "encrypted data too short: {} < {MIN_ENCRYPTED_LEN_V1}",
            encrypted.len()
        )));
    }

    // Check first byte for version
    let first_byte = encrypted[0];
    match first_byte {
        VERSION_ARGON2ID => decrypt_v2(encrypted, password),
        VERSION_BLAKE3 => decrypt_v1(&encrypted[1..], password),
        _ => {
            // Legacy format (no version byte) — try BLAKE3
            // Legacy data starts directly with salt, first byte could be any value
            decrypt_v1(encrypted, password)
        }
    }
}

/// Decrypt v2 format: version(1) || salt(32) || nonce(12) || ciphertext+tag
fn decrypt_v2(encrypted: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    if encrypted.len() < MIN_ENCRYPTED_LEN {
        return Err(WalletError::CorruptedFile(format!(
            "encrypted data too short: {} < {MIN_ENCRYPTED_LEN}",
            encrypted.len()
        )));
    }
    let data = &encrypted[1..]; // skip version byte
    let salt = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    let key = derive_key_argon2id(password, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| WalletError::Decryption(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| WalletError::InvalidPassword)
}

/// Decrypt v1/legacy format: salt(32) || nonce(12) || ciphertext+tag
fn decrypt_v1(data: &[u8], password: &[u8]) -> Result<Vec<u8>, WalletError> {
    if data.len() < MIN_ENCRYPTED_LEN_V1 {
        return Err(WalletError::CorruptedFile(
            "encrypted data too short for v1".to_string()
        ));
    }
    let salt = &data[..SALT_LEN];
    let nonce_bytes = &data[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &data[SALT_LEN + NONCE_LEN..];

    let key = derive_key_blake3(password, salt);
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
    fn derive_key_blake3_deterministic() {
        let key1 = derive_key_blake3(b"password", b"salt");
        let key2 = derive_key_blake3(b"password", b"salt");
        assert_eq!(key1, key2);
    }

    #[test]
    fn derive_key_blake3_different_passwords() {
        let key1 = derive_key_blake3(b"password1", b"salt");
        let key2 = derive_key_blake3(b"password2", b"salt");
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_key_blake3_different_salts() {
        let key1 = derive_key_blake3(b"password", b"salt1");
        let key2 = derive_key_blake3(b"password", b"salt2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn encrypted_has_correct_overhead() {
        let password = b"password";
        let plaintext = b"hello";

        let encrypted = encrypt(plaintext, password).unwrap();
        // version(1) + salt(32) + nonce(12) + plaintext(5) + tag(16) = 66
        assert_eq!(encrypted.len(), 1 + SALT_LEN + NONCE_LEN + plaintext.len() + 16);
    }

    #[test]
    fn encrypt_decrypt_roundtrip_v2() {
        let password = b"correct horse battery staple";
        let plaintext = b"secret wallet data v2";

        let encrypted = encrypt(plaintext, password).unwrap();
        // Verify it's v2 format
        assert_eq!(encrypted[0], VERSION_ARGON2ID);
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn legacy_blake3_decrypt() {
        use rand::RngCore;
        let password = b"legacy password";
        let plaintext = b"old wallet data";

        // Create a v1-format encrypted blob manually
        let mut salt = [0u8; SALT_LEN];
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

        let key = derive_key_blake3(password, &salt);
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();

        // v1 format: salt || nonce || ciphertext (no version byte)
        let mut legacy_encrypted = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        legacy_encrypted.extend_from_slice(&salt);
        legacy_encrypted.extend_from_slice(&nonce_bytes);
        legacy_encrypted.extend_from_slice(&ciphertext);

        // decrypt() should handle it
        let decrypted = decrypt(&legacy_encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn unknown_version_error() {
        use rand::RngCore;
        let password = b"password";

        // Create a blob with an unknown version byte 0xFF
        let mut bad_data = vec![0xFFu8];
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        bad_data.extend_from_slice(&salt);
        bad_data.extend_from_slice(&nonce);
        bad_data.extend_from_slice(&[0u8; 16]); // dummy ciphertext

        // Should fail — fallback to v1 BLAKE3 will fail to decrypt
        let err = decrypt(&bad_data, password).unwrap_err();
        assert_eq!(err, WalletError::InvalidPassword);
    }

    #[test]
    fn different_salts_produce_different_keys_argon2() {
        let password = b"password";
        let salt1 = b"salt1111111111111111111111111111";
        let salt2 = b"salt2222222222222222222222222222";

        let key1 = derive_key_argon2id(password, salt1).unwrap();
        let key2 = derive_key_argon2id(password, salt2).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn argon2id_deterministic() {
        let password = b"password";
        let salt = b"saltsaltsaltsaltsaltsaltsaltsal";

        let key1 = derive_key_argon2id(password, salt).unwrap();
        let key2 = derive_key_argon2id(password, salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn encrypt_v2_has_version_byte() {
        let password = b"password";
        let plaintext = b"test";

        let encrypted = encrypt(plaintext, password).unwrap();
        assert_eq!(encrypted[0], VERSION_ARGON2ID);
        assert!(encrypted.len() >= MIN_ENCRYPTED_LEN);
    }
}
