# Blockers

Cross-agent blockers and unresolved issues. Updated at session end.

## Active Blockers

### VULN-COINBASE-TXID (discovered 2026-02-17)
**Severity:** High — causes UTXO overwrites in storage
**Description:** Witness-stripped `txid()` excludes the coinbase signature field where block height is encoded. All coinbase transactions at the same reward level paying the same address produce identical txids, causing later coinbases to overwrite earlier ones in the UTXO set.
**Impact:** Miners reusing the same address lose prior coinbase UTXOs.
**Fix:** Commit block height to a non-witness field (e.g., `lock_time` or a dedicated `coinbase_height` field).
**Regression test:** `e2e_vuln_coinbase_txid_collision` in `crates/rill-tests/tests/e2e.rs`
**Owner:** core agent

### Node::process_transaction fee=0 incompatibility
**Severity:** Medium — prevents RPC transaction submission
**Description:** `Node::process_transaction()` in `node.rs` inserts mempool entries with `fee: 0`, which now fails with the new MIN_TX_FEE enforcement (1000 rills minimum).
**Impact:** RPC `sendrawtransaction` will fail until fee calculation is added to `process_transaction()`.
**Fix:** Compute actual fee from inputs-outputs before mempool insertion.
**Owner:** node agent

## Resolved
(none yet)
