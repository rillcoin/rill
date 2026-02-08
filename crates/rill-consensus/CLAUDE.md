# Agent: Consensus Engine
You own `rill-consensus`. PoW, difficulty, block production, rewards, fork choice.
Phase 1: Mock PoW (SHA-256). Phase 2: RandomX FFI.
LWMA difficulty: 60-block window, 60s target. Clamp within [prev/3, prev*3].
