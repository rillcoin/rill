//! Decay-aware coin selection algorithm.
//!
//! Selects UTXOs to spend with a preference for high-decay outputs first,
//! minimizing the total value lost to decay. This is the greedy strategy:
//! spend the coins that are decaying the fastest before they lose more value.

use rill_core::constants::CONCENTRATION_PRECISION;
use rill_core::traits::{ChainState, DecayCalculator};
use rill_core::types::{OutPoint, UtxoEntry};

use crate::error::WalletError;

/// A UTXO annotated with decay-adjusted values for coin selection.
#[derive(Debug, Clone)]
pub struct WalletUtxo {
    /// The outpoint identifying this UTXO.
    pub outpoint: OutPoint,
    /// The UTXO entry from the chain state.
    pub entry: UtxoEntry,
    /// Effective (post-decay) value in rills.
    pub effective_value: u64,
    /// Nominal (pre-decay) value in rills.
    pub nominal_value: u64,
    /// Amount decayed away in rills.
    pub decay_amount: u64,
}

/// Result of coin selection: which UTXOs to spend and the fee/change breakdown.
#[derive(Debug, Clone)]
pub struct CoinSelection {
    /// Selected UTXOs to spend.
    pub selected: Vec<WalletUtxo>,
    /// Total nominal value of selected UTXOs.
    pub total_nominal: u64,
    /// Total effective (post-decay) value of selected UTXOs.
    pub total_effective: u64,
    /// Total decay on selected UTXOs.
    pub total_decay: u64,
    /// Change amount to return to the sender (in rills).
    pub change: u64,
    /// Transaction fee in rills.
    pub fee: u64,
}

/// Decay-aware coin selector.
///
/// Sorts UTXOs by decay amount descending (highest-decay first) to minimize
/// value loss, then greedily selects until the target plus fee is met.
pub struct CoinSelector;

impl CoinSelector {
    /// Select UTXOs to meet a target amount.
    ///
    /// # Arguments
    /// - `utxos` — available wallet UTXOs (outpoint + entry pairs)
    /// - `target` — the amount to send in rills (excluding fee)
    /// - `base_fee` — fixed portion of the fee in rills
    /// - `fee_per_input` — additional fee per input consumed
    /// - `decay_calc` — decay calculator for effective value computation
    /// - `chain_state` — chain state for cluster balance and supply lookups
    /// - `height` — current block height for decay computation
    pub fn select(
        utxos: &[(OutPoint, UtxoEntry)],
        target: u64,
        base_fee: u64,
        fee_per_input: u64,
        decay_calc: &dyn DecayCalculator,
        chain_state: &dyn ChainState,
        height: u64,
    ) -> Result<CoinSelection, WalletError> {
        if utxos.is_empty() {
            return Err(WalletError::NoUtxos);
        }

        if target == 0 {
            return Err(WalletError::InvalidAmount("target must be non-zero".into()));
        }

        let supply = chain_state
            .circulating_supply()
            .map_err(|e| WalletError::BuildError(e.to_string()))?;

        // Build annotated UTXOs with decay info
        let mut wallet_utxos: Vec<WalletUtxo> = Vec::with_capacity(utxos.len());
        for (outpoint, entry) in utxos {
            let nominal = entry.output.value;
            let blocks_held = height.saturating_sub(entry.block_height);

            // Compute concentration: cluster_balance * CONCENTRATION_PRECISION / supply
            let cluster_bal = chain_state
                .cluster_balance(&entry.cluster_id)
                .map_err(|e| WalletError::BuildError(e.to_string()))?;
            let concentration = if supply > 0 {
                ((cluster_bal as u128) * (CONCENTRATION_PRECISION as u128)
                    / (supply as u128)) as u64
            } else {
                0
            };

            let effective = decay_calc
                .effective_value(nominal, concentration, blocks_held)
                .map_err(WalletError::Decay)?;

            let decay_amount = nominal.saturating_sub(effective);

            wallet_utxos.push(WalletUtxo {
                outpoint: outpoint.clone(),
                entry: entry.clone(),
                effective_value: effective,
                nominal_value: nominal,
                decay_amount,
            });
        }

        // Sort by decay_amount descending (spend highest-decay first),
        // then by effective_value ascending (prefer smaller UTXOs as tiebreaker)
        wallet_utxos.sort_by(|a, b| {
            b.decay_amount
                .cmp(&a.decay_amount)
                .then(a.effective_value.cmp(&b.effective_value))
        });

        // Greedy selection
        let mut selected = Vec::new();
        let mut total_effective: u64 = 0;
        let mut total_nominal: u64 = 0;
        let mut total_decay: u64 = 0;

        for utxo in wallet_utxos {
            selected.push(utxo.clone());
            total_effective = total_effective.saturating_add(utxo.effective_value);
            total_nominal = total_nominal.saturating_add(utxo.nominal_value);
            total_decay = total_decay.saturating_add(utxo.decay_amount);

            let fee = base_fee.saturating_add(
                fee_per_input.saturating_mul(selected.len() as u64),
            );
            let needed = target.saturating_add(fee);

            if total_effective >= needed {
                let change = total_effective.saturating_sub(needed);
                return Ok(CoinSelection {
                    selected,
                    total_nominal,
                    total_effective,
                    total_decay,
                    change,
                    fee,
                });
            }
        }

        // Not enough funds
        let fee = base_fee.saturating_add(
            fee_per_input.saturating_mul(selected.len() as u64),
        );
        Err(WalletError::InsufficientFunds {
            have: total_effective,
            need: target.saturating_add(fee),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::constants;
    use rill_core::error::{DecayError, RillError, TransactionError};
    use rill_core::types::{Block, BlockHeader, Hash256, TxOutput, Transaction};
    use std::collections::HashMap;

    // --- Mock chain state ---

    struct MockChainState {
        utxos: HashMap<OutPoint, UtxoEntry>,
        supply: u64,
        clusters: HashMap<Hash256, u64>,
    }

    impl MockChainState {
        fn new(supply: u64) -> Self {
            Self {
                utxos: HashMap::new(),
                supply,
                clusters: HashMap::new(),
            }
        }
    }

    impl ChainState for MockChainState {
        fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
            Ok(self.utxos.get(outpoint).cloned())
        }
        fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
            Ok((100, Hash256::ZERO))
        }
        fn get_block_header(&self, _: &Hash256) -> Result<Option<BlockHeader>, RillError> {
            Ok(None)
        }
        fn get_block(&self, _: &Hash256) -> Result<Option<Block>, RillError> {
            Ok(None)
        }
        fn get_block_hash(&self, _: u64) -> Result<Option<Hash256>, RillError> {
            Ok(None)
        }
        fn circulating_supply(&self) -> Result<u64, RillError> {
            Ok(self.supply)
        }
        fn cluster_balance(&self, cluster_id: &Hash256) -> Result<u64, RillError> {
            Ok(*self.clusters.get(cluster_id).unwrap_or(&0))
        }
        fn decay_pool_balance(&self) -> Result<u64, RillError> {
            Ok(0)
        }
        fn validate_transaction(&self, _: &Transaction) -> Result<(), TransactionError> {
            Ok(())
        }
    }

    // --- Mock decay calculator ---

    struct MockDecayCalculator;

    impl DecayCalculator for MockDecayCalculator {
        fn decay_rate_ppb(&self, concentration_ppb: u64) -> Result<u64, DecayError> {
            if concentration_ppb > constants::DECAY_C_THRESHOLD_PPB {
                Ok(10_000_000) // 1%
            } else {
                Ok(0)
            }
        }

        fn compute_decay(
            &self,
            nominal_value: u64,
            concentration_ppb: u64,
            blocks_held: u64,
        ) -> Result<u64, DecayError> {
            let rate = self.decay_rate_ppb(concentration_ppb)?;
            let per_block = nominal_value
                .checked_mul(rate)
                .and_then(|v| v.checked_div(constants::DECAY_PRECISION))
                .ok_or(DecayError::ArithmeticOverflow)?;
            per_block
                .checked_mul(blocks_held)
                .ok_or(DecayError::ArithmeticOverflow)
        }

        fn decay_pool_release(&self, pool_balance: u64) -> Result<u64, DecayError> {
            Ok(pool_balance * constants::DECAY_POOL_RELEASE_BPS / constants::BPS_PRECISION)
        }
    }

    fn make_utxo(index: u64, value: u64, cluster_id: Hash256, block_height: u64) -> (OutPoint, UtxoEntry) {
        let outpoint = OutPoint {
            txid: Hash256([index as u8; 32]),
            index: 0,
        };
        let entry = UtxoEntry {
            output: TxOutput {
                value,
                pubkey_hash: Hash256::ZERO,
            },
            block_height,
            is_coinbase: false,
            cluster_id,
        };
        (outpoint, entry)
    }

    #[test]
    fn select_single_utxo_exact() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        // Below threshold: no decay
        let utxos = vec![make_utxo(1, 10 * constants::COIN, Hash256::ZERO, 50)];

        let result = CoinSelector::select(&utxos, 8 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap();
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.fee, 1500); // 1000 + 500*1
        assert_eq!(result.total_effective, 10 * constants::COIN);
        assert_eq!(result.change, 10 * constants::COIN - 8 * constants::COIN - 1500);
    }

    #[test]
    fn select_with_change() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![
            make_utxo(1, 5 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(2, 5 * constants::COIN, Hash256::ZERO, 50),
        ];

        let result = CoinSelector::select(&utxos, 3 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap();
        // Should pick at least one UTXO
        assert!(!result.selected.is_empty());
        assert!(result.total_effective >= 3 * constants::COIN + result.fee);
        assert_eq!(result.change, result.total_effective - 3 * constants::COIN - result.fee);
    }

    #[test]
    fn select_multi_utxo() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![
            make_utxo(1, 2 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(2, 2 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(3, 2 * constants::COIN, Hash256::ZERO, 50),
        ];

        let result = CoinSelector::select(&utxos, 5 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap();
        assert_eq!(result.selected.len(), 3);
    }

    #[test]
    fn select_high_decay_first() {
        let whale_cluster = Hash256([0xBB; 32]);
        let mut cs = MockChainState::new(1_000_000 * constants::COIN);
        // Make whale_cluster have high concentration (above threshold)
        cs.clusters.insert(whale_cluster, 500_000 * constants::COIN);

        let dc = MockDecayCalculator;

        let utxos = vec![
            make_utxo(1, 5 * constants::COIN, Hash256::ZERO, 50),     // no decay
            make_utxo(2, 5 * constants::COIN, whale_cluster, 50),      // high decay
        ];

        let result = CoinSelector::select(&utxos, 3 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap();

        // Should select the high-decay UTXO first
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.selected[0].outpoint.txid, Hash256([2; 32]));
        assert!(result.selected[0].decay_amount > 0);
    }

    #[test]
    fn select_insufficient_funds() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![make_utxo(1, 1 * constants::COIN, Hash256::ZERO, 50)];

        let err = CoinSelector::select(&utxos, 10 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::InsufficientFunds { .. }));
    }

    #[test]
    fn select_empty_utxos() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos: Vec<(OutPoint, UtxoEntry)> = vec![];

        let err = CoinSelector::select(&utxos, 1 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap_err();
        assert_eq!(err, WalletError::NoUtxos);
    }

    #[test]
    fn select_zero_target_rejected() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![make_utxo(1, 1 * constants::COIN, Hash256::ZERO, 50)];

        let err = CoinSelector::select(&utxos, 0, 1000, 500, &dc, &cs, 100).unwrap_err();
        assert!(matches!(err, WalletError::InvalidAmount(_)));
    }

    #[test]
    fn select_no_decay_below_threshold() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        // Cluster balance is 0 -> concentration below threshold -> no decay
        let utxos = vec![make_utxo(1, 10 * constants::COIN, Hash256::ZERO, 50)];

        let result = CoinSelector::select(&utxos, 1 * constants::COIN, 0, 0, &dc, &cs, 100).unwrap();
        assert_eq!(result.total_decay, 0);
        assert_eq!(result.total_effective, result.total_nominal);
    }

    #[test]
    fn select_fee_scales_with_inputs() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![
            make_utxo(1, 2 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(2, 2 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(3, 2 * constants::COIN, Hash256::ZERO, 50),
        ];

        let result = CoinSelector::select(&utxos, 5 * constants::COIN, 1000, 500, &dc, &cs, 100).unwrap();
        assert_eq!(result.fee, 1000 + 500 * result.selected.len() as u64);
    }

    #[test]
    fn coin_selection_fields_consistent() {
        let cs = MockChainState::new(1_000_000 * constants::COIN);
        let dc = MockDecayCalculator;
        let utxos = vec![
            make_utxo(1, 5 * constants::COIN, Hash256::ZERO, 50),
            make_utxo(2, 3 * constants::COIN, Hash256::ZERO, 50),
        ];
        let target = 4 * constants::COIN;

        let result = CoinSelector::select(&utxos, target, 1000, 500, &dc, &cs, 100).unwrap();

        // Consistency checks
        assert_eq!(
            result.total_nominal,
            result.total_effective + result.total_decay
        );
        assert_eq!(
            result.total_effective,
            target + result.fee + result.change
        );
    }

    #[test]
    fn wallet_utxo_debug() {
        let utxo = WalletUtxo {
            outpoint: OutPoint {
                txid: Hash256([1; 32]),
                index: 0,
            },
            entry: UtxoEntry {
                output: TxOutput {
                    value: 100,
                    pubkey_hash: Hash256::ZERO,
                },
                block_height: 0,
                is_coinbase: false,
                cluster_id: Hash256::ZERO,
            },
            effective_value: 100,
            nominal_value: 100,
            decay_amount: 0,
        };
        let debug = format!("{utxo:?}");
        assert!(debug.contains("WalletUtxo"));
    }
}
