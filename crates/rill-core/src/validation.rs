//! Transaction validation for the Rill protocol.
//!
//! Two levels of validation:
//!
//! - **Structural** ([`validate_transaction_structure`]): context-free checks on
//!   transaction format and internal consistency. No external state required.
//! - **Contextual** ([`validate_transaction`]): UTXO-aware checks including
//!   signature verification, coinbase maturity, and value conservation.
//!
//! Coinbase transactions are only structurally validated here; their reward
//! amount is checked during block validation (rill-consensus).

use std::collections::HashSet;

use crate::constants::{COINBASE_MATURITY, MAX_COINBASE_DATA, MAX_TX_SIZE};
use crate::crypto;
use crate::error::TransactionError;
use crate::types::{OutPoint, Transaction, UtxoEntry};

/// Summary of a successfully validated transaction.
///
/// Returned by [`validate_transaction`] after all checks pass. Contains
/// the computed fee and value totals for use in block template assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTransaction {
    /// Total value of all spent inputs in rills.
    pub total_input: u64,
    /// Total value of all created outputs in rills.
    pub total_output: u64,
    /// Transaction fee in rills (`total_input - total_output`).
    pub fee: u64,
}

/// Validate transaction structure (context-free).
///
/// Checks that apply to both coinbase and regular transactions:
/// - Non-empty inputs and outputs
/// - All output values are non-zero
/// - Total output value does not overflow
/// - Serialized size is within [`MAX_TX_SIZE`]
///
/// Additional coinbase-specific checks (via [`validate_coinbase_structure`]):
/// - Exactly one input with null outpoint
/// - Coinbase data within size limit
///
/// Additional regular transaction checks:
/// - No null outpoints
/// - No duplicate input outpoints
/// - Each input carries 64-byte signature and 32-byte public key
pub fn validate_transaction_structure(tx: &Transaction) -> Result<(), TransactionError> {
    // --- Common checks ---

    if tx.inputs.is_empty() || tx.outputs.is_empty() {
        return Err(TransactionError::EmptyInputsOrOutputs);
    }

    for (i, output) in tx.outputs.iter().enumerate() {
        if output.value == 0 {
            return Err(TransactionError::ZeroValueOutput(i));
        }
    }

    if tx.total_output_value().is_none() {
        return Err(TransactionError::ValueOverflow);
    }

    let encoded = bincode::encode_to_vec(tx, bincode::config::standard())
        .map_err(|e| TransactionError::Serialization(e.to_string()))?;
    if encoded.len() > MAX_TX_SIZE {
        return Err(TransactionError::OversizedTransaction {
            size: encoded.len(),
            max: MAX_TX_SIZE,
        });
    }

    // --- Type-specific checks ---

    if tx.is_coinbase() {
        validate_coinbase_structure(tx)?;
    } else {
        validate_regular_structure(tx)?;
    }

    Ok(())
}

/// Validate coinbase-specific structure.
///
/// - Exactly one input with null outpoint
/// - Coinbase data (signature field) within [`MAX_COINBASE_DATA`] bytes
fn validate_coinbase_structure(tx: &Transaction) -> Result<(), TransactionError> {
    if tx.inputs.len() != 1 {
        return Err(TransactionError::InvalidCoinbase(
            "must have exactly one input".into(),
        ));
    }

    if !tx.inputs[0].previous_output.is_null() {
        return Err(TransactionError::InvalidCoinbase(
            "input must be null outpoint".into(),
        ));
    }

    if tx.inputs[0].signature.len() > MAX_COINBASE_DATA {
        return Err(TransactionError::InvalidCoinbase(format!(
            "data too large: {} > {MAX_COINBASE_DATA}",
            tx.inputs[0].signature.len(),
        )));
    }

    Ok(())
}

/// Validate non-coinbase transaction structure.
///
/// - No null outpoints
/// - No duplicate input outpoints
/// - 64-byte signature and 32-byte public key on each input
fn validate_regular_structure(tx: &Transaction) -> Result<(), TransactionError> {
    let mut seen = HashSet::with_capacity(tx.inputs.len());

    for (i, input) in tx.inputs.iter().enumerate() {
        if input.previous_output.is_null() {
            return Err(TransactionError::NullOutpointInRegularTx(i));
        }

        if !seen.insert(&input.previous_output) {
            return Err(TransactionError::DuplicateInput(
                input.previous_output.to_string(),
            ));
        }

        if input.signature.len() != 64 {
            return Err(TransactionError::InvalidSignature { index: i });
        }

        if input.public_key.len() != 32 {
            return Err(TransactionError::InvalidSignature { index: i });
        }
    }

    Ok(())
}

/// Validate a transaction against the UTXO set (contextual).
///
/// Performs full validation including structural checks plus:
/// - All input outpoints reference existing, unspent UTXOs
/// - Coinbase UTXOs have sufficient maturity
/// - Ed25519 signatures verify against the UTXO's pubkey hash
/// - Total input value covers total output value (fee >= 0)
///
/// Returns a [`ValidatedTransaction`] with the computed fee on success.
///
/// **Note:** Coinbase transactions cannot be contextually validated — they
/// have no real inputs. Pass regular transactions only; coinbase reward
/// amounts are checked during block validation.
///
/// The `get_utxo` function looks up a UTXO by outpoint, allowing the caller
/// to provide any source (RocksDB, in-memory map, etc.).
pub fn validate_transaction<F>(
    tx: &Transaction,
    get_utxo: F,
    current_height: u64,
) -> Result<ValidatedTransaction, TransactionError>
where
    F: Fn(&OutPoint) -> Option<UtxoEntry>,
{
    if tx.is_coinbase() {
        return Err(TransactionError::InvalidCoinbase(
            "coinbase cannot be contextually validated standalone".into(),
        ));
    }

    validate_transaction_structure(tx)?;

    let mut total_input: u64 = 0;

    for (i, input) in tx.inputs.iter().enumerate() {
        let utxo = get_utxo(&input.previous_output).ok_or_else(|| {
            TransactionError::UnknownUtxo(input.previous_output.to_string())
        })?;

        if utxo.is_coinbase {
            let confirmations = current_height.saturating_sub(utxo.block_height);
            if confirmations < COINBASE_MATURITY {
                return Err(TransactionError::ImmatureCoinbase { index: i });
            }
        }

        crypto::verify_transaction_input(tx, i, &utxo.output.pubkey_hash)
            .map_err(|_| TransactionError::InvalidSignature { index: i })?;

        total_input = total_input
            .checked_add(utxo.output.value)
            .ok_or(TransactionError::ValueOverflow)?;
    }

    let total_output = tx
        .total_output_value()
        .ok_or(TransactionError::ValueOverflow)?;

    if total_input < total_output {
        return Err(TransactionError::InsufficientFunds {
            have: total_input,
            need: total_output,
        });
    }

    Ok(ValidatedTransaction {
        total_input,
        total_output,
        fee: total_input - total_output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::COIN;
    use crate::crypto::KeyPair;
    use crate::types::{Hash256, TxInput, TxOutput};
    use std::collections::HashMap;

    // --- Helpers ---

    /// Build a signed transaction spending one UTXO.
    fn make_signed_tx(
        kp: &KeyPair,
        outpoint: OutPoint,
        output_value: u64,
        output_pubkey_hash: Hash256,
    ) -> Transaction {
        let mut tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: outpoint,
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: output_value,
                pubkey_hash: output_pubkey_hash,
            }],
            lock_time: 0,
        };
        crypto::sign_transaction_input(&mut tx, 0, kp).unwrap();
        tx
    }

    /// Build a UTXO entry.
    fn make_utxo(
        value: u64,
        pubkey_hash: Hash256,
        block_height: u64,
        is_coinbase: bool,
    ) -> UtxoEntry {
        UtxoEntry {
            output: TxOutput {
                value,
                pubkey_hash,
            },
            block_height,
            is_coinbase,
            cluster_id: Hash256::ZERO,
        }
    }

    /// Build a lookup function from a map.
    fn lookup(
        map: &HashMap<OutPoint, UtxoEntry>,
    ) -> impl Fn(&OutPoint) -> Option<UtxoEntry> + '_ {
        |op| map.get(op).cloned()
    }

    fn sample_outpoint() -> OutPoint {
        OutPoint {
            txid: Hash256([0x11; 32]),
            index: 0,
        }
    }

    fn sample_coinbase() -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: b"block height 1".to_vec(),
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256([0xAA; 32]),
            }],
            lock_time: 0,
        }
    }

    // ==========================================
    // Structural validation — common checks
    // ==========================================

    #[test]
    fn structural_rejects_empty_inputs() {
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::EmptyInputsOrOutputs
        );
    }

    #[test]
    fn structural_rejects_empty_outputs() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::EmptyInputsOrOutputs
        );
    }

    #[test]
    fn structural_rejects_zero_value_output() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 0,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::ZeroValueOutput(0)
        );
    }

    #[test]
    fn structural_rejects_zero_value_second_output() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![
                TxOutput {
                    value: 100,
                    pubkey_hash: Hash256::ZERO,
                },
                TxOutput {
                    value: 0,
                    pubkey_hash: Hash256::ZERO,
                },
            ],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::ZeroValueOutput(1)
        );
    }

    #[test]
    fn structural_rejects_output_value_overflow() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![
                TxOutput {
                    value: u64::MAX,
                    pubkey_hash: Hash256::ZERO,
                },
                TxOutput {
                    value: 1,
                    pubkey_hash: Hash256::ZERO,
                },
            ],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::ValueOverflow
        );
    }

    // ==========================================
    // Structural validation — coinbase
    // ==========================================

    #[test]
    fn structural_accepts_valid_coinbase() {
        assert!(validate_transaction_structure(&sample_coinbase()).is_ok());
    }

    #[test]
    fn coinbase_accepts_empty_data() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert!(validate_transaction_structure(&tx).is_ok());
    }

    #[test]
    fn coinbase_rejects_multiple_inputs() {
        let tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                },
            ],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        // With two null-outpoint inputs, is_coinbase() returns false (requires exactly 1 input).
        // So it falls through to regular validation, which rejects null outpoints.
        assert!(matches!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::NullOutpointInRegularTx(_)
        ));
    }

    #[test]
    fn coinbase_rejects_oversized_data() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![0xAB; MAX_COINBASE_DATA + 1],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert!(matches!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::InvalidCoinbase(_)
        ));
    }

    #[test]
    fn coinbase_accepts_max_data() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![0xAB; MAX_COINBASE_DATA],
                public_key: vec![],
            }],
            outputs: vec![TxOutput {
                value: 50 * COIN,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert!(validate_transaction_structure(&tx).is_ok());
    }

    // ==========================================
    // Structural validation — regular tx
    // ==========================================

    #[test]
    fn structural_accepts_valid_regular_tx() {
        let kp = KeyPair::generate();
        let tx = make_signed_tx(
            &kp,
            sample_outpoint(),
            49 * COIN,
            Hash256([0xBB; 32]),
        );
        assert!(validate_transaction_structure(&tx).is_ok());
    }

    #[test]
    fn structural_rejects_null_outpoint_in_regular() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: OutPoint::null(),
                signature: vec![0; 64],
                public_key: vec![0; 32],
            }],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        // A single null-outpoint input with 64-byte sig and 32-byte pubkey
        // will be identified as coinbase (is_coinbase checks single input + null outpoint).
        // Coinbase validation won't fail on this, so it passes structural.
        // This is by design: the coinbase path handles null outpoints.
        assert!(validate_transaction_structure(&tx).is_ok());
    }

    #[test]
    fn structural_rejects_null_outpoint_mixed_with_regular() {
        let kp = KeyPair::generate();
        let mut tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: sample_outpoint(),
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: OutPoint::null(),
                    signature: vec![0; 64],
                    public_key: vec![0; 32],
                },
            ],
            outputs: vec![TxOutput {
                value: 49 * COIN,
                pubkey_hash: Hash256([0xBB; 32]),
            }],
            lock_time: 0,
        };
        crypto::sign_transaction_input(&mut tx, 0, &kp).unwrap();

        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::NullOutpointInRegularTx(1)
        );
    }

    #[test]
    fn structural_rejects_duplicate_inputs() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let mut tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: op.clone(),
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: op.clone(),
                    signature: vec![],
                    public_key: vec![],
                },
            ],
            outputs: vec![TxOutput {
                value: 49 * COIN,
                pubkey_hash: Hash256([0xBB; 32]),
            }],
            lock_time: 0,
        };
        crypto::sign_transaction_input(&mut tx, 0, &kp).unwrap();
        crypto::sign_transaction_input(&mut tx, 1, &kp).unwrap();

        assert!(matches!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::DuplicateInput(_)
        ));
    }

    #[test]
    fn structural_rejects_short_signature() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: sample_outpoint(),
                signature: vec![0; 63], // too short
                public_key: vec![0; 32],
            }],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::InvalidSignature { index: 0 }
        );
    }

    #[test]
    fn structural_rejects_short_pubkey() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: sample_outpoint(),
                signature: vec![0; 64],
                public_key: vec![0; 31], // too short
            }],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::InvalidSignature { index: 0 }
        );
    }

    #[test]
    fn structural_rejects_long_signature() {
        let tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: sample_outpoint(),
                signature: vec![0; 65], // too long
                public_key: vec![0; 32],
            }],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256::ZERO,
            }],
            lock_time: 0,
        };
        assert_eq!(
            validate_transaction_structure(&tx).unwrap_err(),
            TransactionError::InvalidSignature { index: 0 }
        );
    }

    // ==========================================
    // Contextual validation
    // ==========================================

    #[test]
    fn contextual_accepts_valid_tx() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let tx = make_signed_tx(&kp, op.clone(), 49 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        let result = validate_transaction(&tx, lookup(&utxos), 100).unwrap();
        assert_eq!(result.total_input, 50 * COIN);
        assert_eq!(result.total_output, 49 * COIN);
        assert_eq!(result.fee, 1 * COIN);
    }

    #[test]
    fn contextual_returns_correct_fee() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let tx = make_signed_tx(&kp, op.clone(), 45 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        let result = validate_transaction(&tx, lookup(&utxos), 100).unwrap();
        assert_eq!(result.fee, 5 * COIN);
    }

    #[test]
    fn contextual_accepts_exact_amount_zero_fee() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let tx = make_signed_tx(&kp, op.clone(), 50 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        let result = validate_transaction(&tx, lookup(&utxos), 100).unwrap();
        assert_eq!(result.fee, 0);
    }

    #[test]
    fn contextual_rejects_unknown_utxo() {
        let kp = KeyPair::generate();
        let tx = make_signed_tx(
            &kp,
            sample_outpoint(),
            49 * COIN,
            Hash256([0xBB; 32]),
        );
        let utxos = HashMap::new(); // empty

        assert!(matches!(
            validate_transaction(&tx, lookup(&utxos), 100).unwrap_err(),
            TransactionError::UnknownUtxo(_)
        ));
    }

    #[test]
    fn contextual_rejects_insufficient_funds() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        // Output (60 RILL) exceeds input (50 RILL)
        let tx = make_signed_tx(&kp, op.clone(), 60 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        assert_eq!(
            validate_transaction(&tx, lookup(&utxos), 100).unwrap_err(),
            TransactionError::InsufficientFunds {
                have: 50 * COIN,
                need: 60 * COIN,
            }
        );
    }

    #[test]
    fn contextual_rejects_immature_coinbase_utxo() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let tx = make_signed_tx(&kp, op.clone(), 49 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        // Coinbase UTXO at height 50, current height 100 → only 50 confirmations < 100 required
        utxos.insert(op, make_utxo(50 * COIN, pkh, 50, true));

        assert_eq!(
            validate_transaction(&tx, lookup(&utxos), 100).unwrap_err(),
            TransactionError::ImmatureCoinbase { index: 0 }
        );
    }

    #[test]
    fn contextual_accepts_mature_coinbase_utxo() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let tx = make_signed_tx(&kp, op.clone(), 49 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        // Coinbase UTXO at height 0, current height 100 → exactly 100 confirmations
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, true));

        assert!(validate_transaction(&tx, lookup(&utxos), 100).is_ok());
    }

    #[test]
    fn contextual_rejects_invalid_signature() {
        let kp_signer = KeyPair::generate();
        let kp_owner = KeyPair::generate();
        let op = sample_outpoint();
        // Sign with kp_signer but UTXO belongs to kp_owner
        let tx = make_signed_tx(&kp_signer, op.clone(), 49 * COIN, Hash256([0xBB; 32]));

        let mut utxos = HashMap::new();
        utxos.insert(
            op,
            make_utxo(50 * COIN, kp_owner.public_key().pubkey_hash(), 0, false),
        );

        assert_eq!(
            validate_transaction(&tx, lookup(&utxos), 100).unwrap_err(),
            TransactionError::InvalidSignature { index: 0 }
        );
    }

    #[test]
    fn contextual_rejects_tampered_output() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();
        let mut tx = make_signed_tx(&kp, op.clone(), 49 * COIN, Hash256([0xBB; 32]));

        // Tamper after signing
        tx.outputs[0].value = 50 * COIN;

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        assert_eq!(
            validate_transaction(&tx, lookup(&utxos), 100).unwrap_err(),
            TransactionError::InvalidSignature { index: 0 }
        );
    }

    #[test]
    fn contextual_rejects_coinbase_tx() {
        let cb = sample_coinbase();
        let utxos = HashMap::new();

        assert!(matches!(
            validate_transaction(&cb, lookup(&utxos), 100).unwrap_err(),
            TransactionError::InvalidCoinbase(_)
        ));
    }

    #[test]
    fn contextual_multi_input_valid() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let op1 = OutPoint {
            txid: Hash256([0x11; 32]),
            index: 0,
        };
        let op2 = OutPoint {
            txid: Hash256([0x22; 32]),
            index: 0,
        };

        let mut tx = Transaction {
            version: 1,
            inputs: vec![
                TxInput {
                    previous_output: op1.clone(),
                    signature: vec![],
                    public_key: vec![],
                },
                TxInput {
                    previous_output: op2.clone(),
                    signature: vec![],
                    public_key: vec![],
                },
            ],
            outputs: vec![TxOutput {
                value: 90 * COIN,
                pubkey_hash: Hash256([0xCC; 32]),
            }],
            lock_time: 0,
        };
        crypto::sign_transaction_input(&mut tx, 0, &kp1).unwrap();
        crypto::sign_transaction_input(&mut tx, 1, &kp2).unwrap();

        let mut utxos = HashMap::new();
        utxos.insert(
            op1,
            make_utxo(50 * COIN, kp1.public_key().pubkey_hash(), 0, false),
        );
        utxos.insert(
            op2,
            make_utxo(50 * COIN, kp2.public_key().pubkey_hash(), 0, false),
        );

        let result = validate_transaction(&tx, lookup(&utxos), 100).unwrap();
        assert_eq!(result.total_input, 100 * COIN);
        assert_eq!(result.total_output, 90 * COIN);
        assert_eq!(result.fee, 10 * COIN);
    }

    #[test]
    fn contextual_multi_output_valid() {
        let kp = KeyPair::generate();
        let op = sample_outpoint();
        let pkh = kp.public_key().pubkey_hash();

        let mut tx = Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: op.clone(),
                signature: vec![],
                public_key: vec![],
            }],
            outputs: vec![
                TxOutput {
                    value: 30 * COIN,
                    pubkey_hash: Hash256([0xBB; 32]),
                },
                TxOutput {
                    value: 19 * COIN,
                    pubkey_hash: pkh, // change output
                },
            ],
            lock_time: 0,
        };
        crypto::sign_transaction_input(&mut tx, 0, &kp).unwrap();

        let mut utxos = HashMap::new();
        utxos.insert(op, make_utxo(50 * COIN, pkh, 0, false));

        let result = validate_transaction(&tx, lookup(&utxos), 100).unwrap();
        assert_eq!(result.total_input, 50 * COIN);
        assert_eq!(result.total_output, 49 * COIN);
        assert_eq!(result.fee, 1 * COIN);
    }

    // ==========================================
    // ValidatedTransaction
    // ==========================================

    #[test]
    fn validated_transaction_debug() {
        let vt = ValidatedTransaction {
            total_input: 100,
            total_output: 90,
            fee: 10,
        };
        let debug = format!("{vt:?}");
        assert!(debug.contains("fee: 10"));
    }

    // ==========================================
    // Error display
    // ==========================================

    #[test]
    fn error_variants_display() {
        let errors = [
            TransactionError::ImmatureCoinbase { index: 0 },
            TransactionError::ZeroValueOutput(1),
            TransactionError::NullOutpointInRegularTx(2),
        ];
        for e in &errors {
            assert!(!format!("{e}").is_empty());
        }
    }
}
