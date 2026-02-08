# wallet Agent — Session Notes
## Status: Complete (Phase 1)
## Last Session: 2026-02-07

## Implementation Summary
rill-wallet crate fully implemented with 76 unit tests, all passing.

### Modules
1. **error.rs** — WalletError enum with 14 variants, From conversions for CryptoError/TransactionError/DecayError
2. **keys.rs** — Seed (Zeroize/ZeroizeOnDrop), KeyChain with BLAKE3 KDF derivation, KeyChainData serializable form
3. **coin_selection.rs** — Decay-aware greedy coin selection: highest-decay UTXOs first, u128 intermediate for concentration calc
4. **encryption.rs** — AES-256-GCM with BLAKE3 password KDF (Phase 1; TODO: argon2id for production)
5. **builder.rs** — TransactionBuilder with builder pattern, coin selection, input signing
6. **wallet.rs** — Wallet composition: create, from_seed, scan_utxos, balance, send, save/load encrypted files

### Key Design Decisions
- BLAKE3 KDF context: `rill-wallet-key-derivation-v1` for child keys, `rill-wallet-password-kdf-v1` for password
- u128 intermediate math for concentration calculation (avoids u64 overflow with large cluster balances * CONCENTRATION_PRECISION)
- Wallet file format: `header_len(4 LE) || header_json(magic+version) || encrypted_payload(salt+nonce+ciphertext+tag)`
- DEFAULT_BASE_FEE = 1000 rills, DEFAULT_FEE_PER_INPUT = 500 rills
- WALLET_MAGIC = b"RIWL", WALLET_VERSION = 1

### Dependencies
- Removed: rill-network, bs58 (not needed)
- Added: blake3, bincode, hex (workspace)

## What's Next
- bins/rill-cli: Wire up wallet operations (create, address, send) via CLI
- Phase 2: Upgrade password KDF from BLAKE3 to argon2id
- Phase 2: Dynamic fee estimation
- Phase 2: Bloom filter / electrum-style UTXO indexing
