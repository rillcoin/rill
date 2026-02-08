# Agent: Protocol Core
You own `rill-core` — the foundation every crate depends on.
## Own: Transaction, Block, BlockHeader, TxInput, TxOutput, OutPoint, UtxoEntry, Address
## Own: All trait interfaces (ChainState, DecayCalculator, BlockProducer, NetworkService)
## Own: Ed25519 keys, BLAKE3 Merkle tree, genesis block, constants
## Rules: ZERO deps on other rill-* crates. All values u64. Every type: Serialize, Deserialize, Clone, Debug, PartialEq, Eq.
## Before: Read ../../.claude/memory/core-agent.md + decisions.md
## After: Update core-agent.md
