//! BIP-39 mnemonic seed backup and restoration.

use bip39::{Language, Mnemonic};

use crate::error::WalletError;
use crate::keys::Seed;

/// Convert a 32-byte seed to a 24-word BIP-39 mnemonic phrase.
pub fn seed_to_mnemonic(seed: &Seed) -> String {
    let m = Mnemonic::from_entropy_in(Language::English, seed.as_bytes())
        .expect("32 bytes always produces valid mnemonic");
    m.to_string()
}

/// Parse a BIP-39 mnemonic phrase and extract the 32-byte entropy as a Seed.
///
/// Normalizes whitespace and converts to lowercase before parsing.
pub fn mnemonic_to_seed(phrase: &str) -> Result<Seed, WalletError> {
    let normalized = phrase
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let m = Mnemonic::parse_in(Language::English, &normalized)
        .map_err(|e| WalletError::InvalidMnemonic(e.to_string()))?;
    let entropy = m.to_entropy();
    if entropy.len() != 32 {
        return Err(WalletError::InvalidMnemonic(format!(
            "expected 32 bytes of entropy, got {}",
            entropy.len()
        )));
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&entropy);
    Ok(Seed::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a random seed, convert to mnemonic, convert back, assert equal.
    #[test]
    fn roundtrip_generate() {
        let seed = Seed::generate();
        let phrase = seed_to_mnemonic(&seed);
        let restored = mnemonic_to_seed(&phrase).expect("roundtrip should succeed");
        assert_eq!(seed.as_bytes(), restored.as_bytes());
    }

    /// Fixed 32-byte seed -> mnemonic -> seed; assert roundtrip.
    #[test]
    fn roundtrip_known_vector() {
        let bytes = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let seed = Seed::from_bytes(bytes);
        let phrase = seed_to_mnemonic(&seed);
        let restored = mnemonic_to_seed(&phrase).expect("known vector roundtrip should succeed");
        assert_eq!(restored.as_bytes(), &bytes);
    }

    /// A 32-byte seed should always produce a 24-word mnemonic.
    #[test]
    fn mnemonic_is_24_words() {
        let seed = Seed::from_bytes([0xAB; 32]);
        let phrase = seed_to_mnemonic(&seed);
        let word_count = phrase.split_whitespace().count();
        assert_eq!(word_count, 24, "expected 24 words, got {word_count}: {phrase}");
    }

    /// A phrase containing an invalid BIP-39 word must be rejected.
    #[test]
    fn invalid_word_rejected() {
        let result = mnemonic_to_seed("abandon abandon abandon invalidword");
        assert!(result.is_err(), "expected error for invalid word");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid mnemonic"), "error message was: {err_msg}");
    }

    /// Valid BIP-39 words but wrong last word (checksum mismatch) must be rejected.
    #[test]
    fn bad_checksum_rejected() {
        // "abandon" repeated 23 times + "zoo" has wrong checksum for 24-word entropy
        let words = vec!["abandon"; 23];
        let mut phrase = words.join(" ");
        phrase.push_str(" zoo");
        let result = mnemonic_to_seed(&phrase);
        assert!(result.is_err(), "expected checksum error for: {phrase}");
    }

    /// Extra spaces and tabs in the mnemonic must be normalized away.
    #[test]
    fn whitespace_normalization() {
        let seed = Seed::from_bytes([0x55; 32]);
        let clean_phrase = seed_to_mnemonic(&seed);
        // Insert extra whitespace between words
        let messy_phrase = clean_phrase.split_whitespace().collect::<Vec<_>>().join("   ");
        let restored = mnemonic_to_seed(&messy_phrase).expect("normalized whitespace should parse");
        assert_eq!(seed.as_bytes(), restored.as_bytes());
    }

    /// A phrase with only 2 words (way too few) must be rejected.
    #[test]
    fn wrong_word_count_rejected() {
        let result = mnemonic_to_seed("abandon abandon");
        assert!(result.is_err(), "expected error for only 2 words");
    }
}
