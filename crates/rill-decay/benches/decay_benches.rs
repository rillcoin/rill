//! Criterion benchmarks for rill-decay critical operations.
//!
//! Covers: sigmoid evaluation, decay rate computation, and cluster determination.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rill_core::constants::{COIN, DECAY_C_THRESHOLD_PPB};
use rill_core::traits::DecayCalculator;
use rill_core::types::Hash256;
use rill_decay::cluster::determine_output_cluster;
use rill_decay::engine::DecayEngine;
use rill_decay::sigmoid::sigmoid_positive;

fn bench_sigmoid(c: &mut Criterion) {
    // Benchmark sigmoid at a representative mid-range input.
    // x_scaled = 2_000_000_000 corresponds to sigmoid(2.0).
    let x_scaled: u128 = 2_000_000_000;

    c.bench_function("sigmoid_calculation", |b| {
        b.iter(|| sigmoid_positive(black_box(x_scaled)))
    });
}

fn bench_decay_rate(c: &mut Criterion) {
    let engine = DecayEngine::new();
    // Concentration well above threshold to exercise the full computation path.
    let concentration = DECAY_C_THRESHOLD_PPB + 5_000_000;

    c.bench_function("decay_rate_calculation", |b| {
        b.iter(|| engine.decay_rate_ppb(black_box(concentration)))
    });
}

fn bench_compute_decay(c: &mut Criterion) {
    let engine = DecayEngine::new();
    let value = 1000 * COIN;
    let concentration = DECAY_C_THRESHOLD_PPB + 5_000_000;
    let blocks = 1000;

    c.bench_function("compute_decay", |b| {
        b.iter(|| {
            engine.compute_decay(
                black_box(value),
                black_box(concentration),
                black_box(blocks),
            )
        })
    });
}

fn bench_determine_cluster(c: &mut Criterion) {
    let c1 = Hash256([0x11; 32]);
    let c2 = Hash256([0x22; 32]);
    let c3 = Hash256([0x33; 32]);
    let txid = Hash256([0xCC; 32]);
    let inputs = vec![c1, c2, c3];

    c.bench_function("determine_cluster", |b| {
        b.iter(|| determine_output_cluster(black_box(&inputs), black_box(&txid)))
    });
}

criterion_group!(
    benches,
    bench_sigmoid,
    bench_decay_rate,
    bench_compute_decay,
    bench_determine_cluster,
);
criterion_main!(benches);
