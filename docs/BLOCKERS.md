# Blockers

Cross-agent blockers and unresolved issues. Updated at session end.

## Active Blockers

### Faucet wallet unfunded (discovered 2026-02-19)
**Severity:** Medium — faucet cannot dispense RILL until funded
**Description:** Faucet wallet (`trill1qnad7...`) has zero balance. Miner wallet needs coinbase UTXOs to mature (100 blocks) before funds can be sent to faucet.
**Resolution needed:** Auto-fund script (`/root/fund_faucet.sh`) running on node0 — will send 5,000 RILL once height ≥ 101. Currently at height 73.

## Resolved

### Discord bot missing Manage Roles permission (discovered 2026-02-19, resolved 2026-02-19)
**Severity:** Low — cosmetic/community only, no protocol impact
**Description:** Bot was invited with `permissions=2048` (SEND_MESSAGES only). Creating Discord roles via API returned 403.
**Resolution:** Admin granted Manage Roles to RillBot. Roles created via API:
- Testnet Pioneer (`#22D3EE`, ID: 1474179312780447754)
- Bug Hunter (`#F97316`, ID: 1474179315137773628)

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
