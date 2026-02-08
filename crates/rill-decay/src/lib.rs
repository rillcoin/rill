//! # rill-decay — Progressive concentration decay engine.
//!
//! All calculations use integer arithmetic only for determinism.
//!
//! This crate implements the novel progressive concentration decay mechanism:
//! - **Sigmoid-based decay rates**: cluster concentrations above the threshold
//!   trigger per-block decay, ramping from ~50% of `R_MAX` at the threshold
//!   to `R_MAX` (15%) at high concentrations.
//! - **Compound decay**: effective value is computed as `nominal * (1 - rate)^blocks`
//!   using fixed-point binary exponentiation.
//! - **UTXO lineage clustering**: outputs inherit cluster membership from inputs,
//!   with piecewise linear weakening over time.
//! - **Decay pool**: decayed amounts accumulate and release 1% per block to miners.

pub mod cluster;
pub mod engine;
pub mod sigmoid;

pub use cluster::{determine_output_cluster, lineage_adjusted_balance, lineage_factor};
pub use engine::DecayEngine;
pub use sigmoid::SIGMOID_PRECISION;
