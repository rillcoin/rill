//! Protocol constants. All monetary values in rills (1 RILL = 10^8 rills).

pub const COIN: u64 = 100_000_000;

/// Maximum mining supply (excluding premine).
///
/// VULN-03 note: The actual total supply includes DEV_FUND_PREMINE.
/// This constant represents only the mining rewards cap, similar to Bitcoin's 21M.
pub const MAX_SUPPLY: u64 = 21_000_000 * COIN;

/// Network type: Mainnet, Testnet, or Regtest.
///
/// Controls magic bytes, default ports, data directory suffix, and minimum
/// proof-of-work difficulty.
///
/// # Examples
///
/// ```
/// use rill_core::constants::NetworkType;
/// let net = NetworkType::default();
/// assert_eq!(net, NetworkType::Mainnet);
/// assert_eq!(net.magic_bytes(), *b"RILL");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NetworkType {
    /// Production network.
    #[default]
    Mainnet,
    /// Public test network with lower difficulty.
    Testnet,
    /// Local regression-test network — minimal difficulty, instant blocks.
    Regtest,
}

impl NetworkType {
    /// Four-byte network identifier prepended to all P2P messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::constants::NetworkType;
    /// assert_eq!(NetworkType::Mainnet.magic_bytes(), *b"RILL");
    /// assert_eq!(NetworkType::Testnet.magic_bytes(), *b"TEST");
    /// assert_eq!(NetworkType::Regtest.magic_bytes(), *b"REGT");
    /// ```
    pub fn magic_bytes(&self) -> [u8; 4] {
        match self {
            Self::Mainnet => [0x52, 0x49, 0x4C, 0x4C], // "RILL"
            Self::Testnet => [0x54, 0x45, 0x53, 0x54], // "TEST"
            Self::Regtest => [0x52, 0x45, 0x47, 0x54], // "REGT"
        }
    }

    /// Default TCP port for P2P connections.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::constants::NetworkType;
    /// assert_eq!(NetworkType::Mainnet.default_p2p_port(), 18333);
    /// ```
    pub fn default_p2p_port(&self) -> u16 {
        match self {
            Self::Mainnet => 18333,
            Self::Testnet => 28333,
            Self::Regtest => 38333,
        }
    }

    /// Default TCP port for the JSON-RPC server.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::constants::NetworkType;
    /// assert_eq!(NetworkType::Mainnet.default_rpc_port(), 18332);
    /// ```
    pub fn default_rpc_port(&self) -> u16 {
        match self {
            Self::Mainnet => 18332,
            Self::Testnet => 28332,
            Self::Regtest => 38332,
        }
    }

    /// Subdirectory name appended to the base data directory path.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::constants::NetworkType;
    /// assert_eq!(NetworkType::Testnet.data_dir_suffix(), "testnet");
    /// ```
    pub fn data_dir_suffix(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Regtest => "regtest",
        }
    }

    /// Minimum allowed difficulty target for this network.
    ///
    /// Regtest uses `u64::MAX` (easiest possible target) so blocks can be
    /// produced instantly without real proof-of-work.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::constants::NetworkType;
    /// assert_eq!(NetworkType::Regtest.min_difficulty(), u64::MAX);
    /// assert_eq!(NetworkType::Mainnet.min_difficulty(), 1);
    /// ```
    pub fn min_difficulty(&self) -> u64 {
        match self {
            Self::Regtest => u64::MAX,
            _ => 1,
        }
    }
}

/// Actual maximum total supply including both mining rewards and premine.
///
/// VULN-03 fix: This constant makes explicit that the total supply exceeds
/// the mining cap due to the 5% dev fund premine.
pub const MAX_TOTAL_SUPPLY: u64 = MAX_SUPPLY + (MAX_SUPPLY * DEV_FUND_BPS / BPS_PRECISION);

pub const INITIAL_REWARD: u64 = 50 * COIN;
pub const HALVING_INTERVAL: u64 = 210_000;
pub const BLOCK_TIME_SECS: u64 = 60;
pub const BLOCKS_PER_YEAR: u64 = 525_960;
pub const MAGIC_BYTES: [u8; 4] = [0x52, 0x49, 0x4C, 0x4C]; // "RILL"
pub const ADDRESS_PREFIX: &str = "rill1";
pub const DIFFICULTY_WINDOW: u64 = 60;
pub const DECAY_R_MAX_PPB: u64 = 1_500_000_000;
pub const DECAY_PRECISION: u64 = 10_000_000_000;
pub const DECAY_C_THRESHOLD_PPB: u64 = 1_000_000;
pub const CONCENTRATION_PRECISION: u64 = 1_000_000_000;
pub const DECAY_K: u64 = 2000;
pub const LINEAGE_HALF_LIFE: u64 = 52_596;
pub const LINEAGE_FULL_RESET: u64 = 525_960;
pub const DECAY_POOL_RELEASE_BPS: u64 = 100;
pub const BPS_PRECISION: u64 = 10_000;
pub const DEV_FUND_BPS: u64 = 500;

/// Dev fund premine amount: 5% of `MAX_SUPPLY` expressed as a constants-layer
/// duplicate so that downstream crates can reference it without depending on
/// the genesis module.
///
/// The authoritative value lives in `genesis::DEV_FUND_PREMINE`; both are
/// computed identically (`MAX_SUPPLY / BPS_PRECISION * DEV_FUND_BPS`).
pub const DEV_FUND_PREMINE_AMOUNT: u64 = MAX_SUPPLY / BPS_PRECISION * DEV_FUND_BPS;

/// Vesting period in blocks: 4 years of blocks at the target block time.
///
/// Linear vesting is enforced by capping dev-fund spending to
/// `DEV_FUND_MAX_SPEND_PER_BLOCK` coins per block height.
///
/// # Examples
///
/// ```
/// use rill_core::constants::{DEV_FUND_VESTING_BLOCKS, BLOCKS_PER_YEAR};
/// assert_eq!(DEV_FUND_VESTING_BLOCKS, BLOCKS_PER_YEAR * 4);
/// ```
pub const DEV_FUND_VESTING_BLOCKS: u64 = BLOCKS_PER_YEAR * 4;

/// Maximum dev-fund coins that may be spent in a single block (linear vesting).
///
/// Uses ceiling division (`div_ceil`) so that the full premine is unlocked
/// within the vesting period even when `DEV_FUND_PREMINE_AMOUNT` is not
/// evenly divisible by `DEV_FUND_VESTING_BLOCKS`.
///
/// # Examples
///
/// ```
/// use rill_core::constants::DEV_FUND_MAX_SPEND_PER_BLOCK;
/// assert!(DEV_FUND_MAX_SPEND_PER_BLOCK > 0);
/// ```
pub const DEV_FUND_MAX_SPEND_PER_BLOCK: u64 =
    DEV_FUND_PREMINE_AMOUNT.div_ceil(DEV_FUND_VESTING_BLOCKS);

pub const DEFAULT_P2P_PORT: u16 = 18333;
pub const DEFAULT_RPC_PORT: u16 = 18332;
pub const MAX_BLOCK_SIZE: usize = 1_048_576;
pub const MAX_TX_SIZE: usize = 100_000;
pub const MAX_INPUTS: usize = 1000;
pub const MAX_OUTPUTS: usize = 1000;
pub const COINBASE_MATURITY: u64 = 100;
pub const MAX_COINBASE_DATA: usize = 100;
pub const MAX_FUTURE_BLOCK_TIME: u64 = 2 * BLOCK_TIME_SECS;
pub const MAX_LOCATOR_SIZE: usize = 64;
pub const LOCKTIME_THRESHOLD: u64 = 500_000_000;
pub const MIN_TX_FEE: u64 = 1000;

/// RandomX key rotation interval. Key block = hash at floor(height / 2048) * 2048.
pub const RANDOMX_KEY_BLOCK_INTERVAL: u64 = 2048;

/// Maximum blocks per peer per minute via request-response.
pub const RATE_LIMIT_BLOCKS_PER_MIN: u32 = 10;
/// Maximum transactions per peer per minute via gossipsub.
pub const RATE_LIMIT_TXS_PER_MIN: u32 = 100;
/// Maximum header requests per peer per minute.
pub const RATE_LIMIT_HEADERS_PER_MIN: u32 = 5;
/// Maximum message size in bytes before deserialization is rejected.
pub const MAX_MESSAGE_SIZE: usize = 2_097_152; // 2 MiB

/// Hard-coded checkpoints: (height, block_hash) pairs.
///
/// During header sync, blocks at checkpoint heights must match the expected
/// hash. Reorgs below the last checkpoint height are rejected.
///
/// Currently empty -- will be populated as the testnet produces known-good
/// blocks. The infrastructure is in place so that adding a checkpoint is a
/// one-line change.
pub const CHECKPOINTS: &[(u64, [u8; 32])] = &[];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_spell_rill() {
        assert_eq!(&MAGIC_BYTES, b"RILL");
    }

    #[test]
    fn supply_math() {
        assert_eq!(INITIAL_REWARD * HALVING_INTERVAL, 10_500_000 * COIN);
    }

    // --- NetworkType ---

    #[test]
    fn network_type_magic_bytes_distinct() {
        let mainnet = NetworkType::Mainnet.magic_bytes();
        let testnet = NetworkType::Testnet.magic_bytes();
        let regtest = NetworkType::Regtest.magic_bytes();
        assert_ne!(mainnet, testnet);
        assert_ne!(mainnet, regtest);
        assert_ne!(testnet, regtest);
    }

    #[test]
    fn network_type_ports_distinct() {
        // P2P ports are all different.
        let ports_p2p = [
            NetworkType::Mainnet.default_p2p_port(),
            NetworkType::Testnet.default_p2p_port(),
            NetworkType::Regtest.default_p2p_port(),
        ];
        assert_ne!(ports_p2p[0], ports_p2p[1]);
        assert_ne!(ports_p2p[0], ports_p2p[2]);
        assert_ne!(ports_p2p[1], ports_p2p[2]);

        // RPC ports are all different.
        let ports_rpc = [
            NetworkType::Mainnet.default_rpc_port(),
            NetworkType::Testnet.default_rpc_port(),
            NetworkType::Regtest.default_rpc_port(),
        ];
        assert_ne!(ports_rpc[0], ports_rpc[1]);
        assert_ne!(ports_rpc[0], ports_rpc[2]);
        assert_ne!(ports_rpc[1], ports_rpc[2]);

        // P2P and RPC ports on the same network must also differ.
        assert_ne!(
            NetworkType::Mainnet.default_p2p_port(),
            NetworkType::Mainnet.default_rpc_port()
        );
    }

    #[test]
    fn network_type_default_is_mainnet() {
        assert_eq!(NetworkType::default(), NetworkType::Mainnet);
    }

    #[test]
    fn dev_fund_premine_is_five_percent() {
        // 5% of MAX_SUPPLY == MAX_SUPPLY * 500 / 10_000
        assert_eq!(
            DEV_FUND_PREMINE_AMOUNT,
            MAX_SUPPLY * DEV_FUND_BPS / BPS_PRECISION
        );
        // Concrete value: 1,050,000 RILL
        assert_eq!(DEV_FUND_PREMINE_AMOUNT, 1_050_000 * COIN);
    }

    #[test]
    fn dev_fund_vesting_blocks_is_four_years() {
        assert_eq!(DEV_FUND_VESTING_BLOCKS, BLOCKS_PER_YEAR * 4);
    }

    #[test]
    fn dev_fund_max_spend_nonzero() {
        assert!(DEV_FUND_MAX_SPEND_PER_BLOCK > 0);
        // Sanity: total vested over the period is at least DEV_FUND_PREMINE_AMOUNT.
        let total_vested = DEV_FUND_MAX_SPEND_PER_BLOCK
            .checked_mul(DEV_FUND_VESTING_BLOCKS)
            .expect("no overflow");
        assert!(total_vested >= DEV_FUND_PREMINE_AMOUNT);
    }

    #[test]
    fn regtest_minimal_difficulty() {
        assert_eq!(NetworkType::Regtest.min_difficulty(), u64::MAX);
        // Mainnet and testnet have normal minimums.
        assert_eq!(NetworkType::Mainnet.min_difficulty(), 1);
        assert_eq!(NetworkType::Testnet.min_difficulty(), 1);
    }
}
