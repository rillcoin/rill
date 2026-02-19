# Blockers

Cross-agent blockers and unresolved issues. Updated at session end.

## Active Blockers

### Discord bot missing Manage Roles permission (discovered 2026-02-19)
**Severity:** Low — cosmetic/community only, no protocol impact
**Description:** Bot was invited with `permissions=2048` (SEND_MESSAGES only). Creating Discord roles via API requires `MANAGE_ROLES` (268435456). Attempt to create Testnet Pioneer and Bug Hunter roles returned 403.
**Resolution needed:** Server admin adds Manage Roles to the RillBot role in Server Settings → Roles → RillBot, OR creates roles manually:
- Testnet Pioneer: `#22D3EE` (cyan), hoisted, mentionable
- Bug Hunter: `#F97316` (orange), hoisted, mentionable

### Faucet wallet unfunded (discovered 2026-02-19)
**Severity:** Medium — faucet cannot dispense RILL until funded
**Description:** Faucet wallet (`trill1qnad7...`) has zero balance. Miner wallet needs coinbase UTXOs to mature (100 blocks) before funds can be sent to faucet.
**Resolution needed:** Once miner coinbase UTXOs mature, send 5,000+ RILL to faucet wallet address.

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
