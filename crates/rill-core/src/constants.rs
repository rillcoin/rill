//! Protocol constants. All monetary values in rills (1 RILL = 10^8 rills).

pub const COIN: u64 = 100_000_000;
pub const MAX_SUPPLY: u64 = 21_000_000 * COIN;
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
pub const DEFAULT_P2P_PORT: u16 = 18333;
pub const DEFAULT_RPC_PORT: u16 = 18332;
pub const MAX_BLOCK_SIZE: usize = 1_048_576;
pub const MAX_TX_SIZE: usize = 100_000;
pub const COINBASE_MATURITY: u64 = 100;
pub const MAX_COINBASE_DATA: usize = 100;
pub const MAX_FUTURE_BLOCK_TIME: u64 = 2 * BLOCK_TIME_SECS;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn magic_bytes_spell_rill() { assert_eq!(&MAGIC_BYTES, b"RILL"); }
    #[test]
    fn supply_math() { assert_eq!(INITIAL_REWARD * HALVING_INTERVAL, 10_500_000 * COIN); }
}
