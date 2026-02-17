//! # rill-consensus — Block production, validation, and proof-of-work.
//!
//! This crate implements the [`BlockProducer`](rill_core::traits::BlockProducer)
//! trait, wiring together rill-core's validation, difficulty adjustment, and
//! reward modules with chain state and decay calculator.
//!
//! Phase 1: Mock PoW using SHA-256 double-hash.
//! Phase 2: RandomX FFI behind the same trait interface.

pub mod engine;
#[cfg(feature = "randomx")]
pub mod randomx;

pub use engine::{mine_block, ConsensusEngine};
