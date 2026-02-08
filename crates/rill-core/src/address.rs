//! Address encoding for the Rill network.
//!
//! Addresses use Bech32m encoding ([BIP-350]) with human-readable prefixes:
//! - Mainnet: `rill1...`
//! - Testnet: `trill1...`
//!
//! Each address encodes a version byte (currently 0) and a 32-byte BLAKE3
//! pubkey hash. The Bech32m checksum provides error detection with a
//! guaranteed detection of up to 4 character errors.
//!
//! [BIP-350]: https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use crate::crypto::PublicKey;
use crate::error::AddressError;
use crate::types::Hash256;

/// Bech32m checksum constant (BIP-350).
const BECH32M_CONST: u32 = 0x2bc830a3;

/// Bech32 character set for encoding 5-bit values.
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Current address version.
pub const ADDRESS_VERSION: u8 = 0;

/// Network identifier determining the address prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Network {
    /// Mainnet (HRP: "rill", addresses start with `rill1`).
    Mainnet,
    /// Testnet (HRP: "trill", addresses start with `trill1`).
    Testnet,
}

impl Network {
    /// Human-readable prefix for this network.
    pub fn hrp(&self) -> &'static str {
        match self {
            Network::Mainnet => "rill",
            Network::Testnet => "trill",
        }
    }

    /// Look up network from a human-readable prefix.
    pub fn from_hrp(hrp: &str) -> Result<Self, AddressError> {
        match hrp {
            "rill" => Ok(Network::Mainnet),
            "trill" => Ok(Network::Testnet),
            _ => Err(AddressError::UnknownNetwork(hrp.to_string())),
        }
    }
}

/// A Rill network address encoding a pubkey hash with Bech32m.
///
/// Human-readable form is `rill1...` (mainnet) or `trill1...` (testnet).
/// Internally stores the network, version byte, and 32-byte BLAKE3 pubkey hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Address {
    network: Network,
    version: u8,
    pubkey_hash: Hash256,
}

impl Address {
    /// Create an address from a pubkey hash and network.
    pub fn from_pubkey_hash(pubkey_hash: Hash256, network: Network) -> Self {
        Self {
            network,
            version: ADDRESS_VERSION,
            pubkey_hash,
        }
    }

    /// Create an address from a public key and network.
    pub fn from_public_key(public_key: &PublicKey, network: Network) -> Self {
        Self::from_pubkey_hash(public_key.pubkey_hash(), network)
    }

    /// The BLAKE3 pubkey hash encoded in this address.
    pub fn pubkey_hash(&self) -> Hash256 {
        self.pubkey_hash
    }

    /// The network this address belongs to.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The address version byte.
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Encode this address as a Bech32m string.
    pub fn encode(&self) -> String {
        let hrp = self.network.hrp();
        // Convert 32-byte hash from 8-bit to 5-bit groups
        let data_5bit = convert_bits(self.pubkey_hash.as_bytes(), 8, 5, true)
            .expect("valid 32-byte hash always converts to 5-bit");

        // Prepend version byte
        let mut payload = Vec::with_capacity(1 + data_5bit.len());
        payload.push(self.version);
        payload.extend_from_slice(&data_5bit);

        let checksum = bech32m_create_checksum(hrp, &payload);

        let mut result = String::with_capacity(hrp.len() + 1 + payload.len() + 6);
        result.push_str(hrp);
        result.push('1');
        for &d in &payload {
            result.push(CHARSET[d as usize] as char);
        }
        for &d in &checksum {
            result.push(CHARSET[d as usize] as char);
        }
        result
    }

    /// Decode a Bech32m address string.
    pub fn decode(s: &str) -> Result<Self, AddressError> {
        // Reject mixed case (Bech32 spec: all alpha chars must be same case)
        let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
        if has_lower && has_upper {
            return Err(AddressError::MixedCase);
        }

        let s_lower = s.to_ascii_lowercase();

        // Find the last '1' separator
        let sep_pos = s_lower.rfind('1').ok_or(AddressError::MissingSeparator)?;

        if sep_pos == 0 {
            return Err(AddressError::InvalidHrp);
        }
        // Need at least 6 checksum chars + 1 version char after separator
        if sep_pos + 8 > s_lower.len() {
            return Err(AddressError::InvalidLength);
        }

        let hrp = &s_lower[..sep_pos];
        let data_part = &s_lower[sep_pos + 1..];

        // Decode characters from Bech32 charset
        let mut data = Vec::with_capacity(data_part.len());
        for c in data_part.chars() {
            let pos = CHARSET
                .iter()
                .position(|&ch| ch as char == c)
                .ok_or(AddressError::InvalidCharacter(c))?;
            data.push(pos as u8);
        }

        // Verify Bech32m checksum
        if !bech32m_verify_checksum(hrp, &data) {
            return Err(AddressError::InvalidChecksum);
        }

        // Remove 6-char checksum
        let payload = &data[..data.len() - 6];

        if payload.is_empty() {
            return Err(AddressError::InvalidLength);
        }

        // First value is version
        let version = payload[0];
        if version != ADDRESS_VERSION {
            return Err(AddressError::InvalidVersion(version));
        }

        // Convert remaining 5-bit data back to 8-bit
        let hash_bytes = convert_bits(&payload[1..], 5, 8, false)
            .ok_or(AddressError::InvalidPadding)?;

        if hash_bytes.len() != 32 {
            return Err(AddressError::InvalidLength);
        }

        let network = Network::from_hrp(hrp)?;

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);

        Ok(Self {
            network,
            version,
            pubkey_hash: Hash256(hash),
        })
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::decode(s)
    }
}

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::decode(&s).map_err(serde::de::Error::custom)
    }
}

// --- Bech32m internals ---

/// Compute the Bech32m polymod over a sequence of 5-bit values.
fn bech32m_polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk: u32 = 1;
    for &v in values {
        let b = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ (v as u32);
        for (i, &g) in GEN.iter().enumerate() {
            if (b >> i) & 1 != 0 {
                chk ^= g;
            }
        }
    }
    chk
}

/// Expand the HRP for Bech32m checksum computation.
fn bech32m_hrp_expand(hrp: &str) -> Vec<u8> {
    let mut ret = Vec::with_capacity(hrp.len() * 2 + 1);
    for c in hrp.bytes() {
        ret.push(c >> 5);
    }
    ret.push(0);
    for c in hrp.bytes() {
        ret.push(c & 31);
    }
    ret
}

/// Create the 6-value Bech32m checksum for the given HRP and data.
fn bech32m_create_checksum(hrp: &str, data: &[u8]) -> Vec<u8> {
    let mut values = bech32m_hrp_expand(hrp);
    values.extend_from_slice(data);
    values.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    let polymod = bech32m_polymod(&values) ^ BECH32M_CONST;
    (0..6)
        .map(|i| ((polymod >> (5 * (5 - i))) & 31) as u8)
        .collect()
}

/// Verify the Bech32m checksum for the given HRP and data (including checksum).
fn bech32m_verify_checksum(hrp: &str, data: &[u8]) -> bool {
    let mut values = bech32m_hrp_expand(hrp);
    values.extend_from_slice(data);
    bech32m_polymod(&values) == BECH32M_CONST
}

/// Convert between bit widths (e.g. 8-bit bytes to 5-bit Bech32 groups).
fn convert_bits(data: &[u8], from_bits: u32, to_bits: u32, pad: bool) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let mut ret = Vec::new();
    let maxv = (1u32 << to_bits) - 1;
    for &value in data {
        let v = value as u32;
        if v >> from_bits != 0 {
            return None;
        }
        acc = (acc << from_bits) | v;
        bits += from_bits;
        while bits >= to_bits {
            bits -= to_bits;
            ret.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 {
            ret.push(((acc << (to_bits - bits)) & maxv) as u8);
        }
    } else if bits >= from_bits || ((acc << (to_bits - bits)) & maxv) != 0 {
        return None;
    }
    Some(ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    fn sample_hash() -> Hash256 {
        Hash256([0xAA; 32])
    }

    // --- Network ---

    #[test]
    fn network_hrp_mainnet() {
        assert_eq!(Network::Mainnet.hrp(), "rill");
    }

    #[test]
    fn network_hrp_testnet() {
        assert_eq!(Network::Testnet.hrp(), "trill");
    }

    #[test]
    fn network_from_hrp_mainnet() {
        assert_eq!(Network::from_hrp("rill").unwrap(), Network::Mainnet);
    }

    #[test]
    fn network_from_hrp_testnet() {
        assert_eq!(Network::from_hrp("trill").unwrap(), Network::Testnet);
    }

    #[test]
    fn network_from_hrp_unknown() {
        assert_eq!(
            Network::from_hrp("bitcoin").unwrap_err(),
            AddressError::UnknownNetwork("bitcoin".into())
        );
    }

    // --- Encoding ---

    #[test]
    fn encode_mainnet_starts_with_rill1() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode();
        assert!(encoded.starts_with("rill1"));
    }

    #[test]
    fn encode_testnet_starts_with_trill1() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        let encoded = addr.encode();
        assert!(encoded.starts_with("trill1"));
    }

    #[test]
    fn encode_is_lowercase() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode();
        assert_eq!(encoded, encoded.to_ascii_lowercase());
    }

    #[test]
    fn encode_deterministic() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        assert_eq!(addr.encode(), addr.encode());
    }

    #[test]
    fn encode_different_hashes_differ() {
        let a1 = Address::from_pubkey_hash(Hash256([0xAA; 32]), Network::Mainnet);
        let a2 = Address::from_pubkey_hash(Hash256([0xBB; 32]), Network::Mainnet);
        assert_ne!(a1.encode(), a2.encode());
    }

    #[test]
    fn encode_different_networks_differ() {
        let a1 = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let a2 = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        assert_ne!(a1.encode(), a2.encode());
    }

    #[test]
    fn encode_mainnet_length() {
        // "rill" (4) + "1" (1) + version (1) + 52 data chars + 6 checksum = 64
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        assert_eq!(addr.encode().len(), 64);
    }

    #[test]
    fn encode_testnet_length() {
        // "trill" (5) + "1" (1) + version (1) + 52 data chars + 6 checksum = 65
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        assert_eq!(addr.encode().len(), 65);
    }

    // --- Decoding ---

    #[test]
    fn decode_mainnet_roundtrip() {
        let original = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = original.encode();
        let decoded = Address::decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_testnet_roundtrip() {
        let original = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        let encoded = original.encode();
        let decoded = Address::decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_uppercase_valid() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode().to_ascii_uppercase();
        let decoded = Address::decode(&encoded).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn decode_mixed_case_fails() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode();
        // Capitalize just the first character of the data part
        let mut mixed = encoded.clone();
        // Find a lowercase letter after "rill1" and uppercase it
        let bytes = unsafe { mixed.as_bytes_mut() };
        for b in bytes[5..].iter_mut() {
            if b.is_ascii_lowercase() {
                *b = b.to_ascii_uppercase();
                break;
            }
        }
        assert_eq!(Address::decode(&mixed).unwrap_err(), AddressError::MixedCase);
    }

    #[test]
    fn decode_invalid_checksum() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let mut encoded = addr.encode();
        // Flip the last character
        let last = encoded.pop().unwrap();
        let replacement = if last == 'q' { 'p' } else { 'q' };
        encoded.push(replacement);
        assert_eq!(
            Address::decode(&encoded).unwrap_err(),
            AddressError::InvalidChecksum
        );
    }

    #[test]
    fn decode_invalid_character() {
        // 'b', 'i', 'o' are not in the Bech32 charset
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode();
        // Replace a data character with 'b' (invalid in bech32)
        let mut bad = encoded[..6].to_string();
        bad.push('b');
        bad.push_str(&encoded[7..]);
        assert!(matches!(
            Address::decode(&bad).unwrap_err(),
            AddressError::InvalidCharacter('b')
        ));
    }

    #[test]
    fn decode_missing_separator() {
        assert_eq!(
            Address::decode("rillnoseparator").unwrap_err(),
            AddressError::MissingSeparator
        );
    }

    #[test]
    fn decode_empty_hrp() {
        assert_eq!(
            Address::decode("1qqqqqqqqqq").unwrap_err(),
            AddressError::InvalidHrp
        );
    }

    #[test]
    fn decode_too_short() {
        assert_eq!(
            Address::decode("rill1qqqq").unwrap_err(),
            AddressError::InvalidLength
        );
    }

    #[test]
    fn decode_unknown_network() {
        assert!(matches!(
            Network::from_hrp("xill").unwrap_err(),
            AddressError::UnknownNetwork(_)
        ));
    }

    // --- Roundtrips ---

    #[test]
    fn roundtrip_from_public_key() {
        let kp = KeyPair::generate();
        let pk = kp.public_key();
        let addr = Address::from_public_key(&pk, Network::Mainnet);

        let encoded = addr.encode();
        let decoded = Address::decode(&encoded).unwrap();

        assert_eq!(decoded.pubkey_hash(), pk.pubkey_hash());
        assert_eq!(decoded.network(), Network::Mainnet);
        assert_eq!(decoded.version(), ADDRESS_VERSION);
    }

    #[test]
    fn roundtrip_zero_hash() {
        let addr = Address::from_pubkey_hash(Hash256::ZERO, Network::Mainnet);
        let decoded = Address::decode(&addr.encode()).unwrap();
        assert_eq!(decoded.pubkey_hash(), Hash256::ZERO);
    }

    #[test]
    fn roundtrip_max_hash() {
        let addr = Address::from_pubkey_hash(Hash256([0xFF; 32]), Network::Mainnet);
        let decoded = Address::decode(&addr.encode()).unwrap();
        assert_eq!(decoded.pubkey_hash(), Hash256([0xFF; 32]));
    }

    #[test]
    fn roundtrip_many_hashes() {
        for i in 0u8..=10 {
            let hash = Hash256([i.wrapping_mul(37); 32]);
            let addr = Address::from_pubkey_hash(hash, Network::Mainnet);
            let decoded = Address::decode(&addr.encode()).unwrap();
            assert_eq!(decoded.pubkey_hash(), hash);
        }
    }

    // --- Accessors ---

    #[test]
    fn pubkey_hash_accessor() {
        let hash = sample_hash();
        let addr = Address::from_pubkey_hash(hash, Network::Mainnet);
        assert_eq!(addr.pubkey_hash(), hash);
    }

    #[test]
    fn network_accessor() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        assert_eq!(addr.network(), Network::Testnet);
    }

    #[test]
    fn version_accessor() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        assert_eq!(addr.version(), ADDRESS_VERSION);
    }

    // --- Display / FromStr ---

    #[test]
    fn display_matches_encode() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        assert_eq!(format!("{addr}"), addr.encode());
    }

    #[test]
    fn from_str_roundtrip() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let encoded = addr.encode();
        let parsed: Address = encoded.parse().unwrap();
        assert_eq!(addr, parsed);
    }

    // --- Serde ---

    #[test]
    fn serde_json_roundtrip() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Mainnet);
        let json = serde_json::to_string(&addr).unwrap();
        // Should serialize as a string, not an object
        assert!(json.starts_with('"'));
        assert!(json.contains("rill1"));
        let decoded: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn serde_json_testnet_roundtrip() {
        let addr = Address::from_pubkey_hash(sample_hash(), Network::Testnet);
        let json = serde_json::to_string(&addr).unwrap();
        assert!(json.contains("trill1"));
        let decoded: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, decoded);
    }

    // --- Bech32m internals ---

    #[test]
    fn convert_bits_8_to_5_roundtrip() {
        let original = [0xDE, 0xAD, 0xBE, 0xEF];
        let five_bit = convert_bits(&original, 8, 5, true).unwrap();
        let back = convert_bits(&five_bit, 5, 8, false).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn convert_bits_32_bytes_to_5_bit() {
        let data = [0u8; 32];
        let five_bit = convert_bits(&data, 8, 5, true).unwrap();
        // 32 * 8 = 256 bits, ceil(256/5) = 52 groups
        assert_eq!(five_bit.len(), 52);
    }

    #[test]
    fn checksum_verifies() {
        let hrp = "rill";
        let data: Vec<u8> = vec![0; 53]; // version + 52 five-bit groups
        let checksum = bech32m_create_checksum(hrp, &data);
        let mut full = data;
        full.extend_from_slice(&checksum);
        assert!(bech32m_verify_checksum(hrp, &full));
    }

    #[test]
    fn checksum_fails_with_wrong_data() {
        let hrp = "rill";
        let data: Vec<u8> = vec![0; 53];
        let checksum = bech32m_create_checksum(hrp, &data);
        let mut full = data;
        full.extend_from_slice(&checksum);
        // Tamper with data
        full[10] ^= 1;
        assert!(!bech32m_verify_checksum(hrp, &full));
    }

    #[test]
    fn checksum_fails_with_wrong_hrp() {
        let data: Vec<u8> = vec![0; 53];
        let checksum = bech32m_create_checksum("rill", &data);
        let mut full = data;
        full.extend_from_slice(&checksum);
        assert!(!bech32m_verify_checksum("trill", &full));
    }

    // --- Error display ---

    #[test]
    fn error_display() {
        let e = AddressError::InvalidChecksum;
        assert!(!format!("{e}").is_empty());
    }
}
