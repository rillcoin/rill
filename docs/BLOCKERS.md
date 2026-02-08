# Blockers

_Cross-agent issues that need resolution_

## Current Blockers

### [Wallet] Weak KDF (VULN-15 - Informational)
- **Impact:** rill-wallet security posture
- **Description:** Currently using BLAKE3 for password-based key derivation. BLAKE3 is fast by design, making it vulnerable to brute-force attacks. An attacker with access to an encrypted wallet file could test billions of passwords per second.
- **Proposed Solution:** Replace BLAKE3 with Argon2id (memory-hard KDF) in `crates/rill-wallet/src/encryption.rs`. This is already marked with a TODO comment.
- **Status:** Discovered during security audit, acknowledged, deferred to Phase 2
- **Priority:** Medium - affects wallet security but not consensus

### [All] Missing Proptest Coverage
- **Impact:** All crates except rill-decay
- **Description:** Property-based testing with proptest only exists in rill-decay and rill-tests. Core validation logic, consensus rules, and network protocol lack proptest coverage.
- **Proposed Solution:** Add proptest coverage to:
  - rill-core: validation, block_validation, chain_state, reward, difficulty, crypto, merkle, address
  - rill-consensus: block production, chain selection
  - rill-network: message handling, peer management
  - rill-wallet: transaction building, coin selection
- **Status:** Discovered during security audit
- **Priority:** Low - nice to have, not blocking testnet

### [DevOps] GitHub Repository Access
- **Impact:** Code publication and collaboration
- **Description:** Remote repository `git@github.com:rillcoin/rill.git` is configured but not accessible. Repository may not exist yet or SSH authentication needs setup.
- **Proposed Solution:** Create GitHub repository under rillcoin organization or verify SSH keys are properly configured.
- **Status:** User will resolve tomorrow (2026-02-09)
- **Priority:** Low - local commits are safe, push can happen when ready

## Resolved

None yet.

---

**Instructions:** Add blockers when discovered. Format:
```
## [Agent] Issue Title
- **Impact:** Which agents/crates affected
- **Description:** What's blocked and why
- **Proposed Solution:** How to resolve
- **Status:** Discovered | Investigating | Resolved
```
