# Blockers

Cross-agent blockers and unresolved issues. Updated at session end.

## Active Blockers
(none)

## Resolved

### VULN-COINBASE-TXID (discovered 2026-02-17, resolved 2026-02-17)
**Severity:** High — caused UTXO overwrites in storage
**Description:** Witness-stripped `txid()` excluded the coinbase signature field where block height was encoded. Coinbase transactions with the same reward+pubkey_hash produced identical txids.
**Resolution:** Coinbase transactions now set `lock_time = height`. Since `lock_time` is included in the txid computation, each coinbase at a distinct height produces a distinct txid. Updated all `make_coinbase_unique` helpers and E2E regression test.
**Commit:** `89f7eca`

### Node::process_transaction fee=0 incompatibility (discovered 2026-02-17, resolved 2026-02-17)
**Severity:** Medium — prevented RPC transaction submission
**Description:** `Node::process_transaction()` inserted mempool entries with `fee: 0`, failing MIN_TX_FEE enforcement.
**Resolution:** Node now computes actual fee (`input_sum - output_sum`) with checked arithmetic before mempool insertion.
**Commit:** `89f7eca`
