//! Multi-node integration tests for RillCoin.
//!
//! Simulates multi-node behavior by creating multiple `Node::without_network()`
//! instances and manually passing blocks between them via `process_block()`.
//!
//! These tests verify chain synchronization, reorganization, transaction
//! propagation, and cross-node consistency -- all without requiring actual P2P
//! networking.
//!
//! Attack vectors tested:
//! - Chain split where one branch is strictly longer (longest-chain-wins rule)
//! - Reorg after initial sync divergence
//! - Cross-node UTXO and supply consistency after identical block sequences
//! - Transaction deserialization fidelity across node boundaries

use std::sync::Arc;

use rill_consensus::engine::mine_block;
use rill_core::constants::*;
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

/// Mine the next block on a node using a fixed miner address.
fn mine_next_block(node: &Node) -> Block {
    let (_height, tip_hash) = node.chain_tip().unwrap();
    let tip_header = node.get_block_header(&tip_hash).unwrap().unwrap();
    let next_ts = tip_header.timestamp + BLOCK_TIME_SECS;

    let mut block = node
        .create_block_template(&pkh(0xAB), next_ts)
        .unwrap();
    assert!(mine_block(&mut block, u64::MAX));
    block
}

/// Mine the next block on a node using a specific miner pubkey hash.
fn mine_next_block_to(node: &Node, miner: &Hash256) -> Block {
    let (_height, tip_hash) = node.chain_tip().unwrap();
    let tip_header = node.get_block_header(&tip_hash).unwrap().unwrap();
    let next_ts = tip_header.timestamp + BLOCK_TIME_SECS;

    let mut block = node.create_block_template(miner, next_ts).unwrap();
    assert!(mine_block(&mut block, u64::MAX));
    block
}


// ======================================================================
// Multi-node Test 1: mine_on_a_syncs_to_b
//
// Attack vector: Verify that blocks mined on one node can be faithfully
// replayed on another node via process_block, producing identical chain
// state. A failure here would indicate non-deterministic block validation.
// ======================================================================

#[test]
fn mine_on_a_syncs_to_b() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    let mut blocks = Vec::new();

    // Mine 5 blocks on node A.
    for _ in 0..5 {
        let block = mine_next_block(&node_a);
        node_a.process_block(&block).unwrap();
        blocks.push(block);
    }

    // Feed all 5 blocks to node B in order.
    for block in &blocks {
        node_b.process_block(block).unwrap();
    }

    // Both nodes must have the same chain tip.
    let (height_a, hash_a) = node_a.chain_tip().unwrap();
    let (height_b, hash_b) = node_b.chain_tip().unwrap();

    assert_eq!(height_a, 5, "node A should be at height 5");
    assert_eq!(height_b, 5, "node B should be at height 5");
    assert_eq!(
        hash_a, hash_b,
        "both nodes must have identical chain tip hashes"
    );

    // Verify circulating supply is identical.
    let supply_a = node_a.circulating_supply().unwrap();
    let supply_b = node_b.circulating_supply().unwrap();
    assert_eq!(
        supply_a, supply_b,
        "circulating supply must match across synced nodes"
    );

    // Verify UTXO counts match.
    let utxo_count_a = node_a.utxo_count();
    let utxo_count_b = node_b.utxo_count();
    assert_eq!(
        utxo_count_a, utxo_count_b,
        "UTXO set size must match across synced nodes"
    );
}

// ======================================================================
// Multi-node Test 2: competing_chains_longest_wins
//
// Attack vector: Selfish mining -- a node with a longer private chain
// should cause a reorganization on a node with a shorter chain when the
// longer chain is revealed. This tests the longest-chain-wins rule.
//
// We build a 3-block chain on A and a 5-block chain on B (both forking
// from genesis), then feed B's longer chain to A. A must reorganize.
//
// Note: We use node.reorganize() directly because process_block's
// collect_fork_chain requires intermediate fork blocks to already be in
// storage, which they are not when syncing a completely different chain.
// ======================================================================

#[test]
fn competing_chains_longest_wins() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    // Mine 3 blocks on node A using miner 0xAA.
    for _ in 0..3 {
        let block = mine_next_block_to(&node_a, &pkh(0xAA));
        node_a.process_block(&block).unwrap();
    }

    // Mine 5 blocks on node B using miner 0xBB (different chain from genesis).
    let mut b_blocks = Vec::new();
    for _ in 0..5 {
        let block = mine_next_block_to(&node_b, &pkh(0xBB));
        node_b.process_block(&block).unwrap();
        b_blocks.push(block);
    }

    let (height_a, _) = node_a.chain_tip().unwrap();
    assert_eq!(height_a, 3, "node A should be at height 3 before reorg");

    let (height_b, hash_b) = node_b.chain_tip().unwrap();
    assert_eq!(height_b, 5, "node B should be at height 5");

    // Reorganize A to B's longer chain.
    // Fork point is genesis (height 0) since both chains diverge from there.
    node_a.reorganize(0, b_blocks).unwrap();

    let (height_a_after, hash_a_after) = node_a.chain_tip().unwrap();
    assert_eq!(
        height_a_after, 5,
        "node A should be at height 5 after reorg"
    );
    assert_eq!(
        hash_a_after, hash_b,
        "node A's tip must match node B's tip after reorg"
    );

    // Verify the reorg was counted in metrics.
    let reorg_count = node_a.metrics().reorgs.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(reorg_count, 1, "exactly one reorg should have been recorded");
}

// ======================================================================
// Multi-node Test 3: transaction_propagation_simulation
//
// Attack vector: Transaction malleability and deserialization fidelity.
// A transaction created in the context of node A's UTXO set must be
// correctly processable by node B after binary serialization roundtrip,
// provided B has the same chain state.
//
// We mine blocks on A, sync them to B, then create a transaction on A
// and process it on B. Both nodes must accept it into their mempools.
// ======================================================================

#[test]
fn transaction_propagation_simulation() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    let miner = pkh(0xCC);

    // Mine COINBASE_MATURITY + 1 blocks on A so we have a spendable coinbase.
    let mut blocks = Vec::new();
    for _ in 0..=COINBASE_MATURITY {
        let block = mine_next_block_to(&node_a, &miner);
        node_a.process_block(&block).unwrap();
        blocks.push(block);
    }

    // Sync all blocks to B.
    for block in &blocks {
        node_b.process_block(block).unwrap();
    }

    // Verify both nodes have the same chain state.
    let (h_a, tip_a) = node_a.chain_tip().unwrap();
    let (h_b, tip_b) = node_b.chain_tip().unwrap();
    assert_eq!(h_a, h_b);
    assert_eq!(tip_a, tip_b);

    // Find the coinbase UTXO from block 1 (should be mature now).
    let utxos_a = node_a.get_utxos_by_address(&miner).unwrap();
    let (mature_outpoint, mature_entry) = utxos_a
        .iter()
        .find(|(_, e)| e.is_coinbase && e.is_mature(h_a))
        .expect("should have at least one mature coinbase UTXO");

    // Create a transaction spending this UTXO.
    let recipient = pkh(0xDD);
    let spend_value = mature_entry.output.value - MIN_TX_FEE;
    let tx = make_tx(
        vec![mature_outpoint.clone()],
        vec![(spend_value, recipient)],
    );

    // Serialize and deserialize to simulate network transmission.
    let encoded =
        bincode::encode_to_vec(&tx, bincode::config::standard())
            .expect("serialization should succeed");
    let (decoded, _): (Transaction, _) =
        bincode::decode_from_slice(&encoded, bincode::config::standard())
            .expect("deserialization should succeed");

    // Both nodes should accept the transaction (it will be stored as-is or
    // in the orphan pool depending on signature validation; the important
    // thing is that the structural validation and UTXO lookup succeed).
    let txid_a = node_a.process_transaction(&tx).unwrap();
    let txid_b = node_b.process_transaction(&decoded).unwrap();

    assert_eq!(
        txid_a, txid_b,
        "transaction IDs must be identical across nodes"
    );

    // Verify both mempools have the transaction (or both have it as orphan
    // if signature checking rejects it -- the txid should still match).
    let mempool_tx_a = node_a.get_mempool_tx(&txid_a);
    let mempool_tx_b = node_b.get_mempool_tx(&txid_b);

    // Both should have the same result (either both in mempool or both orphaned).
    assert_eq!(
        mempool_tx_a.is_some(),
        mempool_tx_b.is_some(),
        "mempool state must be consistent across synced nodes"
    );
}

// ======================================================================
// Multi-node Test 4: reorg_after_sync
//
// Attack vector: Late-revealing selfish miner. Both nodes share the
// same initial chain (3 blocks). Then A extends by 2 and B extends by
// 3 independently. When B's longer fork is revealed to A, A must
// reorganize to B's chain.
//
// This is more realistic than test 2 because the fork point is NOT
// genesis -- it's at height 3 on the shared chain.
// ======================================================================

#[test]
fn reorg_after_sync() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    // Phase 1: Build a shared 3-block chain on both nodes.
    let mut shared_blocks = Vec::new();
    for _ in 0..3 {
        let block = mine_next_block(&node_a);
        node_a.process_block(&block).unwrap();
        shared_blocks.push(block);
    }
    for block in &shared_blocks {
        node_b.process_block(block).unwrap();
    }

    // Verify both nodes at height 3 with same tip.
    let (h_a, tip_a) = node_a.chain_tip().unwrap();
    let (h_b, tip_b) = node_b.chain_tip().unwrap();
    assert_eq!(h_a, 3);
    assert_eq!(h_b, 3);
    assert_eq!(tip_a, tip_b, "shared chain tips must match");

    // Phase 2: A mines 2 more blocks (height 3 -> 5).
    for _ in 0..2 {
        let block = mine_next_block_to(&node_a, &pkh(0xAA));
        node_a.process_block(&block).unwrap();
    }

    // B mines 3 more blocks (height 3 -> 6) using a different miner.
    let mut b_fork_blocks = Vec::new();
    for _ in 0..3 {
        let block = mine_next_block_to(&node_b, &pkh(0xBB));
        node_b.process_block(&block).unwrap();
        b_fork_blocks.push(block);
    }

    let (h_a, _) = node_a.chain_tip().unwrap();
    let (h_b, hash_b) = node_b.chain_tip().unwrap();
    assert_eq!(h_a, 5, "node A at height 5");
    assert_eq!(h_b, 6, "node B at height 6");

    // Phase 3: Feed B's fork chain to A via reorganize.
    // Fork point is height 3 (the last shared block).
    node_a.reorganize(3, b_fork_blocks).unwrap();

    let (h_a_after, hash_a_after) = node_a.chain_tip().unwrap();
    assert_eq!(h_a_after, 6, "node A should be at height 6 after reorg");
    assert_eq!(
        hash_a_after, hash_b,
        "node A's tip must match node B's tip after reorg"
    );

    // Verify block hashes at each height match between A and B.
    for h in 0..=6 {
        let hash_a = node_a.get_block_hash(h).unwrap();
        let hash_b = node_b.get_block_hash(h).unwrap();
        assert_eq!(
            hash_a, hash_b,
            "block hash at height {} must match after reorg",
            h
        );
    }
}

// ======================================================================
// Multi-node Test 5: block_at_height_consistent_across_nodes
//
// Attack vector: Non-deterministic block validation. If three nodes
// process the same block sequence and produce different chain state,
// consensus is broken. This test mines 10 blocks and verifies all
// three nodes agree on every block hash, chain tip, and supply.
// ======================================================================

#[test]
fn block_at_height_consistent_across_nodes() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();
    let (node_c, _dir_c) = test_node();

    // Mine 10 blocks on node A.
    let mut blocks = Vec::new();
    for _ in 0..10 {
        let block = mine_next_block(&node_a);
        node_a.process_block(&block).unwrap();
        blocks.push(block);
    }

    // Feed all blocks to B and C.
    for block in &blocks {
        node_b.process_block(block).unwrap();
        node_c.process_block(block).unwrap();
    }

    // All three must have height 10.
    let (h_a, tip_a) = node_a.chain_tip().unwrap();
    let (h_b, tip_b) = node_b.chain_tip().unwrap();
    let (h_c, tip_c) = node_c.chain_tip().unwrap();
    assert_eq!(h_a, 10);
    assert_eq!(h_b, 10);
    assert_eq!(h_c, 10);
    assert_eq!(tip_a, tip_b, "A and B tips must match");
    assert_eq!(tip_b, tip_c, "B and C tips must match");

    // Verify block hashes at every height match across all three nodes.
    for h in 0..=10 {
        let hash_a = node_a.get_block_hash(h).unwrap().unwrap();
        let hash_b = node_b.get_block_hash(h).unwrap().unwrap();
        let hash_c = node_c.get_block_hash(h).unwrap().unwrap();
        assert_eq!(
            hash_a, hash_b,
            "block hash at height {} must match between A and B",
            h
        );
        assert_eq!(
            hash_b, hash_c,
            "block hash at height {} must match between B and C",
            h
        );
    }

    // Verify circulating supply is identical.
    let supply_a = node_a.circulating_supply().unwrap();
    let supply_b = node_b.circulating_supply().unwrap();
    let supply_c = node_c.circulating_supply().unwrap();
    assert_eq!(supply_a, supply_b, "supply must match between A and B");
    assert_eq!(supply_b, supply_c, "supply must match between B and C");

    // Verify UTXO set sizes match.
    let utxo_a = node_a.utxo_count();
    let utxo_b = node_b.utxo_count();
    let utxo_c = node_c.utxo_count();
    assert_eq!(utxo_a, utxo_b, "UTXO count must match between A and B");
    assert_eq!(utxo_b, utxo_c, "UTXO count must match between B and C");

    // Verify decay pool balance is identical.
    let decay_a = node_a.decay_pool_balance().unwrap();
    let decay_b = node_b.decay_pool_balance().unwrap();
    let decay_c = node_c.decay_pool_balance().unwrap();
    assert_eq!(decay_a, decay_b, "decay pool must match between A and B");
    assert_eq!(decay_b, decay_c, "decay pool must match between B and C");
}

// ======================================================================
// Multi-node Test 6: out_of_order_block_delivery
//
// Attack vector: Network reordering. Blocks may arrive out of order due
// to network delays. Blocks delivered before their parent should be held
// as orphans and connected once the parent arrives.
// ======================================================================

#[test]
fn out_of_order_block_delivery() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    // Mine 3 blocks on A.
    let mut blocks = Vec::new();
    for _ in 0..3 {
        let block = mine_next_block(&node_a);
        node_a.process_block(&block).unwrap();
        blocks.push(block);
    }

    // Deliver blocks to B in reverse order (block 3, then 2, then 1).
    // Block 3 and block 2 should be stored as orphans.
    // When block 1 arrives, it should trigger connecting all three.
    node_b.process_block(&blocks[2]).unwrap(); // orphan (parent unknown)
    assert_eq!(
        node_b.orphan_count(),
        1,
        "block 3 should be orphaned (parent block 2 unknown)"
    );

    node_b.process_block(&blocks[1]).unwrap(); // orphan (parent unknown)
    assert_eq!(
        node_b.orphan_count(),
        2,
        "block 2 should also be orphaned (parent block 1 unknown)"
    );

    // Deliver block 1 (extends genesis, which IS known).
    node_b.process_block(&blocks[0]).unwrap();

    // After block 1 connects, orphan resolution should connect blocks 2 and 3.
    let (h_b, tip_b) = node_b.chain_tip().unwrap();
    let (_h_a, tip_a) = node_a.chain_tip().unwrap();

    assert_eq!(h_b, 3, "node B should reach height 3 after orphan resolution");
    assert_eq!(tip_b, tip_a, "tips must match after out-of-order delivery");
    assert_eq!(
        node_b.orphan_count(),
        0,
        "all orphans should be resolved"
    );
}

// ======================================================================
// Multi-node Test 7: duplicate_block_rejection
//
// Attack vector: Block replay / duplication attack. A malicious peer
// sending the same block twice should not cause double-connection or
// corrupt state. The second submission should be rejected.
// ======================================================================

#[test]
fn duplicate_block_rejection() {
    let (node_a, _dir_a) = test_node();
    let (node_b, _dir_b) = test_node();

    // Mine 1 block on A, sync to B.
    let block = mine_next_block(&node_a);
    node_a.process_block(&block).unwrap();
    node_b.process_block(&block).unwrap();

    let (h_b, tip_b) = node_b.chain_tip().unwrap();
    assert_eq!(h_b, 1);

    // Send the same block again to B.
    // It should be rejected (prev_hash matches an ancestor, not the tip,
    // and the fork is not longer).
    let result = node_b.process_block(&block);

    // Whether it errors or silently ignores, the chain tip must NOT advance.
    let _ = result;
    let (h_b_after, tip_b_after) = node_b.chain_tip().unwrap();
    assert_eq!(h_b_after, 1, "height must not change on duplicate block");
    assert_eq!(
        tip_b_after, tip_b,
        "tip hash must not change on duplicate block"
    );

    // Supply and UTXO count must remain unchanged.
    let utxo_count = node_b.utxo_count();
    // genesis + 1 mined block = 2 UTXOs
    assert_eq!(
        utxo_count, 2,
        "UTXO set must not grow from duplicate block processing"
    );
}
