//! Per-peer rate limiting using a sliding window approach.
//!
//! Tracks how many block messages, transaction messages, and header requests
//! each peer has sent within the last 60 seconds. When a peer exceeds the
//! configured limit the check method returns `false`, signalling the caller to
//! drop or penalize the peer.
//!
//! # Design
//!
//! Each peer has a [`PeerRateLimits`] record that holds three
//! [`VecDeque`](std::collections::VecDeque)s of [`Instant`](std::time::Instant)
//! timestamps — one per message category.  On every check the deque is first
//! pruned to remove entries older than 60 seconds, then the remaining length is
//! compared against the configured limit.  Recording a message appends the
//! current timestamp.

use libp2p::PeerId;
use rill_core::constants::{MAX_MESSAGE_SIZE, RATE_LIMIT_BLOCKS_PER_MIN, RATE_LIMIT_HEADERS_PER_MIN, RATE_LIMIT_TXS_PER_MIN};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::debug;

/// The sliding window duration — 60 seconds.
const WINDOW: Duration = Duration::from_secs(60);

/// Per-peer timestamp queues for the three rate-limited message categories.
#[derive(Debug, Clone)]
pub struct PeerRateLimits {
    /// Timestamps of recent `NewBlock` / `GetBlock` messages from this peer.
    pub blocks: VecDeque<Instant>,
    /// Timestamps of recent `NewTransaction` messages from this peer.
    pub transactions: VecDeque<Instant>,
    /// Timestamps of recent `GetHeaders` requests from this peer.
    pub headers: VecDeque<Instant>,
}

impl PeerRateLimits {
    fn new() -> Self {
        Self {
            blocks: VecDeque::new(),
            transactions: VecDeque::new(),
            headers: VecDeque::new(),
        }
    }

    /// Remove timestamps older than [`WINDOW`] from a queue.
    fn prune(queue: &mut VecDeque<Instant>) {
        let cutoff = Instant::now() - WINDOW;
        while queue.front().is_some_and(|t| *t <= cutoff) {
            queue.pop_front();
        }
    }
}

/// Manages sliding-window rate limits for all connected peers.
///
/// # Usage
///
/// Before processing a message, call the appropriate `check_*` method.  If it
/// returns `false` the peer has exceeded its rate limit and the message should
/// be discarded (and the peer possibly penalized).  After deciding to process
/// the message call the corresponding `record_*` method to register the event.
///
/// Combining check + record in one step would couple rate limiting to processing
/// outcomes; keeping them separate gives callers the flexibility to record only
/// messages that pass validation.
pub struct RateLimiter {
    peers: HashMap<PeerId, PeerRateLimits>,
}

impl RateLimiter {
    /// Create an empty rate limiter.
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Check methods — return `true` if the peer is within its limit.
    // -------------------------------------------------------------------------

    /// Returns `true` if the peer has not exceeded the block rate limit.
    ///
    /// Prunes stale entries before checking so the window always reflects the
    /// last 60 seconds.
    pub fn check_block(&mut self, peer: &PeerId) -> bool {
        let entry = self.peers.entry(*peer).or_insert_with(PeerRateLimits::new);
        PeerRateLimits::prune(&mut entry.blocks);
        let ok = entry.blocks.len() < RATE_LIMIT_BLOCKS_PER_MIN as usize;
        if !ok {
            debug!(%peer, count = entry.blocks.len(), limit = RATE_LIMIT_BLOCKS_PER_MIN,
                "rate_limiter: block rate limit exceeded");
        }
        ok
    }

    /// Returns `true` if the peer has not exceeded the transaction rate limit.
    pub fn check_transaction(&mut self, peer: &PeerId) -> bool {
        let entry = self.peers.entry(*peer).or_insert_with(PeerRateLimits::new);
        PeerRateLimits::prune(&mut entry.transactions);
        let ok = entry.transactions.len() < RATE_LIMIT_TXS_PER_MIN as usize;
        if !ok {
            debug!(%peer, count = entry.transactions.len(), limit = RATE_LIMIT_TXS_PER_MIN,
                "rate_limiter: transaction rate limit exceeded");
        }
        ok
    }

    /// Returns `true` if the peer has not exceeded the header request rate limit.
    pub fn check_headers(&mut self, peer: &PeerId) -> bool {
        let entry = self.peers.entry(*peer).or_insert_with(PeerRateLimits::new);
        PeerRateLimits::prune(&mut entry.headers);
        let ok = entry.headers.len() < RATE_LIMIT_HEADERS_PER_MIN as usize;
        if !ok {
            debug!(%peer, count = entry.headers.len(), limit = RATE_LIMIT_HEADERS_PER_MIN,
                "rate_limiter: header rate limit exceeded");
        }
        ok
    }

    // -------------------------------------------------------------------------
    // Record methods — call after deciding to process a message.
    // -------------------------------------------------------------------------

    /// Record a block message from this peer.
    pub fn record_block(&mut self, peer: &PeerId) {
        self.peers
            .entry(*peer)
            .or_insert_with(PeerRateLimits::new)
            .blocks
            .push_back(Instant::now());
        debug!(%peer, "rate_limiter: block recorded");
    }

    /// Record a transaction message from this peer.
    pub fn record_transaction(&mut self, peer: &PeerId) {
        self.peers
            .entry(*peer)
            .or_insert_with(PeerRateLimits::new)
            .transactions
            .push_back(Instant::now());
        debug!(%peer, "rate_limiter: transaction recorded");
    }

    /// Record a header request from this peer.
    pub fn record_headers(&mut self, peer: &PeerId) {
        self.peers
            .entry(*peer)
            .or_insert_with(PeerRateLimits::new)
            .headers
            .push_back(Instant::now());
        debug!(%peer, "rate_limiter: headers request recorded");
    }

    // -------------------------------------------------------------------------
    // Lifecycle
    // -------------------------------------------------------------------------

    /// Remove all rate-limit state for a peer.
    ///
    /// Call this when a peer disconnects to free memory.
    pub fn remove_peer(&mut self, peer: &PeerId) {
        if self.peers.remove(peer).is_some() {
            debug!(%peer, "rate_limiter: peer removed");
        }
    }

    // -------------------------------------------------------------------------
    // Static helpers
    // -------------------------------------------------------------------------

    /// Returns `true` if `size` is within the allowed maximum message size.
    ///
    /// Messages larger than [`MAX_MESSAGE_SIZE`] must be rejected before
    /// deserialization to prevent memory exhaustion attacks.
    pub fn check_message_size(size: usize) -> bool {
        size <= MAX_MESSAGE_SIZE
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer() -> PeerId {
        PeerId::random()
    }

    /// Under-limit block requests all pass.
    #[test]
    fn within_block_rate_limit() {
        let mut rl = RateLimiter::new();
        let peer = make_peer();

        for _ in 0..RATE_LIMIT_BLOCKS_PER_MIN {
            assert!(rl.check_block(&peer), "request within limit should pass");
            rl.record_block(&peer);
        }
    }

    /// Once the limit is reached the next check fails.
    #[test]
    fn exceeds_block_rate_limit() {
        let mut rl = RateLimiter::new();
        let peer = make_peer();

        // Fill up to the limit.
        for _ in 0..RATE_LIMIT_BLOCKS_PER_MIN {
            assert!(rl.check_block(&peer));
            rl.record_block(&peer);
        }

        // One more should be rejected.
        assert!(!rl.check_block(&peer), "request over limit should be rejected");
    }

    /// Old entries that fall outside the 60-second window are pruned, so the
    /// limit resets for "old" traffic.  We simulate passage of time by backdating
    /// the stored timestamps.
    #[test]
    fn rate_limit_window_slides() {
        let mut rl = RateLimiter::new();
        let peer = make_peer();

        // Fill up to the limit with timestamps backdated to just beyond the window.
        {
            let entry = rl.peers.entry(peer).or_insert_with(PeerRateLimits::new);
            let old = Instant::now() - WINDOW - Duration::from_secs(1);
            for _ in 0..RATE_LIMIT_BLOCKS_PER_MIN {
                entry.blocks.push_back(old);
            }
        }

        // All old entries should have been pruned, so the check should pass.
        assert!(
            rl.check_block(&peer),
            "after window slides, limit should have reset"
        );
    }

    /// Rate limits for one peer must not affect a different peer.
    #[test]
    fn per_peer_isolation() {
        let mut rl = RateLimiter::new();
        let peer_a = make_peer();
        let peer_b = make_peer();

        // Exhaust peer_a's block limit.
        for _ in 0..RATE_LIMIT_BLOCKS_PER_MIN {
            rl.record_block(&peer_a);
        }

        // peer_b should still have a clean slate.
        assert!(
            rl.check_block(&peer_b),
            "peer_b should be unaffected by peer_a's usage"
        );
        // peer_a should be rejected.
        assert!(
            !rl.check_block(&peer_a),
            "peer_a should be over limit"
        );
    }

    /// Messages within the size cap pass; oversized messages are rejected.
    #[test]
    fn message_size_check() {
        assert!(
            RateLimiter::check_message_size(0),
            "zero-byte message should pass"
        );
        assert!(
            RateLimiter::check_message_size(MAX_MESSAGE_SIZE),
            "exactly at the limit should pass"
        );
        assert!(
            !RateLimiter::check_message_size(MAX_MESSAGE_SIZE + 1),
            "one byte over limit should be rejected"
        );
        assert!(
            !RateLimiter::check_message_size(usize::MAX),
            "maximum usize should be rejected"
        );
    }

    /// After `remove_peer` the peer's state is gone and limits reset.
    #[test]
    fn peer_cleanup() {
        let mut rl = RateLimiter::new();
        let peer = make_peer();

        // Exhaust all three categories.
        for _ in 0..RATE_LIMIT_BLOCKS_PER_MIN {
            rl.record_block(&peer);
        }
        for _ in 0..RATE_LIMIT_TXS_PER_MIN {
            rl.record_transaction(&peer);
        }
        for _ in 0..RATE_LIMIT_HEADERS_PER_MIN {
            rl.record_headers(&peer);
        }

        // All three should be over limit.
        assert!(!rl.check_block(&peer));
        assert!(!rl.check_transaction(&peer));
        assert!(!rl.check_headers(&peer));

        // Remove the peer.
        rl.remove_peer(&peer);

        // State should be cleared — all checks pass again.
        assert!(rl.check_block(&peer), "block limit should reset after remove_peer");
        assert!(rl.check_transaction(&peer), "tx limit should reset after remove_peer");
        assert!(rl.check_headers(&peer), "header limit should reset after remove_peer");

        // Internal map should be empty (the entry above was just created fresh by check_*).
        // Remove again — should be a no-op without panic.
        rl.remove_peer(&peer);
    }
}
