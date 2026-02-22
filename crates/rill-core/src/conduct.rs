//! Proof of Conduct scoring engine.
//!
//! Pure computation — no storage, no IO. All integer-only math.
//! Values are in rills (1 RILL = 10^8 rills) where amounts appear.
//! Conduct scores are dimensionless integers in the range `0–1000`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum epochs tracked in the velocity baseline rolling window.
pub const VELOCITY_BASELINE_MAX_EPOCHS: usize = 90;

/// Minimum epochs of history required before velocity anomaly detection fires.
pub const VELOCITY_BASELINE_MIN_EPOCHS: usize = 10;

/// Exponential-smoothing weight applied to the existing (old) conduct score,
/// expressed as a percentage out of 100.
pub const SMOOTH_OLD_WEIGHT: u16 = 85;

/// Exponential-smoothing weight applied to the incoming raw score,
/// expressed as a percentage out of 100.
pub const SMOOTH_NEW_WEIGHT: u16 = 15;

/// Number of standard deviations above the mean that triggers an anomaly.
pub const ANOMALY_STDDEV_THRESHOLD: u64 = 3;

// ---------------------------------------------------------------------------
// Score-to-multiplier mapping
// ---------------------------------------------------------------------------

/// Map a conduct score (0–1000) to a decay multiplier expressed in basis
/// points (BPS), where 10 000 BPS = 1.0×.
///
/// | Score Range | BPS    | Multiplier |
/// |-------------|--------|------------|
/// | 900–1000    | 5,000  | 0.5×       |
/// | 750–899     | 7,500  | 0.75×      |
/// | 600–749     | 10,000 | 1.0×       |
/// | 500–599     | 15,000 | 1.5×       |
/// | 350–499     | 20,000 | 2.0×       |
/// | 200–349     | 25,000 | 2.5×       |
/// | 0–199       | 30,000 | 3.0×       |
///
/// # Examples
///
/// ```
/// use rill_core::conduct::score_to_multiplier_bps;
///
/// assert_eq!(score_to_multiplier_bps(1000), 5_000);
/// assert_eq!(score_to_multiplier_bps(900),  5_000);
/// assert_eq!(score_to_multiplier_bps(899),  7_500);
/// assert_eq!(score_to_multiplier_bps(600),  10_000);
/// assert_eq!(score_to_multiplier_bps(500),  15_000);
/// assert_eq!(score_to_multiplier_bps(0),    30_000);
/// ```
pub fn score_to_multiplier_bps(score: u16) -> u64 {
    match score {
        900..=1000 => 5_000,
        750..=899 => 7_500,
        600..=749 => 10_000,
        500..=599 => 15_000,
        350..=499 => 20_000,
        200..=349 => 25_000,
        // 0..=199
        _ => 30_000,
    }
}

// ---------------------------------------------------------------------------
// VelocityBaseline
// ---------------------------------------------------------------------------

/// Rolling velocity statistics used for Undertow detection.
///
/// Tracks outbound transaction volumes across the last
/// [`VELOCITY_BASELINE_MAX_EPOCHS`] epochs (90 maximum). All arithmetic uses
/// integer-only math; the standard deviation is approximated via an integer
/// square root of the variance.
///
/// # Invariants
///
/// * `epoch_volumes.len() <= VELOCITY_BASELINE_MAX_EPOCHS`
/// * `sum` equals `epoch_volumes.iter().sum::<u64>()`
/// * `sum_squares` equals `epoch_volumes.iter().map(|v| (*v as u128).pow(2)).sum::<u128>()`
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct VelocityBaseline {
    /// Outbound transaction volumes per epoch (last 90 epochs maximum).
    pub epoch_volumes: Vec<u64>,
    /// Running sum of all epoch volumes (used for mean calculation).
    pub sum: u64,
    /// Running sum of squared volumes (used for variance calculation).
    /// Stored as `u128` to prevent overflow when volumes are large.
    pub sum_squares: u128,
}

impl VelocityBaseline {
    /// Create an empty baseline with no history.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let b = VelocityBaseline::new();
    /// assert_eq!(b.epoch_count(), 0);
    /// assert_eq!(b.mean(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            epoch_volumes: Vec::new(),
            sum: 0,
            sum_squares: 0,
        }
    }

    /// Record a new epoch volume, maintaining a rolling window of at most
    /// [`VELOCITY_BASELINE_MAX_EPOCHS`] entries.
    ///
    /// When the window is full the oldest entry is evicted and its contribution
    /// is subtracted from the running sums before the new entry is added.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// b.push_epoch(100);
    /// b.push_epoch(200);
    /// assert_eq!(b.epoch_count(), 2);
    /// assert_eq!(b.mean(), 150);
    /// ```
    pub fn push_epoch(&mut self, volume: u64) {
        // Evict the oldest entry if the window is full.
        if self.epoch_volumes.len() == VELOCITY_BASELINE_MAX_EPOCHS {
            let oldest = self.epoch_volumes.remove(0);
            self.sum = self.sum.saturating_sub(oldest);
            let oldest_sq = (oldest as u128).saturating_mul(oldest as u128);
            self.sum_squares = self.sum_squares.saturating_sub(oldest_sq);
        }

        self.epoch_volumes.push(volume);
        self.sum = self.sum.saturating_add(volume);
        self.sum_squares = self
            .sum_squares
            .saturating_add((volume as u128).saturating_mul(volume as u128));
    }

    /// Number of epochs currently tracked in the rolling window.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// assert_eq!(b.epoch_count(), 0);
    /// b.push_epoch(50);
    /// assert_eq!(b.epoch_count(), 1);
    /// ```
    pub fn epoch_count(&self) -> usize {
        self.epoch_volumes.len()
    }

    /// Arithmetic mean of the epoch volumes.
    ///
    /// Returns `0` when the baseline is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// assert_eq!(b.mean(), 0);
    /// b.push_epoch(100);
    /// b.push_epoch(300);
    /// assert_eq!(b.mean(), 200);
    /// ```
    pub fn mean(&self) -> u64 {
        let count = self.epoch_volumes.len() as u64;
        if count == 0 {
            return 0;
        }
        self.sum / count
    }

    /// Population variance of the epoch volumes.
    ///
    /// Uses `u128` intermediates to avoid overflow. Returns `0` when the
    /// baseline is empty.
    ///
    /// `variance = (sum_squares / count) - (mean * mean)`
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// // Two values: 0 and 200 — mean = 100, variance = (0^2 + 200^2)/2 - 100^2
    /// //           = (0 + 40000)/2 - 10000 = 20000 - 10000 = 10000.
    /// b.push_epoch(0);
    /// b.push_epoch(200);
    /// assert_eq!(b.variance(), 10_000);
    /// ```
    pub fn variance(&self) -> u64 {
        let count = self.epoch_volumes.len() as u128;
        if count == 0 {
            return 0;
        }
        let mean_u128 = self.sum as u128 / count;
        let mean_sq = mean_u128.saturating_mul(mean_u128);
        let sq_mean = self.sum_squares / count;
        // Variance is non-negative by construction, but guard against integer
        // truncation making sq_mean < mean_sq due to floor division.
        let variance_u128 = sq_mean.saturating_sub(mean_sq);
        // Clamp to u64::MAX rather than panic.
        variance_u128.min(u64::MAX as u128) as u64
    }

    /// Approximate integer square root of the variance.
    ///
    /// Uses Newton's method starting from a reasonable initial guess.
    /// The result satisfies `result^2 <= variance < (result+1)^2`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// b.push_epoch(0);
    /// b.push_epoch(200);
    /// // variance = 10_000, stddev = 100
    /// assert_eq!(b.stddev_approx(), 100);
    /// ```
    pub fn stddev_approx(&self) -> u64 {
        isqrt(self.variance())
    }

    /// Returns `true` if `current_volume` is anomalously high relative to the
    /// historical baseline.
    ///
    /// Anomaly condition: `current_volume > mean + ANOMALY_STDDEV_THRESHOLD * stddev`
    /// AND the baseline has at least [`VELOCITY_BASELINE_MIN_EPOCHS`] entries.
    ///
    /// When insufficient history is available this always returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rill_core::conduct::VelocityBaseline;
    ///
    /// let mut b = VelocityBaseline::new();
    /// // Fill 10 epochs with a constant volume of 100.
    /// for _ in 0..10 {
    ///     b.push_epoch(100);
    /// }
    /// // Perfectly normal volume.
    /// assert!(!b.is_anomalous(100));
    /// // A volume 5× the mean and well above mean + 3σ (stddev is 0 here so
    /// // anything strictly above the mean triggers it).
    /// assert!(b.is_anomalous(101));
    /// ```
    pub fn is_anomalous(&self, current_volume: u64) -> bool {
        if self.epoch_count() < VELOCITY_BASELINE_MIN_EPOCHS {
            return false;
        }
        let mean = self.mean();
        let stddev = self.stddev_approx();
        let threshold = mean.saturating_add(
            ANOMALY_STDDEV_THRESHOLD.saturating_mul(stddev),
        );
        current_volume > threshold
    }
}

impl Default for VelocityBaseline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Signal scoring functions
// ---------------------------------------------------------------------------

/// Wallet age score: logarithmic credit for wallet longevity.
///
/// `score = min(1000, ilog2(age_in_epochs + 1) * 150)`
///
/// Uses an integer base-2 logarithm (position of the highest set bit) as a
/// fast approximation. A wallet that has never seen an epoch (`age = 0`)
/// returns 0.
///
/// | Age (epochs) | Approx score |
/// |--------------|-------------|
/// | 0            | 0           |
/// | 1            | 150         |
/// | 7            | 450         |
/// | 100          | 1000 (cap)  |
///
/// # Examples
///
/// ```
/// use rill_core::conduct::wallet_age_score;
///
/// assert_eq!(wallet_age_score(0), 0);
/// // log2(1 + 1) = 1 → 1 * 150 = 150
/// assert_eq!(wallet_age_score(1), 150);
/// // High ages are capped at 1000.
/// assert_eq!(wallet_age_score(10_000), 1000);
/// ```
pub fn wallet_age_score(age_in_epochs: u64) -> u16 {
    let arg = age_in_epochs.saturating_add(1);
    // ilog2(arg) = floor(log2(arg)) = position of the highest set bit.
    let log2 = u64::BITS - arg.leading_zeros() - 1; // equivalent to ilog2
    let raw = (log2 as u64).saturating_mul(150);
    raw.min(1000) as u16
}

/// Contract fulfilment rate score.
///
/// Returns a value in `0–1000` based on the ratio of fulfilled contracts to
/// total contracts. A perfect fulfilment rate yields 1000; zero contracts
/// fulfilled from a non-zero total yields 0. With no history at all, returns
/// 500 (neutral).
///
/// # Examples
///
/// ```
/// use rill_core::conduct::contract_fulfilment_score;
///
/// assert_eq!(contract_fulfilment_score(0, 0), 500);   // no contracts → neutral
/// assert_eq!(contract_fulfilment_score(10, 10), 1000); // perfect → max
/// assert_eq!(contract_fulfilment_score(5, 10), 500);   // 50% → neutral
/// assert_eq!(contract_fulfilment_score(0, 10), 0);     // none fulfilled → min
/// ```
pub fn contract_fulfilment_score(fulfilled: u64, total: u64) -> u16 {
    if total == 0 {
        return 500; // Neutral if no contracts.
    }
    let score = fulfilled.saturating_mul(1000) / total;
    score.min(1000) as u16
}

/// Dispute rate score.
///
/// Returns a value in `0–1000` inversely proportional to the dispute rate.
/// No disputes yields 1000; all contracts disputed yields 0. With no
/// contract history, returns 500 (neutral).
///
/// # Examples
///
/// ```
/// use rill_core::conduct::dispute_rate_score;
///
/// assert_eq!(dispute_rate_score(0, 0), 500);    // no history → neutral
/// assert_eq!(dispute_rate_score(0, 10), 1000);  // no disputes → max
/// assert_eq!(dispute_rate_score(10, 10), 0);    // all disputed → min
/// assert_eq!(dispute_rate_score(5, 10), 500);   // 50% dispute rate → neutral
/// ```
pub fn dispute_rate_score(disputed: u64, total: u64) -> u16 {
    if total == 0 {
        return 500; // Neutral if no contracts.
    }
    let dispute_ratio = disputed.saturating_mul(1000) / total;
    1000u16.saturating_sub(dispute_ratio as u16)
}

/// Peer review aggregate score.
///
/// Maps the average peer review score (1–10 scale) onto the `0–1000`
/// conduct score range. An average of 10 yields 1000; an average of 1
/// yields 0. With no reviews, returns 500 (neutral).
///
/// Formula: `(avg_score - 1) * 1000 / 9`, where `avg_score` ∈ `[1, 10]`.
///
/// # Examples
///
/// ```
/// use rill_core::conduct::peer_review_score;
///
/// assert_eq!(peer_review_score(0, 0), 500);          // no reviews → neutral
/// assert_eq!(peer_review_score(100, 10), 1000);       // avg 10 → max
/// assert_eq!(peer_review_score(10, 10), 0);           // avg 1 → min
/// let mid = peer_review_score(55, 10);                // avg 5.5 → ~500
/// assert!(mid >= 490 && mid <= 510, "expected ~500, got {mid}");
/// ```
pub fn peer_review_score(review_sum: u64, review_count: u64) -> u16 {
    if review_count == 0 {
        return 500; // Neutral if no reviews.
    }
    // avg * 100 to preserve fractional part in integer arithmetic.
    let avg_x100 = review_sum.saturating_mul(100) / review_count;
    // Map from [100, 1000] → [0, 1000]: subtract min (100), scale by 1000/900.
    let score = avg_x100.saturating_sub(100).saturating_mul(1000) / 900;
    score.min(1000) as u16
}

/// Velocity anomaly score: higher is better (less anomalous).
///
/// * If the baseline has fewer than [`VELOCITY_BASELINE_MIN_EPOCHS`] epochs,
///   returns `500` (neutral — not enough data).
/// * If `current_volume` is anomalous (> mean + 3σ), returns `0`.
/// * Otherwise returns a value in `[1, 1000]` that falls as the volume deviates
///   further above the mean:
///   `score = 1000 * (mean + 3*stddev - excess) / (mean + 3*stddev + 1)`
///   clamped to `[0, 1000]`.
///
/// # Examples
///
/// ```
/// use rill_core::conduct::{ VelocityBaseline, velocity_anomaly_score };
///
/// // No baseline yet → neutral 500.
/// let empty = VelocityBaseline::new();
/// assert_eq!(velocity_anomaly_score(&empty, 9999), 500);
///
/// // Stable baseline (mean=100, stddev=0), volume below the mean → maximum score.
/// let mut b = VelocityBaseline::new();
/// for _ in 0..10 { b.push_epoch(100); }
/// assert_eq!(velocity_anomaly_score(&b, 0), 1000);
///
/// // Volume strictly above mean + 3σ → 0.
/// assert_eq!(velocity_anomaly_score(&b, 10_000), 0);
/// ```
pub fn velocity_anomaly_score(baseline: &VelocityBaseline, current_volume: u64) -> u16 {
    if baseline.epoch_count() < VELOCITY_BASELINE_MIN_EPOCHS {
        return 500;
    }

    let mean = baseline.mean();
    let stddev = baseline.stddev_approx();
    let upper = mean.saturating_add(ANOMALY_STDDEV_THRESHOLD.saturating_mul(stddev));

    if current_volume > upper {
        return 0;
    }

    // Scale score inversely with how much of the allowed range is consumed.
    // When current_volume == 0 and upper == 0 we return full score.
    if upper == 0 {
        return 1000;
    }

    // score = 1000 * (upper - current_volume) / upper, plus 1 so it is
    // never 0 for non-anomalous volumes (anomalous is handled above).
    let numerator = (upper - current_volume) as u128 * 1000;
    let score = (numerator / upper as u128).min(1000) as u16;
    // Ensure non-anomalous volumes always score at least 1.
    score.max(1)
}

// ---------------------------------------------------------------------------
// Composite score computation
// ---------------------------------------------------------------------------

/// Compute the raw conduct score from individual signal scores.
///
/// Weights (signals not yet implemented use a neutral default of 500):
///
/// | Signal              | Weight |
/// |---------------------|--------|
/// | contract_fulfilment | 30%    |
/// | dispute_rate        | 25%    |
/// | velocity_anomaly    | 20%    |
/// | peer_review         | 15%    |
/// | wallet_age          | 10%    |
///
/// All weights sum to 100. Returns a value in `0–1000`.
///
/// # Examples
///
/// ```
/// use rill_core::conduct::compute_raw_score;
///
/// // All neutral inputs → 500.
/// assert_eq!(compute_raw_score(500, 500, 500, 500, 500), 500);
///
/// // All perfect inputs → 1000.
/// assert_eq!(compute_raw_score(1000, 1000, 1000, 1000, 1000), 1000);
///
/// // All terrible inputs → 0.
/// assert_eq!(compute_raw_score(0, 0, 0, 0, 0), 0);
/// ```
pub fn compute_raw_score(
    contract_fulfilment: u16, // 0–1000, default 500
    dispute_rate: u16,        // 0–1000, default 500
    velocity_anomaly: u16,    // 0–1000
    peer_review: u16,         // 0–1000, default 500
    wallet_age: u16,          // 0–1000
) -> u16 {
    // Use u64 intermediates to avoid overflow (max value: 1000 * 30 = 30_000).
    let weighted_sum: u64 = (contract_fulfilment as u64) * 30
        + (dispute_rate as u64) * 25
        + (velocity_anomaly as u64) * 20
        + (peer_review as u64) * 15
        + (wallet_age as u64) * 10;

    // Divide by total weight (100) and clamp.
    let score = weighted_sum / 100;
    score.min(1000) as u16
}

/// Apply exponential smoothing to carry forward a conduct score.
///
/// `new_score = (old_score * 85 + raw_score * 15) / 100`
///
/// Uses integer division and clamps the result to `0–1000`.
///
/// # Examples
///
/// ```
/// use rill_core::conduct::smooth_score;
///
/// // No change when both inputs are equal.
/// assert_eq!(smooth_score(500, 500), 500);
///
/// // Moves slowly toward the raw score.
/// let s = smooth_score(500, 1000);
/// assert!(s > 500 && s <= 1000);
///
/// // Result is clamped.
/// assert_eq!(smooth_score(1000, 1000), 1000);
/// assert_eq!(smooth_score(0, 0), 0);
/// ```
pub fn smooth_score(old_score: u16, raw_score: u16) -> u16 {
    let blended = (old_score as u32) * (SMOOTH_OLD_WEIGHT as u32)
        + (raw_score as u32) * (SMOOTH_NEW_WEIGHT as u32);
    let result = blended / 100;
    result.min(1000) as u16
}

// ---------------------------------------------------------------------------
// Internal helper: integer square root (Newton's method)
// ---------------------------------------------------------------------------

/// Compute `floor(sqrt(n))` using Newton's method.
///
/// The initial guess is set high enough that Newton's method (which converges
/// from above for this application) is guaranteed to find the floor. The loop
/// terminates when the estimate stops decreasing, at which point the previous
/// value is the floor.
///
/// Returns `0` for `n == 0`.
fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    // Start with an overestimate: 2^ceil(bits/2).
    // For n with `b` bits, sqrt(n) < 2^(b/2 + 1), so this is always >= sqrt(n).
    let bits = 64 - n.leading_zeros(); // number of bits in n
    let mut x = 1u64 << bits.div_ceil(2); // initial overestimate

    // Newton iterations converge monotonically downward from an overestimate.
    loop {
        let x_next = (x + n / x) / 2;
        if x_next >= x {
            // x is already floor(sqrt(n)).
            return x;
        }
        x = x_next;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- contract_fulfilment_score ---

    #[test]
    fn contract_fulfilment_score_no_contracts() {
        assert_eq!(contract_fulfilment_score(0, 0), 500);
    }

    #[test]
    fn contract_fulfilment_score_perfect() {
        assert_eq!(contract_fulfilment_score(10, 10), 1000);
    }

    #[test]
    fn contract_fulfilment_score_half() {
        assert_eq!(contract_fulfilment_score(5, 10), 500);
    }

    #[test]
    fn contract_fulfilment_score_none_fulfilled() {
        assert_eq!(contract_fulfilment_score(0, 10), 0);
    }

    #[test]
    fn contract_fulfilment_score_never_exceeds_1000() {
        // fulfilled > total would be a protocol bug, but saturate safely.
        assert!(contract_fulfilment_score(20, 10) <= 1000);
    }

    // --- dispute_rate_score ---

    #[test]
    fn dispute_rate_score_no_contracts() {
        assert_eq!(dispute_rate_score(0, 0), 500);
    }

    #[test]
    fn dispute_rate_score_no_disputes() {
        assert_eq!(dispute_rate_score(0, 10), 1000);
    }

    #[test]
    fn dispute_rate_score_all_disputed() {
        assert_eq!(dispute_rate_score(10, 10), 0);
    }

    #[test]
    fn dispute_rate_score_half_disputed() {
        assert_eq!(dispute_rate_score(5, 10), 500);
    }

    // --- peer_review_score ---

    #[test]
    fn peer_review_score_no_reviews() {
        assert_eq!(peer_review_score(0, 0), 500);
    }

    #[test]
    fn peer_review_score_perfect_ten() {
        // 10 reviews all scoring 10 → sum = 100, avg = 10, score = 1000.
        assert_eq!(peer_review_score(100, 10), 1000);
    }

    #[test]
    fn peer_review_score_minimum_one() {
        // 10 reviews all scoring 1 → sum = 10, avg = 1, score = 0.
        assert_eq!(peer_review_score(10, 10), 0);
    }

    #[test]
    fn peer_review_score_midpoint() {
        // 10 reviews with avg 5.5 → sum = 55, expected ≈ 500.
        let mid = peer_review_score(55, 10);
        assert!(mid >= 490 && mid <= 510, "expected ~500 got {mid}");
    }

    #[test]
    fn peer_review_score_single_review() {
        // 1 review scoring 5 → avg = 5, score = (5-1)*1000/9 = 444.
        let s = peer_review_score(5, 1);
        assert_eq!(s, 444);
    }

    // --- score_to_multiplier_bps ---

    #[test]
    fn multiplier_bps_top_bracket() {
        assert_eq!(score_to_multiplier_bps(1000), 5_000);
        assert_eq!(score_to_multiplier_bps(900), 5_000);
    }

    #[test]
    fn multiplier_bps_second_bracket() {
        assert_eq!(score_to_multiplier_bps(899), 7_500);
        assert_eq!(score_to_multiplier_bps(750), 7_500);
    }

    #[test]
    fn multiplier_bps_third_bracket() {
        assert_eq!(score_to_multiplier_bps(749), 10_000);
        assert_eq!(score_to_multiplier_bps(600), 10_000);
    }

    #[test]
    fn multiplier_bps_fourth_bracket() {
        assert_eq!(score_to_multiplier_bps(599), 15_000);
        assert_eq!(score_to_multiplier_bps(500), 15_000);
    }

    #[test]
    fn multiplier_bps_fifth_bracket() {
        assert_eq!(score_to_multiplier_bps(499), 20_000);
        assert_eq!(score_to_multiplier_bps(350), 20_000);
    }

    #[test]
    fn multiplier_bps_sixth_bracket() {
        assert_eq!(score_to_multiplier_bps(349), 25_000);
        assert_eq!(score_to_multiplier_bps(200), 25_000);
    }

    #[test]
    fn multiplier_bps_bottom_bracket() {
        assert_eq!(score_to_multiplier_bps(199), 30_000);
        assert_eq!(score_to_multiplier_bps(0), 30_000);
    }

    #[test]
    fn multiplier_bps_boundaries_are_correct() {
        // Verify every boundary transitions to the right bracket.
        let cases: &[(u16, u64)] = &[
            (900, 5_000),
            (899, 7_500),
            (750, 7_500),
            (749, 10_000),
            (600, 10_000),
            (599, 15_000),
            (500, 15_000),
            (499, 20_000),
            (350, 20_000),
            (349, 25_000),
            (200, 25_000),
            (199, 30_000),
        ];
        for &(score, expected_bps) in cases {
            assert_eq!(
                score_to_multiplier_bps(score),
                expected_bps,
                "score={score}"
            );
        }
    }

    // --- VelocityBaseline::push_epoch / rolling window ---

    #[test]
    fn push_epoch_accumulates_correctly() {
        let mut b = VelocityBaseline::new();
        b.push_epoch(100);
        b.push_epoch(200);
        b.push_epoch(300);
        assert_eq!(b.epoch_count(), 3);
        assert_eq!(b.sum, 600);
        assert_eq!(b.mean(), 200);
    }

    #[test]
    fn push_epoch_rolling_window_evicts_oldest() {
        let mut b = VelocityBaseline::new();
        // Fill the window completely.
        for i in 0..VELOCITY_BASELINE_MAX_EPOCHS {
            b.push_epoch(i as u64);
        }
        assert_eq!(b.epoch_count(), VELOCITY_BASELINE_MAX_EPOCHS);

        // Push one more — the oldest (0) should be evicted.
        b.push_epoch(999);
        assert_eq!(b.epoch_count(), VELOCITY_BASELINE_MAX_EPOCHS);
        assert_eq!(b.epoch_volumes[0], 1); // formerly index 1
        assert_eq!(*b.epoch_volumes.last().unwrap(), 999);

        // Running sum must be consistent with the stored volumes.
        let expected_sum: u64 = b.epoch_volumes.iter().sum();
        assert_eq!(b.sum, expected_sum);
    }

    #[test]
    fn push_epoch_running_sum_squares_consistent() {
        let mut b = VelocityBaseline::new();
        for v in [10u64, 20, 30, 40, 50] {
            b.push_epoch(v);
        }
        let expected: u128 = b
            .epoch_volumes
            .iter()
            .map(|&v| (v as u128) * (v as u128))
            .sum();
        assert_eq!(b.sum_squares, expected);
    }

    #[test]
    fn push_epoch_eviction_keeps_sum_squares_consistent() {
        let mut b = VelocityBaseline::new();
        for i in 0..VELOCITY_BASELINE_MAX_EPOCHS {
            b.push_epoch((i as u64) * 10);
        }
        b.push_epoch(12345);

        let expected_sq: u128 = b
            .epoch_volumes
            .iter()
            .map(|&v| (v as u128) * (v as u128))
            .sum();
        assert_eq!(b.sum_squares, expected_sq);
    }

    // --- mean / variance / stddev ---

    #[test]
    fn mean_empty_is_zero() {
        assert_eq!(VelocityBaseline::new().mean(), 0);
    }

    #[test]
    fn variance_empty_is_zero() {
        assert_eq!(VelocityBaseline::new().variance(), 0);
    }

    #[test]
    fn variance_uniform_is_zero() {
        let mut b = VelocityBaseline::new();
        for _ in 0..10 {
            b.push_epoch(500);
        }
        assert_eq!(b.variance(), 0);
        assert_eq!(b.stddev_approx(), 0);
    }

    #[test]
    fn variance_known_values() {
        // volumes: 0, 200 → mean=100, var=(0+40000)/2 - 10000 = 10000
        let mut b = VelocityBaseline::new();
        b.push_epoch(0);
        b.push_epoch(200);
        assert_eq!(b.mean(), 100);
        assert_eq!(b.variance(), 10_000);
        assert_eq!(b.stddev_approx(), 100);
    }

    #[test]
    fn stddev_perfect_square() {
        // volumes: 0, 0, 200, 200 → mean=100
        // sum_sq = 2*(0^2) + 2*(200^2) = 80000
        // sq_mean = 80000/4 = 20000
        // mean_sq = 100^2 = 10000
        // variance = 10000, stddev = 100
        let mut b = VelocityBaseline::new();
        for _ in 0..2 {
            b.push_epoch(0);
            b.push_epoch(200);
        }
        assert_eq!(b.variance(), 10_000);
        assert_eq!(b.stddev_approx(), 100);
    }

    // --- is_anomalous ---

    #[test]
    fn is_anomalous_insufficient_data_returns_false() {
        let mut b = VelocityBaseline::new();
        for _ in 0..9 {
            b.push_epoch(100);
        }
        // Only 9 epochs — below the minimum.
        assert!(!b.is_anomalous(9_999_999));
    }

    #[test]
    fn is_anomalous_normal_volume_returns_false() {
        let mut b = VelocityBaseline::new();
        for _ in 0..10 {
            b.push_epoch(100);
        }
        // Exactly at the mean — not anomalous.
        assert!(!b.is_anomalous(100));
    }

    #[test]
    fn is_anomalous_high_volume_returns_true() {
        let mut b = VelocityBaseline::new();
        // Mean = 100, stddev = 0 (all identical).
        // Threshold = 100 + 3*0 = 100. Anything > 100 is anomalous.
        for _ in 0..10 {
            b.push_epoch(100);
        }
        assert!(b.is_anomalous(101));
        assert!(b.is_anomalous(10_000));
    }

    #[test]
    fn is_anomalous_exactly_at_threshold_is_not_anomalous() {
        let mut b = VelocityBaseline::new();
        for _ in 0..10 {
            b.push_epoch(100);
        }
        // stddev == 0, threshold == 100. Volume == 100 is NOT > threshold.
        assert!(!b.is_anomalous(100));
    }

    #[test]
    fn is_anomalous_with_nonzero_stddev() {
        let mut b = VelocityBaseline::new();
        // Add 10 epochs: 5 × 0, 5 × 200 → mean=100, variance=10000, stddev=100.
        for _ in 0..5 {
            b.push_epoch(0);
            b.push_epoch(200);
        }
        // threshold = 100 + 3*100 = 400.
        assert!(!b.is_anomalous(400)); // exactly at threshold, not above
        assert!(b.is_anomalous(401));
    }

    // --- wallet_age_score ---

    #[test]
    fn wallet_age_score_zero() {
        // log2(0 + 1) = log2(1) = 0 → score = 0
        assert_eq!(wallet_age_score(0), 0);
    }

    #[test]
    fn wallet_age_score_one() {
        // log2(1 + 1) = log2(2) = 1 → score = 150
        assert_eq!(wallet_age_score(1), 150);
    }

    #[test]
    fn wallet_age_score_medium() {
        // log2(100 + 1) = 6 → score = 900
        let s = wallet_age_score(100);
        assert!(s > 0 && s <= 1000);
    }

    #[test]
    fn wallet_age_score_cap() {
        assert_eq!(wallet_age_score(10_000), 1000);
        assert_eq!(wallet_age_score(u64::MAX), 1000);
    }

    #[test]
    fn wallet_age_score_monotone() {
        let ages = [0u64, 1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 10_000];
        let mut prev = 0u16;
        for &age in &ages {
            let s = wallet_age_score(age);
            assert!(s >= prev, "score should be non-decreasing: age={age}, s={s}, prev={prev}");
            prev = s;
        }
    }

    // --- velocity_anomaly_score ---

    #[test]
    fn velocity_anomaly_score_no_baseline() {
        let b = VelocityBaseline::new();
        assert_eq!(velocity_anomaly_score(&b, 0), 500);
        assert_eq!(velocity_anomaly_score(&b, 9_999_999), 500);
    }

    #[test]
    fn velocity_anomaly_score_normal_volume_high_score() {
        let mut b = VelocityBaseline::new();
        for _ in 0..10 {
            b.push_epoch(100);
        }
        // Normal volume at mean (100). stddev=0, upper=100.
        // volume == upper → score = 1000 * 0 / 100 = 0, but clamped to min 1.
        let s = velocity_anomaly_score(&b, 100);
        assert!(s >= 1, "expected >= 1, got {s}");

        // Volume below the mean should score very high (maximum 1000).
        let s_low = velocity_anomaly_score(&b, 0);
        assert_eq!(s_low, 1000);
    }

    #[test]
    fn velocity_anomaly_score_anomalous_returns_zero() {
        let mut b = VelocityBaseline::new();
        for _ in 0..10 {
            b.push_epoch(100);
        }
        // Threshold = 100 + 3*0 = 100. Volume 101 > 100.
        assert_eq!(velocity_anomaly_score(&b, 10_000), 0);
        assert_eq!(velocity_anomaly_score(&b, 101), 0);
    }

    #[test]
    fn velocity_anomaly_score_insufficient_epochs_neutral() {
        let mut b = VelocityBaseline::new();
        for _ in 0..9 {
            b.push_epoch(100);
        }
        assert_eq!(velocity_anomaly_score(&b, 10_000), 500);
    }

    // --- compute_raw_score ---

    #[test]
    fn compute_raw_score_all_neutral() {
        assert_eq!(compute_raw_score(500, 500, 500, 500, 500), 500);
    }

    #[test]
    fn compute_raw_score_all_perfect() {
        assert_eq!(compute_raw_score(1000, 1000, 1000, 1000, 1000), 1000);
    }

    #[test]
    fn compute_raw_score_all_terrible() {
        assert_eq!(compute_raw_score(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn compute_raw_score_weights_sum_to_100() {
        // With score 1 for one signal and 0 for the rest the result should
        // equal the weight of that signal divided by 100 (integer division).
        // contract_fulfilment weight = 30 → 1 * 30 / 100 = 0
        assert_eq!(compute_raw_score(1, 0, 0, 0, 0), 0);
        // All-100 → 100 * (30+25+20+15+10) / 100 = 100
        assert_eq!(compute_raw_score(100, 100, 100, 100, 100), 100);
    }

    #[test]
    fn compute_raw_score_mixed_inputs() {
        // contract=1000(30%) + dispute=0(25%) + vel=500(20%) + peer=0(15%) + age=0(10%)
        // = 30000 + 0 + 10000 + 0 + 0 = 40000 / 100 = 400
        assert_eq!(compute_raw_score(1000, 0, 500, 0, 0), 400);
    }

    // --- smooth_score ---

    #[test]
    fn smooth_score_no_change() {
        assert_eq!(smooth_score(500, 500), 500);
        assert_eq!(smooth_score(0, 0), 0);
        assert_eq!(smooth_score(1000, 1000), 1000);
    }

    #[test]
    fn smooth_score_weighted_average() {
        // old=500, raw=1000 → (500*85 + 1000*15) / 100 = (42500 + 15000) / 100 = 575
        assert_eq!(smooth_score(500, 1000), 575);
    }

    #[test]
    fn smooth_score_moves_slowly_toward_raw() {
        let s = smooth_score(0, 1000);
        // (0*85 + 1000*15) / 100 = 150
        assert_eq!(s, 150);
    }

    #[test]
    fn smooth_score_clamp_high() {
        // old=1000, raw=1000 → 1000, no overflow.
        assert_eq!(smooth_score(1000, 1000), 1000);
    }

    #[test]
    fn smooth_score_clamp_low() {
        assert_eq!(smooth_score(0, 0), 0);
    }

    // --- isqrt internal helper ---

    #[test]
    fn isqrt_known_values() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(10_000), 100);
        assert_eq!(isqrt(u64::MAX), 4_294_967_295);
    }

    #[test]
    fn isqrt_floor_property() {
        for n in [2u64, 3, 5, 7, 8, 15, 24, 99, 1000, 999_983] {
            let r = isqrt(n);
            assert!(r * r <= n, "isqrt({n}) = {r}: r^2 > n");
            assert!((r + 1) * (r + 1) > n, "isqrt({n}) = {r}: (r+1)^2 <= n");
        }
    }
}
