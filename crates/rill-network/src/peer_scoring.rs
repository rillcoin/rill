//! Peer scoring and banning system.
//!
//! Tracks per-peer scores based on observed behaviour. Peers that send invalid
//! blocks, invalid transactions, or time out repeatedly accumulate negative
//! score. Once a peer's score drops to or below [`BAN_THRESHOLD`] it is banned
//! for [`BAN_DURATION`]. Good behaviour (valid blocks/headers) earns positive
//! score, capped at `+100`.

use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Penalty applied when a peer sends an invalid block.
pub const PENALTY_INVALID_BLOCK: i64 = -100;
/// Penalty applied when a peer sends invalid headers.
pub const PENALTY_INVALID_HEADERS: i64 = -50;
/// Penalty applied when a peer times out on a request.
pub const PENALTY_TIMEOUT: i64 = -10;
/// Penalty applied when a peer sends an invalid transaction.
pub const PENALTY_INVALID_TRANSACTION: i64 = -25;
/// Penalty applied when a peer sends a duplicate message.
pub const PENALTY_DUPLICATE_MESSAGE: i64 = -5;

/// Score threshold at which a peer is banned.
pub const BAN_THRESHOLD: i64 = -200;

/// How long a ban lasts.
pub const BAN_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Score bonus awarded when a peer provides a valid block.
pub const BONUS_VALID_BLOCK: i64 = 10;
/// Score bonus awarded when a peer provides valid headers.
pub const BONUS_VALID_HEADERS: i64 = 5;

/// Maximum score a peer can accumulate (rewards are capped at this value).
const MAX_SCORE: i64 = 100;

/// Per-peer score record.
#[derive(Debug, Clone)]
pub struct PeerScore {
    /// Cumulative score (starts at 0, negative is bad).
    pub score: i64,
    /// When this peer was banned (`None` if not currently banned).
    pub banned_at: Option<Instant>,
    /// Total penalty points received over the lifetime of this record.
    pub total_penalties: u64,
    /// Total bonus points received over the lifetime of this record.
    pub total_bonuses: u64,
}

impl PeerScore {
    fn new() -> Self {
        Self {
            score: 0,
            banned_at: None,
            total_penalties: 0,
            total_bonuses: 0,
        }
    }
}

/// Manages peer scores for all known peers.
///
/// # Usage
///
/// Call [`PeerScoreBoard::penalize`] when a peer misbehaves and
/// [`PeerScoreBoard::reward`] when a peer does something useful. Poll
/// [`PeerScoreBoard::is_banned`] before accepting work from a peer, and call
/// [`PeerScoreBoard::unban_expired`] periodically to lift expired bans.
pub struct PeerScoreBoard {
    scores: HashMap<PeerId, PeerScore>,
}

impl PeerScoreBoard {
    /// Create an empty score board.
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
        }
    }

    /// Apply a penalty (negative value) to a peer.
    ///
    /// Returns `true` if this penalty caused the peer to be banned (i.e. the
    /// peer crossed [`BAN_THRESHOLD`] with this call and was not already
    /// banned).
    pub fn penalize(&mut self, peer: &PeerId, penalty: i64) -> bool {
        let entry = self.scores.entry(*peer).or_insert_with(PeerScore::new);

        // Only penalise peers that are not already banned.
        if entry.banned_at.is_some() {
            debug!(%peer, "peer_score: skipping penalty for already-banned peer");
            return false;
        }

        entry.score = entry.score.saturating_add(penalty);
        entry.total_penalties = entry.total_penalties.saturating_add(penalty.unsigned_abs());

        debug!(%peer, score = entry.score, penalty, "peer_score: penalty applied");

        // Check if we just crossed the ban threshold.
        if entry.score <= BAN_THRESHOLD {
            entry.banned_at = Some(Instant::now());
            warn!(%peer, score = entry.score, "peer_score: peer banned");
            return true;
        }

        false
    }

    /// Apply a bonus (positive value) to a peer's score, capped at [`MAX_SCORE`].
    pub fn reward(&mut self, peer: &PeerId, bonus: i64) {
        let entry = self.scores.entry(*peer).or_insert_with(PeerScore::new);

        entry.score = entry.score.saturating_add(bonus).min(MAX_SCORE);
        entry.total_bonuses = entry.total_bonuses.saturating_add(bonus.unsigned_abs());

        debug!(%peer, score = entry.score, bonus, "peer_score: bonus applied");
    }

    /// Returns `true` if the peer is currently banned (and the ban has not yet
    /// expired).
    pub fn is_banned(&self, peer: &PeerId) -> bool {
        match self.scores.get(peer) {
            Some(ps) => match ps.banned_at {
                Some(banned_at) => banned_at.elapsed() < BAN_DURATION,
                None => false,
            },
            None => false,
        }
    }

    /// Returns the peer's current score, or `0` if the peer is unknown.
    pub fn score(&self, peer: &PeerId) -> i64 {
        self.scores.get(peer).map(|ps| ps.score).unwrap_or(0)
    }

    /// Lift all bans whose [`BAN_DURATION`] has elapsed.
    ///
    /// Resets the score of each unbanned peer to `0` and returns the list of
    /// peers whose bans were lifted so callers can reconnect them if desired.
    pub fn unban_expired(&mut self) -> Vec<PeerId> {
        let mut unbanned = Vec::new();

        for (peer, ps) in self.scores.iter_mut() {
            if let Some(banned_at) = ps.banned_at {
                if banned_at.elapsed() >= BAN_DURATION {
                    ps.banned_at = None;
                    ps.score = 0;
                    unbanned.push(*peer);
                    info!(%peer, "peer_score: ban expired, peer unbanned");
                }
            }
        }

        unbanned
    }

    /// Remove all tracking data for a peer (call on disconnect).
    pub fn remove_peer(&mut self, peer: &PeerId) {
        if self.scores.remove(peer).is_some() {
            debug!(%peer, "peer_score: peer removed from score board");
        }
    }

    /// Returns the [`PeerId`]s of all currently banned peers.
    pub fn banned_peers(&self) -> Vec<PeerId> {
        self.scores
            .iter()
            .filter(|(_, ps)| {
                ps.banned_at
                    .map(|t| t.elapsed() < BAN_DURATION)
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns a snapshot of the score record for a peer, or `None` if the
    /// peer is unknown.
    pub fn peer_info(&self, peer: &PeerId) -> Option<PeerScore> {
        self.scores.get(peer).cloned()
    }
}

impl Default for PeerScoreBoard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn make_peer() -> PeerId {
        PeerId::random()
    }

    // -------------------------------------------------------------------------
    // Tests
    // -------------------------------------------------------------------------

    /// A peer that has never been seen starts with score 0.
    #[test]
    fn new_peer_starts_at_zero() {
        let board = PeerScoreBoard::new();
        let peer = make_peer();
        assert_eq!(board.score(&peer), 0);
    }

    /// Applying a penalty reduces the peer's score.
    #[test]
    fn penalize_decreases_score() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        let banned = board.penalize(&peer, PENALTY_TIMEOUT);
        assert!(!banned, "small penalty should not trigger ban");
        assert_eq!(board.score(&peer), PENALTY_TIMEOUT);
    }

    /// Rewards increase the score but are capped at MAX_SCORE (100).
    #[test]
    fn reward_increases_score() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        board.reward(&peer, BONUS_VALID_BLOCK);
        assert_eq!(board.score(&peer), BONUS_VALID_BLOCK);

        // Applying many bonuses must not exceed the cap.
        for _ in 0..50 {
            board.reward(&peer, BONUS_VALID_BLOCK);
        }
        assert_eq!(board.score(&peer), MAX_SCORE, "score must not exceed MAX_SCORE");
    }

    /// A peer is banned once its score reaches BAN_THRESHOLD.
    #[test]
    fn ban_triggered_at_threshold() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        // Two PENALTY_INVALID_BLOCK penalties bring score to -200, exactly the threshold.
        let first = board.penalize(&peer, PENALTY_INVALID_BLOCK);
        assert!(!first, "first penalty should not ban");

        let second = board.penalize(&peer, PENALTY_INVALID_BLOCK);
        assert!(second, "second penalty should trigger ban at threshold");

        assert!(board.is_banned(&peer));
    }

    /// is_banned returns true for a peer that has been banned.
    #[test]
    fn banned_peer_detected() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        // Drive score below threshold with a single huge penalty.
        board.penalize(&peer, BAN_THRESHOLD - 1);

        assert!(board.is_banned(&peer), "peer should be detected as banned");
        assert!(
            board.banned_peers().contains(&peer),
            "peer should appear in banned_peers()"
        );
    }

    /// After BAN_DURATION has elapsed, is_banned returns false (simulated by
    /// manipulating the banned_at timestamp directly).
    #[test]
    fn ban_expires_after_duration() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        board.penalize(&peer, BAN_THRESHOLD - 1);
        assert!(board.is_banned(&peer));

        // Backdate the ban so it appears to have expired.
        if let Some(ps) = board.scores.get_mut(&peer) {
            ps.banned_at = Some(Instant::now() - BAN_DURATION - Duration::from_secs(1));
        }

        assert!(!board.is_banned(&peer), "expired ban should not count as banned");
    }

    /// unban_expired clears expired bans and resets the peer's score to 0.
    #[test]
    fn unban_expired_clears_old_bans() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        board.penalize(&peer, BAN_THRESHOLD - 1);
        assert!(board.is_banned(&peer));

        // Backdate the ban.
        if let Some(ps) = board.scores.get_mut(&peer) {
            ps.banned_at = Some(Instant::now() - BAN_DURATION - Duration::from_secs(1));
        }

        let unbanned = board.unban_expired();
        assert!(unbanned.contains(&peer), "peer should be in unbanned list");
        assert!(!board.is_banned(&peer), "peer should no longer be banned");
        assert_eq!(board.score(&peer), 0, "score should reset to 0 after unban");
    }

    /// remove_peer erases all tracking data for that peer.
    #[test]
    fn remove_peer_clears_data() {
        let mut board = PeerScoreBoard::new();
        let peer = make_peer();

        board.reward(&peer, BONUS_VALID_BLOCK);
        assert!(board.peer_info(&peer).is_some());

        board.remove_peer(&peer);

        assert!(board.peer_info(&peer).is_none(), "peer_info should be None after removal");
        assert_eq!(board.score(&peer), 0, "score should be 0 for unknown peer after removal");
        assert!(!board.is_banned(&peer));
    }
}
