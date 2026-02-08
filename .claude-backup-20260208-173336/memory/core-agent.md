# core Agent — Session Notes
## Status: In Progress
## Last Session: 2026-02-06

## Completed
- **types module** (`crates/rill-core/src/types.rs`): All core protocol types
  - Hash256, OutPoint, TxInput, TxOutput, Transaction, BlockHeader, Block, UtxoEntry
  - 31 unit tests

- **traits module** (`crates/rill-core/src/traits.rs`): 4 trait interfaces
  - ChainState, DecayCalculator, BlockProducer, NetworkService
  - All Send + Sync, dyn-compatible, with default method impls
  - 33 unit tests with mock implementations

- **crypto module** (`crates/rill-core/src/crypto.rs`): Ed25519 key types and tx signing
  - KeyPair, PublicKey, pubkey_hash(), signing_hash(), sign/verify_transaction_input()
  - CryptoError in error.rs
  - 32 unit tests

- **merkle module** (`crates/rill-core/src/merkle.rs`): BLAKE3 Merkle tree
  - Domain-separated hashing: leaf = BLAKE3(0x00 || data), node = BLAKE3(0x01 || left || right)
  - `merkle_root(leaves)` — efficient standalone root computation
  - `MerkleTree::from_leaves(leaves)` — full tree with proof generation
  - `MerkleProof` with `verify(root)` — inclusion proofs for SPV
  - Side enum (Left/Right), ProofStep struct
  - Odd layers padded by duplicating last element; empty → ZERO
  - All proof types derive Serialize, Deserialize, bincode::Encode/Decode
  - 32 unit tests covering: domain separation, root computation (empty/1/2/3/4/odd/even),
    proof generation for all leaves (sizes 1-5 and 33), proof depth, verification failures
    (wrong root, tampered leaf/sibling, cross-tree), bincode roundtrip, duplication behavior
  - Total: 128 tests across rill-core, 0 failures

- **genesis module** (`crates/rill-core/src/genesis.rs`): Genesis block (height 0)
  - LazyLock-cached genesis data (block, hash, coinbase_txid)
  - Constants: GENESIS_TIMESTAMP (Jan 1, 2026), GENESIS_MESSAGE, DEV_FUND_PREMINE (5% of MAX_SUPPLY)
  - dev_fund_pubkey_hash() → BLAKE3("rill genesis dev fund") — deterministic placeholder
  - genesis_block(), genesis_hash(), genesis_coinbase_txid(), is_genesis()
  - Single coinbase tx paying DEV_FUND_PREMINE to dev fund address
  - Header: version 1, prev_hash ZERO, difficulty u64::MAX, nonce 0
  - 25 unit tests covering: constants, block structure, header fields, merkle root,
    hash determinism, txid matching, is_genesis true/false/modified, dev fund pubkey hash
  - Total: 153 tests across rill-core, 0 failures

- **address module** (`crates/rill-core/src/address.rs`): Bech32m address encoding
  - Network enum: Mainnet (HRP "rill"), Testnet (HRP "trill")
  - Address struct: network + version (0) + 32-byte BLAKE3 pubkey hash
  - Full Bech32m implementation: polymod, HRP expansion, checksum create/verify, bit conversion
  - `Address::from_pubkey_hash(hash, network)`, `from_public_key(pk, network)`
  - `Address::encode()` → `rill1...` (64 chars mainnet, 65 testnet)
  - `Address::decode(s)` → Result with full validation (case, charset, checksum, version, length)
  - Display/FromStr, Serialize/Deserialize (as bech32m string)
  - AddressError in error.rs: InvalidHrp, InvalidLength, InvalidChecksum, InvalidCharacter,
    InvalidVersion, InvalidPadding, UnknownNetwork, MissingSeparator, MixedCase
  - 40 unit tests covering: network HRP, encoding (prefix/case/determinism/length/network diff),
    decoding (roundtrip/uppercase/mixed case/bad checksum/bad char/missing sep/short/unknown),
    roundtrips (pubkey/zero/max/many hashes), accessors, Display/FromStr, serde JSON,
    bech32m internals (bit conversion, checksum verify/tamper/wrong HRP)
  - Total: 193 tests across rill-core, 0 failures

- **validation module** (`crates/rill-core/src/validation.rs`): Transaction validation
  - Two-level validation: structural (context-free) + contextual (UTXO-aware)
  - `validate_transaction_structure(tx)` — common + coinbase/regular-specific checks
    - Non-empty inputs/outputs, no zero-value outputs, no overflow, size limit
    - Coinbase: single null-outpoint input, data size ≤ MAX_COINBASE_DATA (100 bytes)
    - Regular: no null outpoints, no duplicates, 64-byte sig + 32-byte pubkey per input
  - `validate_transaction(tx, get_utxo, current_height)` — full contextual validation
    - Rejects coinbase (validated in block validation instead)
    - UTXO existence, coinbase maturity, Ed25519 signature verification, value conservation
    - Returns `ValidatedTransaction { total_input, total_output, fee }`
  - Generic `get_utxo: Fn(&OutPoint) -> Option<UtxoEntry>` for flexible UTXO source
  - New error variants: ImmatureCoinbase, ZeroValueOutput, NullOutpointInRegularTx
  - New constant: MAX_COINBASE_DATA = 100
  - 31 unit tests covering: structural (empty in/out, zero-value, overflow, coinbase structure,
    regular structure, null outpoints, duplicates, sig/pubkey lengths), contextual (valid tx,
    fee calculation, zero fee, unknown UTXO, insufficient funds, immature/mature coinbase,
    invalid signature, tampered output, coinbase rejection, multi-input, multi-output),
    error display, ValidatedTransaction debug
  - Total: 224 tests across rill-core, 0 failures

- **block_validation module** (`crates/rill-core/src/block_validation.rs`): Block validation
  - Two-level validation: structural (context-free) + contextual (chain-state-aware)
  - `validate_block_structure(block)` — context-free checks
    - Non-empty block, first tx is coinbase, no other coinbase
    - No duplicate txids, merkle root matches, block size ≤ MAX_BLOCK_SIZE
    - PoW: header hash satisfies claimed difficulty (check_pow)
    - All transactions pass structural validation
  - `validate_block(block, context, get_utxo)` — full contextual validation
    - Header linkage: prev_hash matches expected parent
    - Difficulty: header target matches expected difficulty
    - Timestamp: strictly after parent, not too far in future (MAX_FUTURE_BLOCK_TIME)
    - All non-coinbase txs: contextual validation + cross-tx double-spend detection
    - Coinbase reward: output value ≤ block_reward + total_fees
    - Returns `ValidatedBlock { total_fees, coinbase_value }`
  - `BlockContext` struct: height, prev_hash, prev_timestamp, expected_difficulty,
    current_time, block_reward
  - `check_pow(block)` — LE u64 of first 8 hash bytes ≤ difficulty_target
  - New BlockError variants: FirstTxNotCoinbase, MultipleCoinbase, DuplicateTxid,
    DoubleSpend, InvalidDifficulty, TimestampNotAfterParent
  - New constant: MAX_FUTURE_BLOCK_TIME = 2 * BLOCK_TIME_SECS (120s)
  - Genesis block not validated here (use genesis::is_genesis instead)
  - 32 unit tests covering: structural (empty/no-coinbase/multi-coinbase/merkle/PoW/
    bad-tx-structure/valid), contextual (prev_hash/difficulty/timestamp-monotonic/
    timestamp-future/exact-reward/partial-reward/excess-reward/fees-in-reward/
    unknown-utxo/invalid-sig/double-spend/valid-with-txs/coinbase-only),
    types (debug), error display
  - Total: 256 tests across rill-core, 0 failures

- **reward module** (`crates/rill-core/src/reward.rs`): Reward schedule & halving logic
  - `block_reward(height)` — base mining reward at any height
  - `epoch_reward(epoch)` — reward for a halving epoch (INITIAL_REWARD >> epoch)
  - `halving_epoch(height)` — which epoch a height belongs to
  - `epoch_start_height(epoch)` — first height of an epoch
  - `next_halving_height(height)` / `blocks_until_halving(height)` — halving countdown
  - `cumulative_reward(height)` — total schedule rewards through height (O(epochs))
  - `total_mining_supply()` — sum across all 33 epochs ≈ MAX_SUPPLY - 2,310,000 rills
  - `last_reward_epoch()` → 32, `last_reward_height()` → 6,929,999
  - Shift-overflow guard: epoch ≥ 64 → 0 (Rust would panic on u64 >> 64)
  - 55 unit tests covering: reward at boundaries/halvings/zero/large, epoch reward
    (decreasing/terminal), epoch arithmetic, next halving/blocks until,
    cumulative (at 0/1/epoch boundaries/past all rewards/monotonicity),
    total supply (positive/close to MAX/deterministic/epoch-by-epoch),
    last epoch/height, consistency checks, premine relationship
  - Total: 311 tests across rill-core, 0 failures

- **difficulty module** (`crates/rill-core/src/difficulty.rs`): Difficulty adjustment algorithm
  - Rolling window adjustment — recalculates every block (not epoch-based like Bitcoin)
  - `next_target(timestamps, current_target)` — core algorithm from timestamp window
    - Computes actual vs expected time, adjusts proportionally
    - Uses u128 intermediate to prevent overflow
    - Clamps per-window change to MAX_ADJUSTMENT_FACTOR (4×)
    - Result clamped to [MIN_TARGET (1), MAX_TARGET (u64::MAX)]
  - `target_for_height(height, parent_target, get_timestamp)` — convenience wrapper
    - Returns MAX_TARGET for heights 0 and 1 (insufficient data)
    - Height ≥ 2: uses min(height, DIFFICULTY_WINDOW+1) timestamps
    - Growing window during early chain, full DIFFICULTY_WINDOW intervals at steady state
  - `expected_window_time()` → 3600s, `full_window_size()` → 61 timestamps
  - Constants: MAX_ADJUSTMENT_FACTOR=4, MIN_TARGET=1, MAX_TARGET=u64::MAX
  - 45 unit tests covering: edge cases (empty/single timestamp), on-target timing
    (full window/small window), slow blocks (2x/3x increase), fast blocks (2x/3x decrease),
    clamping (4x max/quarter min/exact boundaries), bounds (min/max target), partial windows
    (2/10 timestamps), proportional adjustment, non-uniform spacing, target_for_height
    (heights 0/1/2/growing/full/past boundary/slow/fast), constants, convergence,
    stability (repeated on-target), oscillation dampening, u128 overflow, integer truncation
  - Total: 356 tests across rill-core, 0 failures

- **mempool module** (`crates/rill-core/src/mempool.rs`): Transaction pool
  - `MempoolEntry` struct: tx, txid, fee, size, fee_rate (milli-rills/byte)
  - `Mempool` struct: HashMap entries, HashMap outpoint index, BTreeSet fee-rate index
  - `new(max_count, max_bytes)`, `with_defaults()` — DEFAULT_MAX_COUNT=5000, DEFAULT_MAX_BYTES=5MiB
  - `insert(tx, fee)` — validates not duplicate, checks conflicts, evicts lowest-fee-rate if full
  - `remove(txid)`, `contains(txid)`, `get(txid)` — O(1) lookup
  - `has_conflict(tx)`, `conflicting_txids(tx)` — O(1) conflict detection via outpoint index
  - `select_transactions(max_block_bytes)` — greedy highest-fee-rate-first for block template
  - `remove_confirmed_block(block)` — removes confirmed txs + any conflicting pool txs
  - Accessors: `len()`, `is_empty()`, `total_bytes()`, `total_fees()`, `iter()`, `txids()`
  - MempoolError in error.rs: AlreadyExists, Conflict, PoolFull, Internal
  - Single bincode serialization for both txid and size computation
  - Fee rate: fee * 1000 / size with u128 intermediate to prevent overflow
  - Not thread-safe (caller wraps in Mutex/RwLock as needed)
  - 43 unit tests covering: new/defaults, insert (basic/duplicate/conflict), remove (exists/missing),
    get/contains, has_conflict/conflicting_txids, select_transactions (empty/single/multi/size limit/
    fee-rate ordering), remove_confirmed_block (confirmed/conflicts/unrelated), eviction (pool full/
    low fee rejected/high fee evicts), capacity (count limit/byte limit), total_fees, iter/txids,
    entry fields, error display
  - Total: 399 tests across rill-core, 0 failures

- **chain_state module** (`crates/rill-core/src/chain_state.rs`): Chain state storage interface
  - `ConnectBlockResult` struct: utxos_created, utxos_spent
  - `DisconnectBlockResult` struct: utxos_restored, utxos_removed
  - `BlockUndo` (internal): stores spent UTXOs for reorg reversion
  - `ChainStore` trait (Send + Sync, dyn-compatible):
    - `connect_block(block, height)` — applies block: spends inputs, creates outputs, stores undo data
    - `disconnect_tip()` — reverts tip block using undo data, restores spent UTXOs
    - `get_utxo(outpoint)`, `contains_utxo(outpoint)` (default impl) — UTXO lookups
    - `chain_tip()` — (height, hash), returns (0, ZERO) for empty chain
    - `get_block(hash)`, `get_block_header(hash)`, `get_block_hash(height)` — block lookups
    - `utxo_count()`, `is_empty()` — state queries
  - `MemoryChainStore` struct: in-memory implementation for testing
    - HashMap-backed: utxos, blocks, headers, height_to_hash, undo_data
    - `new()`, `Default`, `block_count()`, `undo_count()` — construction and accessors
    - Height validation: first block must be 0, subsequent must be tip_height + 1
    - Duplicate block detection via block hash
    - Blocks persist after disconnect (for reorg/history queries)
  - ChainStateError in error.rs: EmptyChain, BlockNotFound, UndoDataMissing, HeightMismatch, DuplicateBlock
  - 44 unit tests covering: empty store (new/default/tip/utxo/block/header/hash), connect genesis
    (basic/utxos/stores/wrong height), multiple blocks (two/spending tx/wrong height/duplicate),
    multi-output (coinbase/regular), disconnect (empty error/genesis/restores utxos/height mapping/
    undo removal), roundtrip (3 blocks/reconnect), lookups (block/header/hash), UTXO queries
    (contains/fields/regular not coinbase), persist after disconnect, dyn compatible, result types
    (debug/eq/clone), error display/eq, edge cases (10 coinbase blocks/disconnect all/spending chain/
    multi-input)
  - Total: 443 tests across rill-core, 0 failures

## Design Decisions
- Transaction::txid() returns Result (no panics in library code)
- BlockHeader::hash() uses explicit manual byte layout, infallible
- All numeric fields u64; bincode::Encode/Decode alongside serde derives
- Coinbase marker: OutPoint::null() = (ZERO txid, u64::MAX index)
- Traits sync-only, Send + Sync, dyn-compatible
- KeyPair non-serializable (security); pubkey_hash uses BLAKE3
- Sighash: version + all outpoints + all outputs + locktime + input_index (excludes sigs)
- Merkle tree: domain-separated BLAKE3 (0x00 leaf, 0x01 node) prevents 2nd-preimage
- Merkle odd handling: duplicate last element (Bitcoin-style, safe with domain separation)
- merkle_root() standalone for efficiency; MerkleTree for proofs
- Genesis: LazyLock for one-time computation, all accessors return refs/copies
- Genesis dev fund pubkey hash: BLAKE3("rill genesis dev fund") — deterministic, transparent
- DEV_FUND_PREMINE = MAX_SUPPLY / BPS_PRECISION * DEV_FUND_BPS (division first avoids overflow)
- Address: Bech32m (not base58check) — modern, error-detecting, "rill1..." natural from HRP
- Address version byte = 0 (extensible for future address types)
- Address encodes 32-byte BLAKE3 pubkey hash (consistent with TxOutput::pubkey_hash)
- Serde serializes Address as bech32m string (clean JSON), not raw fields
- Validation: two-level (structural + contextual), generic UTXO lookup via Fn closure
- Coinbase txs excluded from contextual validation (reward checked in block validation)
- validate_transaction returns ValidatedTransaction with fee for block template use
- Block validation: PoW = LE u64 of first 8 hash bytes ≤ difficulty_target (Phase 1 mock)
- Block validation uses BlockContext struct for all chain-state parameters
- Timestamp: strictly increasing (> parent), max drift = 2 * BLOCK_TIME_SECS from wall clock
- Coinbase reward: must be ≤ base_reward + total_fees (underpaying is allowed)
- No intra-block spending: all inputs must reference pre-block UTXOs
- Reward: pure functions (not methods on a trait), standalone module separate from BlockProducer
- Halving schedule: INITIAL_REWARD >> epoch, 33 non-zero epochs (0–32), epoch 32 pays 1 rill/block
- total_mining_supply ≈ MAX_SUPPLY (off by 2,310,000 rills ≈ 0.0231 RILL due to truncation)
- Mining supply + DEV_FUND_PREMINE > MAX_SUPPLY: MAX_SUPPLY is the mining cap, premine is additional
- cumulative_reward: O(epochs) using epoch structure, not O(height)
- Difficulty: rolling per-block adjustment (not epoch-based) for faster hashrate response
- Difficulty window: DIFFICULTY_WINDOW intervals = DIFFICULTY_WINDOW+1 timestamps at steady state
- Early chain (height < DIFFICULTY_WINDOW+1): growing window using all available blocks
- Heights 0 and 1 always get MAX_TARGET (genesis difficulty)
- Adjustment clamped to 4× per window; total elapsed time matters, not individual intervals
- u128 intermediate for target * actual_time / expected_time to prevent u64 overflow
- Mempool: not thread-safe (caller wraps in Mutex); simplifies internal logic
- Mempool: BTreeSet<(u64, Hash256)> for fee-rate ordering; Hash256 breaks ties deterministically
- Mempool: single bincode serialization for both txid computation and size measurement
- Mempool: eviction of lowest-fee-rate entry when pool full, only if new tx has higher fee rate
- Mempool: O(1) conflict detection via HashMap<OutPoint, Hash256> outpoint index
- Mempool: fee_rate = fee * 1000 / size (milli-rills per byte) with u128 intermediate
- ChainStore: separate from ChainState trait — ChainState is read-only for consumers, ChainStore is mutable storage
- ChainStore: assumes blocks are pre-validated; only checks height consistency and duplicate blocks
- ChainStore: undo data stored per-block for disconnect_tip (reorg support)
- ChainStore: blocks/headers persist after disconnect (available for reorg queries and history)
- ChainStore: not thread-safe (caller wraps in Mutex/RwLock)
- ChainStore: empty chain = (0, ZERO) tip; genesis at height 0 changes hash to non-ZERO
- ChainStore: cluster_id = ZERO for all UTXOs in Phase 1 (no clustering yet)
- ChainStore: coinbase inputs skipped in spend_inputs (no real UTXOs to remove)

## Fixes Applied
- Created missing rill-decay bench stub
- Fixed libp2p feature: kademlia → kad

## What's Next
1. rill-core Phase 1 complete — all foundation modules implemented
2. Next: rill-decay (progressive decay algorithm, clustering) or rill-consensus (RandomX PoW)
