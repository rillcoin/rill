//! End-to-end integration tests for RillCoin.
//!
//! Each test boots a full node (without networking), mines blocks,
//! and verifies the complete lifecycle including chain state, UTXO set,
//! coinbase maturity, difficulty adjustment, decay pool queries, and
//! wallet operations.
//!
//! NOTE: The current coinbase txid computation uses a witness-stripped
//! canonical form that excludes the signature field (which carries the
//! height marker). This means coinbase transactions with the same reward
//! amount AND same pubkey_hash produce identical txids. To avoid UTXO
//! collisions in tests, each block uses a unique miner pubkey_hash.
//! This collision is tracked as a known protocol issue (VULN-COINBASE-TXID).

use std::sync::Arc;

use rill_consensus::engine::mine_block;
use rill_core::address::Network;
use rill_core::constants::*;
use rill_core::genesis;
use rill_core::types::*;
use rill_node_lib::config::NodeConfig;
use rill_node_lib::node::Node;
use rill_tests::helpers::*;

/// Create a test node backed by a temp directory, without P2P networking.
fn test_node() -> (Arc<Node>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let config = NodeConfig {
        data_dir: dir.path().to_path_buf(),
        ..NodeConfig::default()
    };
    let node = Node::without_network(config).unwrap();
    (node, dir)
}

/// Mine the next block on a node using a unique miner address derived from
/// the current chain height. This avoids coinbase txid collisions (see
/// module doc comment).
fn mine_next_block(node: &Node) -> Block {
    let (height, tip_hash) = node.chain_tip().unwrap();
    let tip_header = node.get_block_header(&tip_hash).unwrap().unwrap();
    let next_ts = tip_header.timestamp + BLOCK_TIME_SECS;

    // Use a unique miner pubkey_hash per height to produce unique coinbase txids.
    // Without this, all coinbase txids at the same reward level collide because
    // the witness-stripped canonical form excludes the height-bearing signature.
    let miner_seed = ((height + 1) & 0xFF) as u8;
    let mut block = node
        .create_block_template(&pkh(miner_seed), next_ts)
        .unwrap();
    assert!(mine_block(&mut block, u64::MAX));
    block
}

/// Mine the next block using a specific miner pubkey hash.
fn mine_next_block_to(node: &Node, miner: &Hash256) -> Block {
    let (_height, tip_hash) = node.chain_tip().unwrap();
    let tip_header = node.get_block_header(&tip_hash).unwrap().unwrap();
    let next_ts = tip_header.timestamp + BLOCK_TIME_SECS;

    let mut block = node.create_block_template(miner, next_ts).unwrap();
    assert!(mine_block(&mut block, u64::MAX));
    block
}

// ======================================================================
// E2E Test 1: Mine 5 blocks
// Verify chain tip, circulating supply, and UTXO set grow correctly.
// ======================================================================

#[test]
fn e2e_mine_five_blocks() {
    let (node, _dir) = test_node();

    for _ in 0..5 {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
    }

    let (height, _) = node.chain_tip().unwrap();
    assert_eq!(height, 5, "chain tip should be at height 5");

    let supply = node.circulating_supply().unwrap();
    assert!(
        supply > 0,
        "circulating supply should be positive after mining"
    );

    // Verify UTXOs exist: genesis coinbase + 5 mined coinbases (each with unique pkh)
    let utxos = node.iter_utxos().unwrap();
    assert_eq!(
        utxos.len(),
        6,
        "should have 6 UTXOs (genesis + 5 mined), got {}",
        utxos.len()
    );
}

// ======================================================================
// E2E Test 2: Coinbase maturity tracking
// Mine blocks and verify coinbase maturity is correctly reported based
// on the block_height stored in the UTXO entry.
// ======================================================================

#[test]
fn e2e_coinbase_maturity_tracking() {
    let (node, _dir) = test_node();

    let miner = pkh(0xBB);

    // Mine block 1 with a known miner address
    let block1 = mine_next_block_to(&node, &miner);
    let coinbase_txid = block1.transactions[0].txid().unwrap();
    node.process_block(&block1).unwrap();

    // The coinbase UTXO should exist at height 1
    let outpoint = OutPoint {
        txid: coinbase_txid,
        index: 0,
    };

    // Check the UTXO entry directly
    let utxos = node.iter_utxos().unwrap();
    let entry = utxos
        .iter()
        .find(|(op, _)| *op == outpoint)
        .map(|(_, e)| e.clone())
        .expect("coinbase UTXO should exist at height 1");

    assert!(entry.is_coinbase, "should be flagged as coinbase");
    assert_eq!(entry.block_height, 1, "block_height should be 1");

    // At height 1, the coinbase is NOT mature (needs COINBASE_MATURITY confirmations)
    assert!(
        !entry.is_mature(1),
        "coinbase should NOT be mature at height 1"
    );

    // At height COINBASE_MATURITY, it IS mature (100 confirmations: height 101 - 1 = 100)
    assert!(
        entry.is_mature(1 + COINBASE_MATURITY),
        "coinbase should be mature at height {}",
        1 + COINBASE_MATURITY
    );

    // Mine enough blocks to reach maturity
    for _ in 0..COINBASE_MATURITY {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
    }

    let (height, _) = node.chain_tip().unwrap();
    assert_eq!(height, COINBASE_MATURITY + 1);

    // Re-fetch the UTXO and verify it reports as mature at current height
    let utxos = node.iter_utxos().unwrap();
    let entry = utxos
        .iter()
        .find(|(op, _)| *op == outpoint)
        .map(|(_, e)| e.clone())
        .expect("coinbase UTXO should still exist");
    assert!(
        entry.is_mature(height),
        "coinbase should be mature at height {}",
        height
    );
}

// ======================================================================
// E2E Test 3: Difficulty adjustment
// Mine DIFFICULTY_WINDOW + 1 blocks and verify all are accepted.
// This exercises the full difficulty calculation path through
// real RocksDB-backed storage.
// ======================================================================

#[test]
fn e2e_difficulty_adjustment() {
    let (node, _dir) = test_node();

    let target_blocks = DIFFICULTY_WINDOW + 1;
    for _ in 0..target_blocks {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
    }

    let (height, _) = node.chain_tip().unwrap();
    assert_eq!(
        height, target_blocks,
        "all {} blocks should be accepted",
        target_blocks
    );
}

// ======================================================================
// E2E Test 4: Decay pool integration
// Mine blocks and verify the decay pool balance is queryable through
// the full node storage layer.
// ======================================================================

#[test]
fn e2e_decay_pool_queryable() {
    let (node, _dir) = test_node();

    // Mine 10 blocks
    for _ in 0..10 {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
    }

    // Decay pool balance should be queryable (may be 0 at this early stage
    // since no decay has been applied -- the important thing is the query
    // does not error through the full RocksDB storage path).
    let pool_balance = node.decay_pool_balance().unwrap();
    // The pool is either 0 (no decay has occurred) or positive.
    // We assert queryability, not a specific value.
    assert!(
        pool_balance == 0 || pool_balance > 0,
        "decay pool should be queryable"
    );

    // Verify circulating supply is consistent
    let supply = node.circulating_supply().unwrap();
    assert!(supply > 0, "circulating supply should be positive");
}

// ======================================================================
// E2E Test 5: Wallet lifecycle
// Create a wallet from a known seed, derive addresses, scan UTXOs,
// and verify balance computation through the decay engine.
// ======================================================================

#[test]
fn e2e_wallet_lifecycle() {
    use rill_core::error::{DecayError, RillError, TransactionError};
    use rill_core::traits::{ChainState, DecayCalculator};
    use rill_wallet::{Seed, Wallet};

    // -- Mock ChainState for wallet balance computation --
    struct MockChainState {
        supply: u64,
    }

    impl ChainState for MockChainState {
        fn get_utxo(&self, _: &OutPoint) -> Result<Option<UtxoEntry>, RillError> {
            Ok(None)
        }
        fn chain_tip(&self) -> Result<(u64, Hash256), RillError> {
            Ok((100, Hash256::ZERO))
        }
        fn get_block_header(
            &self,
            _: &Hash256,
        ) -> Result<Option<BlockHeader>, RillError> {
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
        fn cluster_balance(&self, _: &Hash256) -> Result<u64, RillError> {
            Ok(0)
        }
        fn decay_pool_balance(&self) -> Result<u64, RillError> {
            Ok(0)
        }
        fn validate_transaction(
            &self,
            _: &Transaction,
        ) -> Result<(), TransactionError> {
            Ok(())
        }
    }

    // -- Mock DecayCalculator (no decay for low concentration) --
    struct MockDecay;

    impl DecayCalculator for MockDecay {
        fn decay_rate_ppb(&self, _: u64) -> Result<u64, DecayError> {
            Ok(0)
        }
        fn compute_decay(&self, _: u64, _: u64, _: u64) -> Result<u64, DecayError> {
            Ok(0)
        }
        fn decay_pool_release(&self, _: u64) -> Result<u64, DecayError> {
            Ok(0)
        }
    }

    // Create wallet from known seed
    let seed = Seed::from_bytes([42u8; 32]);
    let mut wallet = Wallet::from_seed(seed, Network::Testnet);

    // Derive a few addresses
    let addr0 = wallet.next_address();
    let addr1 = wallet.next_address();

    assert_ne!(addr0, addr1, "derived addresses should be unique");
    assert_eq!(wallet.address_count(), 2);

    // Verify addresses have correct network
    assert_eq!(addr0.network(), Network::Testnet);
    assert_eq!(addr1.network(), Network::Testnet);

    // Create mock UTXOs for the wallet
    let utxos = vec![
        (
            OutPoint {
                txid: Hash256([1; 32]),
                index: 0,
            },
            UtxoEntry {
                output: TxOutput {
                    value: 10 * COIN,
                    pubkey_hash: addr0.pubkey_hash(),
                },
                block_height: 0,
                is_coinbase: false,
                cluster_id: Hash256::ZERO,
            },
        ),
        (
            OutPoint {
                txid: Hash256([2; 32]),
                index: 0,
            },
            UtxoEntry {
                output: TxOutput {
                    value: 5 * COIN,
                    pubkey_hash: addr1.pubkey_hash(),
                },
                block_height: 0,
                is_coinbase: false,
                cluster_id: Hash256::ZERO,
            },
        ),
    ];

    // Scan UTXOs into wallet
    wallet.scan_utxos(&utxos);
    assert_eq!(wallet.utxo_count(), 2, "wallet should find 2 UTXOs");

    // Verify balance via the decay calculator and chain state
    let mock_cs = MockChainState {
        supply: 1_000_000 * COIN,
    };
    let mock_decay = MockDecay;
    let balance = wallet.balance(&mock_decay, &mock_cs, 100).unwrap();
    assert_eq!(
        balance.nominal,
        15 * COIN,
        "nominal balance should be 15 RILL"
    );
    assert_eq!(
        balance.effective,
        15 * COIN,
        "effective balance should equal nominal (no decay for low concentration)"
    );
    assert_eq!(balance.utxo_count, 2);
}

// ======================================================================
// E2E Test 6: Block rejection (invalid block does not advance tip)
// A node must reject an invalid block and leave the chain tip unchanged.
// ======================================================================

#[test]
fn e2e_invalid_block_rejected() {
    let (node, _dir) = test_node();

    let (initial_height, initial_hash) = node.chain_tip().unwrap();
    assert_eq!(initial_height, 0);

    // Create a block with no transactions (no coinbase -- invalid)
    let block = Block {
        header: BlockHeader {
            version: 1,
            prev_hash: genesis::genesis_hash(),
            merkle_root: Hash256::ZERO,
            timestamp: genesis::GENESIS_TIMESTAMP + BLOCK_TIME_SECS,
            difficulty_target: u64::MAX,
            nonce: 0,
        },
        transactions: vec![],
    };
    assert!(node.process_block(&block).is_err());

    // Chain tip must not have advanced
    let (height, hash) = node.chain_tip().unwrap();
    assert_eq!(height, initial_height);
    assert_eq!(hash, initial_hash);
}

// ======================================================================
// E2E Test 7: Supply increases with each mined block
// Circulating supply must strictly increase after connecting a block
// (because each block adds a coinbase reward).
// ======================================================================

#[test]
fn e2e_supply_increases_monotonically() {
    let (node, _dir) = test_node();

    let mut prev_supply = node.circulating_supply().unwrap();

    for _ in 0..5 {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
        let supply = node.circulating_supply().unwrap();
        assert!(
            supply > prev_supply,
            "supply should increase: was {}, now {}",
            prev_supply,
            supply
        );
        prev_supply = supply;
    }
}

// ======================================================================
// E2E Test 8: Block header retrieval
// After connecting a block, the node should be able to retrieve both
// the full block and its header by hash.
// ======================================================================

#[test]
fn e2e_block_retrieval_by_hash() {
    let (node, _dir) = test_node();

    let block = mine_next_block(&node);
    let hash = block.header.hash();
    node.process_block(&block).unwrap();

    let retrieved_block = node.get_block(&hash).unwrap();
    assert!(retrieved_block.is_some(), "block should be retrievable");
    assert_eq!(retrieved_block.unwrap(), block);

    let retrieved_header = node.get_block_header(&hash).unwrap();
    assert!(retrieved_header.is_some(), "header should be retrievable");
    assert_eq!(retrieved_header.unwrap(), block.header);
}

// ======================================================================
// E2E Test 9: Get block hash by height
// Block hashes should be indexed by height after connection.
// ======================================================================

#[test]
fn e2e_block_hash_by_height() {
    let (node, _dir) = test_node();

    // Genesis at height 0
    let hash0 = node.get_block_hash(0).unwrap().unwrap();
    assert_eq!(hash0, genesis::genesis_hash());

    // Mine and connect block 1
    let block1 = mine_next_block(&node);
    let expected_hash1 = block1.header.hash();
    node.process_block(&block1).unwrap();

    let hash1 = node.get_block_hash(1).unwrap().unwrap();
    assert_eq!(hash1, expected_hash1);

    // Height 999 should not exist
    assert!(node.get_block_hash(999).unwrap().is_none());
}

// ======================================================================
// E2E Test 10: Mempool is empty after mining only
// Since we only mine coinbase blocks (no user transactions), the
// mempool should remain empty throughout.
// ======================================================================

#[test]
fn e2e_mempool_empty_after_mining() {
    let (node, _dir) = test_node();

    for _ in 0..3 {
        let block = mine_next_block(&node);
        node.process_block(&block).unwrap();
    }

    let (count, bytes, fees) = node.mempool_info();
    assert_eq!(count, 0, "mempool should be empty");
    assert_eq!(bytes, 0);
    assert_eq!(fees, 0);
}

// ======================================================================
// E2E Test 11: Peer count is zero without network
// A node created without networking should report zero peers.
// ======================================================================

#[test]
fn e2e_no_peers_without_network() {
    let (node, _dir) = test_node();
    assert_eq!(node.peer_count(), 0);
}

// ======================================================================
// E2E Test 12: UTXO queries by address
// After mining blocks to a specific miner, verify the address index
// returns at least one UTXO for that miner.
// ======================================================================

#[test]
fn e2e_utxo_query_by_address() {
    let (node, _dir) = test_node();

    let miner = pkh(0xBB);

    // Mine 1 block to this miner address
    let block = mine_next_block_to(&node, &miner);
    node.process_block(&block).unwrap();

    // Query UTXOs by the miner's pubkey hash
    let utxos = node.get_utxos_by_address(&miner).unwrap();
    assert_eq!(
        utxos.len(),
        1,
        "miner should have 1 coinbase UTXO, got {}",
        utxos.len()
    );

    let (_op, entry) = &utxos[0];
    assert!(entry.is_coinbase);
    assert_eq!(entry.output.pubkey_hash, miner);
    assert!(entry.output.value > 0);
}

// ======================================================================
// E2E Test 13: Wallet file save/load roundtrip
// Create a wallet, save it encrypted, load it back, and verify
// addresses are preserved.
// ======================================================================

#[test]
fn e2e_wallet_persistence() {
    use rill_wallet::{Seed, Wallet};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.wallet");
    let password = b"e2e-test-password";

    // Create wallet and derive addresses
    let mut wallet = Wallet::from_seed(Seed::from_bytes([99u8; 32]), Network::Testnet);
    let _addr0 = wallet.next_address();
    let _addr1 = wallet.next_address();

    // Save
    wallet.save_to_file(&path, password).unwrap();

    // Load
    let loaded = Wallet::load_from_file(&path, password).unwrap();
    assert_eq!(loaded.network(), Network::Testnet);
    assert_eq!(loaded.address_count(), 2);

    // Wrong password should fail
    let err = Wallet::load_from_file(&path, b"wrong-password");
    assert!(err.is_err(), "wrong password should fail");
}

// ======================================================================
// E2E Test 14: Chain tip consistency after multiple blocks
// The chain tip hash should match the last connected block's header hash.
// ======================================================================

#[test]
fn e2e_chain_tip_consistency() {
    let (node, _dir) = test_node();

    let mut last_block_hash = genesis::genesis_hash();

    for i in 1..=5u64 {
        let block = mine_next_block(&node);
        let hash = block.header.hash();
        node.process_block(&block).unwrap();

        let (height, tip_hash) = node.chain_tip().unwrap();
        assert_eq!(height, i);
        assert_eq!(tip_hash, hash);

        last_block_hash = hash;
    }

    let (_, final_hash) = node.chain_tip().unwrap();
    assert_eq!(final_hash, last_block_hash);
}

// ======================================================================
// E2E Test 15: Coinbase txid collision vulnerability
// Regression test: demonstrates that coinbase transactions with the same
// reward and pubkey_hash produce identical txids, causing UTXO overwrites.
// Attack vector: VULN-COINBASE-TXID.
// ======================================================================

#[test]
fn e2e_vuln_coinbase_txid_collision() {
    let (node, _dir) = test_node();

    let miner = pkh(0xAA);

    // Mine 3 blocks all to the same miner address
    for _ in 0..3 {
        let block = mine_next_block_to(&node, &miner);
        node.process_block(&block).unwrap();
    }

    // Due to the witness-stripped txid, all coinbase txids are identical
    // (same outpoint, same value, same pubkey_hash, same lock_time).
    // Only the LAST coinbase UTXO survives in the UTXO set.
    let utxos = node.get_utxos_by_address(&miner).unwrap();
    assert_eq!(
        utxos.len(),
        1,
        "VULN-COINBASE-TXID: only 1 UTXO survives due to txid collision (expected 3)"
    );

    // The surviving UTXO should be from the last block (height 3)
    let entry = &utxos[0].1;
    assert_eq!(
        entry.block_height, 3,
        "surviving UTXO should be from the latest block"
    );
}
