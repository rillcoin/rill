//! Chain synchronization state machine.
//!
//! Implements header-first sync protocol with multi-peer parallel block download.

use libp2p::PeerId;
use rill_core::types::{Block, BlockHeader, Hash256};
use std::collections::{HashMap, VecDeque};
use tracing::{debug, info, warn};

/// Default timeout for a block request (seconds).
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
/// Maximum failures before banning a peer.
pub const DEFAULT_MAX_FAILURES: u32 = 3;
/// Maximum concurrent in-flight requests per peer.
pub const DEFAULT_MAX_IN_FLIGHT_PER_PEER: u32 = 8;

/// The current state of the chain synchronization process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// Not syncing — we're caught up.
    Idle,
    /// Querying peers to find their chain tips.
    DiscoveringPeers,
    /// Downloading headers from the best peer to the target height.
    DownloadingHeaders { target_height: u64 },
    /// Downloading blocks from the header chain.
    DownloadingBlocks { remaining: Vec<Hash256> },
    /// Synchronization complete.
    Done,
}

/// Actions to take as a result of sync state machine progression.
#[derive(Debug, Clone)]
pub enum SyncAction {
    /// Request the chain tip from a peer.
    RequestChainTip(PeerId),
    /// Request headers from a peer using the given locator.
    RequestHeaders {
        peer: PeerId,
        locator: Vec<Hash256>,
    },
    /// Request a specific block by hash from a peer.
    RequestBlock { peer: PeerId, hash: Hash256 },
    /// Connect a downloaded block to the chain.
    ConnectBlock(Block),
    /// Synchronization is complete.
    SyncComplete,
    /// No action to take right now.
    Wait,
}

/// Metadata about a peer's chain tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerTip {
    /// The height of the peer's chain tip.
    pub height: u64,
    /// The block hash at the tip.
    pub hash: Hash256,
}

/// Per-peer synchronization metadata.
#[derive(Debug, Clone)]
pub struct PeerState {
    /// The peer's chain tip.
    pub tip: PeerTip,
    /// Number of outstanding block requests to this peer.
    pub in_flight: u32,
    /// Number of failed/timed-out requests from this peer.
    pub failures: u32,
    /// Timestamp of last request sent (for timeout detection).
    pub last_request_at: Option<std::time::Instant>,
    /// Whether this peer is banned (too many failures).
    pub banned: bool,
}

impl PeerState {
    /// Compute a score for peer selection. Higher is better.
    ///
    /// Peers with more failures are penalised heavily.
    fn score(&self) -> i64 {
        self.tip.height as i64 - self.failures as i64 * 1000
    }
}

/// Chain synchronization manager.
///
/// Uses a state machine to coordinate header-first sync with multi-peer
/// parallel block download.
pub struct SyncManager {
    /// Current synchronization state.
    state: SyncState,
    /// The best peer we've found and their chain tip (backward-compat field).
    best_peer: Option<(PeerId, PeerTip)>,
    /// Headers we've downloaded but not yet processed into blocks.
    pending_headers: Vec<BlockHeader>,
    /// Block hashes we need to download.
    blocks_to_download: VecDeque<Hash256>,
    /// All known peers and their per-peer state.
    peers: HashMap<PeerId, PeerState>,
    /// Cached ID of the current best peer.
    best_peer_id: Option<PeerId>,
    /// Map from in-flight block hash to the peer it was assigned to.
    in_flight: HashMap<Hash256, PeerId>,
}

impl SyncManager {
    /// Create a new sync manager in the Idle state.
    pub fn new() -> Self {
        Self {
            state: SyncState::Idle,
            best_peer: None,
            pending_headers: Vec::new(),
            blocks_to_download: VecDeque::new(),
            peers: HashMap::new(),
            best_peer_id: None,
            in_flight: HashMap::new(),
        }
    }

    /// Recompute `best_peer_id` and keep `best_peer` in sync.
    fn refresh_best_peer(&mut self) {
        let best = self
            .peers
            .iter()
            .filter(|(_, ps)| !ps.banned)
            .max_by_key(|(_, ps)| ps.tip.height)
            .map(|(id, ps)| (*id, ps.tip.clone()));

        match best {
            Some((id, tip)) => {
                self.best_peer_id = Some(id);
                self.best_peer = Some((id, tip));
            }
            None => {
                self.best_peer_id = None;
                self.best_peer = None;
            }
        }
    }

    /// A peer has connected — register them and optionally begin peer discovery.
    pub fn on_peer_connected(&mut self, peer: PeerId) {
        debug!(%peer, "sync: peer connected");

        // Register with a zero tip until we hear from them.
        self.peers.entry(peer).or_insert_with(|| PeerState {
            tip: PeerTip {
                height: 0,
                hash: Hash256::ZERO,
            },
            in_flight: 0,
            failures: 0,
            last_request_at: None,
            banned: false,
        });

        // If we're idle, start discovering peers.
        if matches!(self.state, SyncState::Idle) {
            self.state = SyncState::DiscoveringPeers;
        }
    }

    /// We received a peer's chain tip response.
    pub fn on_peer_tip(&mut self, peer: PeerId, height: u64, hash: Hash256) {
        debug!(%peer, height, %hash, "sync: received peer tip");
        let tip = PeerTip { height, hash };

        // Update (or insert) the peer's tip in the peers map.
        self.peers
            .entry(peer)
            .and_modify(|ps| ps.tip = tip.clone())
            .or_insert_with(|| PeerState {
                tip: tip.clone(),
                in_flight: 0,
                failures: 0,
                last_request_at: None,
                banned: false,
            });

        // Refresh the cached best peer.
        let should_update = match &self.best_peer {
            None => true,
            Some((_, current_best)) => height > current_best.height,
        };

        if should_update {
            debug!(%peer, height, "sync: new best peer");
            self.best_peer_id = Some(peer);
            self.best_peer = Some((peer, tip));
        }
    }

    /// A peer has disconnected.
    ///
    /// Removes it from the peers map and reassigns any in-flight blocks for
    /// that peer back to the front of the download queue.
    pub fn on_peer_disconnected(&mut self, peer: PeerId) {
        info!(%peer, "sync: peer disconnected");

        // Collect all blocks assigned to this peer.
        let mut reassign: Vec<Hash256> = self
            .in_flight
            .iter()
            .filter_map(|(hash, &assigned)| {
                if assigned == peer {
                    Some(*hash)
                } else {
                    None
                }
            })
            .collect();

        // Remove them from in-flight.
        for hash in &reassign {
            self.in_flight.remove(hash);
        }

        // Push reassigned blocks to the front of the queue (preserve order).
        reassign.reverse();
        for hash in reassign {
            self.blocks_to_download.push_front(hash);
        }

        self.peers.remove(&peer);

        // Refresh best peer now that this peer is gone.
        let was_best = self.best_peer_id == Some(peer);
        if was_best {
            self.refresh_best_peer();
        }
    }

    /// Check all in-flight requests for timeouts.
    ///
    /// Timed-out requests are reassigned to the front of the download queue.
    /// Peers that exceed `DEFAULT_MAX_FAILURES` are banned.
    pub fn check_timeouts(&mut self) {
        let timeout = std::time::Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS);
        let now = std::time::Instant::now();

        let timed_out: Vec<Hash256> = self
            .in_flight
            .iter()
            .filter_map(|(hash, peer_id)| {
                if let Some(ps) = self.peers.get(peer_id) {
                    if let Some(last) = ps.last_request_at {
                        if now.duration_since(last) >= timeout {
                            return Some(*hash);
                        }
                    }
                }
                None
            })
            .collect();

        for hash in timed_out {
            if let Some(peer_id) = self.in_flight.remove(&hash) {
                self.blocks_to_download.push_front(hash);

                if let Some(ps) = self.peers.get_mut(&peer_id) {
                    ps.in_flight = ps.in_flight.saturating_sub(1);
                    ps.failures += 1;
                    if ps.failures >= DEFAULT_MAX_FAILURES {
                        warn!(%peer_id, failures = ps.failures, "sync: banning peer after max failures");
                        ps.banned = true;
                    }
                }
            }
        }

        // If best peer got banned, refresh.
        if let Some(best_id) = self.best_peer_id {
            if self.peers.get(&best_id).map(|ps| ps.banned).unwrap_or(false) {
                self.refresh_best_peer();
            }
        }
    }

    /// We received a batch of headers from a peer.
    ///
    /// Validates header linkage and transitions to downloading blocks.
    pub fn on_headers_received(&mut self, headers: Vec<BlockHeader>) {
        if headers.is_empty() {
            debug!("sync: received empty headers response");
            return;
        }

        debug!(count = headers.len(), "sync: received headers");

        // Validate header chain linkage.
        for i in 1..headers.len() {
            if headers[i].prev_hash != headers[i - 1].hash() {
                debug!("sync: invalid header chain linkage, resetting");
                self.state = SyncState::Idle;
                self.pending_headers.clear();
                return;
            }
        }

        // Store headers and build download queue.
        self.pending_headers.extend(headers.clone());
        let hashes: Vec<Hash256> = headers.iter().map(|h| h.hash()).collect();
        self.blocks_to_download.extend(hashes.clone());

        info!(
            count = hashes.len(),
            total_pending = self.blocks_to_download.len(),
            "sync: queued blocks for download"
        );

        // Transition to downloading blocks.
        self.state = SyncState::DownloadingBlocks {
            remaining: hashes,
        };
    }

    /// We received a block from a peer.
    ///
    /// Removes it from the download queue and from the in-flight map.
    pub fn on_block_received(&mut self, block: Block) {
        let block_hash = block.header.hash();
        debug!(%block_hash, "sync: received block");

        // Remove from in-flight map and decrement peer's counter.
        if let Some(peer_id) = self.in_flight.remove(&block_hash) {
            if let Some(ps) = self.peers.get_mut(&peer_id) {
                ps.in_flight = ps.in_flight.saturating_sub(1);
            }
        }

        // Remove from download queue if present.
        if let Some(pos) = self.blocks_to_download.iter().position(|h| *h == block_hash) {
            self.blocks_to_download.remove(pos);
            debug!(
                remaining = self.blocks_to_download.len(),
                "sync: block download progress"
            );
        }

        // If download queue is empty, we're done.
        if self.blocks_to_download.is_empty()
            && self.in_flight.is_empty()
            && !matches!(self.state, SyncState::Idle)
        {
            info!("sync: all blocks downloaded, transitioning to Done");
            self.state = SyncState::Done;
        }
    }

    /// Check if we should sync based on our current height vs best peer.
    pub fn should_sync(&self, our_height: u64) -> bool {
        if let Some((_, peer_tip)) = &self.best_peer {
            peer_tip.height > our_height
        } else {
            false
        }
    }

    /// Get the next actions to take given our current height and chain state.
    ///
    /// In the `DownloadingBlocks` state this will distribute work across all
    /// available (non-banned, not-at-capacity) peers. In all other states it
    /// returns at most one action, matching the behaviour of the original
    /// `next_action`.
    pub fn next_actions<F>(&mut self, our_height: u64, get_locator: F) -> Vec<SyncAction>
    where
        F: Fn() -> Vec<Hash256>,
    {
        match &self.state.clone() {
            SyncState::Idle => {
                if self.should_sync(our_height) {
                    if let Some((peer, peer_tip)) = &self.best_peer.clone() {
                        info!(
                            %peer,
                            our_height,
                            peer_height = peer_tip.height,
                            "sync: starting header download"
                        );
                        let locator = get_locator();
                        self.state = SyncState::DownloadingHeaders {
                            target_height: peer_tip.height,
                        };
                        return vec![SyncAction::RequestHeaders {
                            peer: *peer,
                            locator,
                        }];
                    }
                }
                vec![SyncAction::Wait]
            }
            SyncState::DiscoveringPeers => {
                if let Some((peer, _)) = &self.best_peer {
                    vec![SyncAction::RequestChainTip(*peer)]
                } else {
                    vec![SyncAction::Wait]
                }
            }
            SyncState::DownloadingHeaders { .. } => {
                vec![SyncAction::Wait]
            }
            SyncState::DownloadingBlocks { .. } => {
                // Build a sorted list of available peers (not banned, under capacity).
                let mut available: Vec<(PeerId, i64)> = self
                    .peers
                    .iter()
                    .filter(|(_, ps)| !ps.banned && ps.in_flight < DEFAULT_MAX_IN_FLIGHT_PER_PEER)
                    .map(|(id, ps)| (*id, ps.score()))
                    .collect();

                // Sort descending by score so we prefer higher-scoring peers.
                available.sort_by(|a, b| b.1.cmp(&a.1));

                if available.is_empty() {
                    return vec![SyncAction::Wait];
                }

                let mut actions = Vec::new();
                let mut peer_cursor = 0usize;

                // Walk the queue and assign hashes that are not already in-flight.
                let hashes_to_assign: Vec<Hash256> = self
                    .blocks_to_download
                    .iter()
                    .filter(|h| !self.in_flight.contains_key(*h))
                    .copied()
                    .collect();

                for hash in hashes_to_assign {
                    // Re-check capacity for the current cursor peer.
                    // We may have filled it up in this loop iteration.
                    let initial_cursor = peer_cursor;
                    loop {
                        let (peer_id, _) = available[peer_cursor % available.len()];
                        let ps = self.peers.get(&peer_id);
                        let current_in_flight = ps.map(|p| p.in_flight).unwrap_or(u32::MAX);

                        if current_in_flight < DEFAULT_MAX_IN_FLIGHT_PER_PEER {
                            // Assign this hash to this peer.
                            self.in_flight.insert(hash, peer_id);
                            if let Some(ps) = self.peers.get_mut(&peer_id) {
                                ps.in_flight += 1;
                                ps.last_request_at = Some(std::time::Instant::now());
                            }
                            actions.push(SyncAction::RequestBlock {
                                peer: peer_id,
                                hash,
                            });
                            peer_cursor += 1;
                            break;
                        }

                        peer_cursor += 1;

                        // If we've cycled through all peers and none can take more, stop.
                        if peer_cursor - initial_cursor >= available.len() {
                            return if actions.is_empty() {
                                vec![SyncAction::Wait]
                            } else {
                                actions
                            };
                        }
                    }
                }

                if actions.is_empty() {
                    vec![SyncAction::Wait]
                } else {
                    actions
                }
            }
            SyncState::Done => {
                info!("sync: complete, returning to idle");
                self.state = SyncState::Idle;
                self.best_peer = None;
                self.best_peer_id = None;
                self.pending_headers.clear();
                vec![SyncAction::SyncComplete]
            }
        }
    }

    /// Get the next action to take given our current height and chain state.
    ///
    /// This is a backward-compatibility shim that delegates to `next_actions`
    /// and returns the first element, or `Wait` if the list is empty.
    pub fn next_action<F>(&mut self, our_height: u64, get_locator: F) -> SyncAction
    where
        F: Fn() -> Vec<Hash256>,
    {
        self.next_actions(our_height, get_locator)
            .into_iter()
            .next()
            .unwrap_or(SyncAction::Wait)
    }

    /// Get the current sync state.
    pub fn state(&self) -> &SyncState {
        &self.state
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::types::BlockHeader;

    fn peer(_id: u8) -> PeerId {
        PeerId::random()
    }

    fn sample_header(height: u64, prev_hash: Hash256) -> BlockHeader {
        BlockHeader {
            version: 1,
            prev_hash,
            merkle_root: Hash256::ZERO,
            timestamp: 1_700_000_000 + height * 60,
            difficulty_target: u64::MAX,
            nonce: height,
        }
    }

    // -------------------------------------------------------------------------
    // Existing tests (must all continue to pass)
    // -------------------------------------------------------------------------

    #[test]
    fn initial_state_is_idle() {
        let mgr = SyncManager::new();
        assert_eq!(*mgr.state(), SyncState::Idle);
    }

    #[test]
    fn on_peer_connected_transitions_to_discovering() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.on_peer_connected(p);
        assert_eq!(*mgr.state(), SyncState::DiscoveringPeers);
    }

    #[test]
    fn on_peer_tip_updates_best_peer() {
        let mut mgr = SyncManager::new();
        let p1 = peer(1);
        mgr.on_peer_tip(p1, 10, Hash256([0xAA; 32]));
        assert_eq!(mgr.best_peer.as_ref().map(|(_, t)| t.height), Some(10));

        let p2 = peer(2);
        mgr.on_peer_tip(p2, 20, Hash256([0xBB; 32]));
        assert_eq!(mgr.best_peer.as_ref().map(|(_, t)| t.height), Some(20));
    }

    #[test]
    fn should_sync_false_when_caught_up() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));
        assert!(!mgr.should_sync(10));
        assert!(!mgr.should_sync(11));
    }

    #[test]
    fn should_sync_true_when_behind() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));
        assert!(mgr.should_sync(5));
        assert!(mgr.should_sync(9));
    }

    #[test]
    fn next_action_returns_wait_when_idle_and_caught_up() {
        let mut mgr = SyncManager::new();
        let action = mgr.next_action(0, || vec![]);
        assert!(matches!(action, SyncAction::Wait));
    }

    #[test]
    fn next_action_requests_chain_tip_when_discovering() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.on_peer_connected(p);
        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));

        // State is DiscoveringPeers, should request chain tip.
        let action = mgr.next_action(0, || vec![]);
        assert!(matches!(action, SyncAction::RequestChainTip(_)));
    }

    #[test]
    fn next_action_requests_headers_when_behind() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));

        // We're at height 0, peer at 10, should request headers.
        let action = mgr.next_action(0, || vec![Hash256::ZERO]);
        match action {
            SyncAction::RequestHeaders { peer, locator } => {
                assert_eq!(peer, p);
                assert_eq!(locator, vec![Hash256::ZERO]);
            }
            _ => panic!("expected RequestHeaders, got {:?}", action),
        }

        // State should transition to DownloadingHeaders.
        assert!(matches!(mgr.state(), SyncState::DownloadingHeaders { .. }));
    }

    #[test]
    fn on_headers_received_validates_linkage() {
        let mut mgr = SyncManager::new();
        let h0 = sample_header(0, Hash256::ZERO);
        let h1 = sample_header(1, h0.hash());
        let h2 = sample_header(2, h1.hash());

        mgr.on_headers_received(vec![h0.clone(), h1.clone(), h2.clone()]);

        // Should transition to DownloadingBlocks.
        assert!(matches!(mgr.state(), SyncState::DownloadingBlocks { .. }));
        assert_eq!(mgr.blocks_to_download.len(), 3);
    }

    #[test]
    fn on_headers_received_rejects_invalid_chain() {
        let mut mgr = SyncManager::new();
        let h0 = sample_header(0, Hash256::ZERO);
        let h1 = sample_header(1, Hash256([0xFF; 32])); // Wrong prev_hash

        mgr.on_headers_received(vec![h0, h1]);

        // Should reset to Idle.
        assert_eq!(*mgr.state(), SyncState::Idle);
        assert_eq!(mgr.pending_headers.len(), 0);
    }

    #[test]
    fn on_block_received_removes_from_queue() {
        let mut mgr = SyncManager::new();
        let h0 = sample_header(0, Hash256::ZERO);
        let h1 = sample_header(1, h0.hash());

        mgr.on_headers_received(vec![h0.clone(), h1.clone()]);
        assert_eq!(mgr.blocks_to_download.len(), 2);

        let block0 = Block {
            header: h0,
            transactions: vec![],
        };
        mgr.on_block_received(block0);
        assert_eq!(mgr.blocks_to_download.len(), 1);
    }

    #[test]
    fn state_transitions_to_done_when_all_blocks_downloaded() {
        let mut mgr = SyncManager::new();
        let h0 = sample_header(0, Hash256::ZERO);
        mgr.on_headers_received(vec![h0.clone()]);

        assert!(matches!(mgr.state(), SyncState::DownloadingBlocks { .. }));

        let block0 = Block {
            header: h0,
            transactions: vec![],
        };
        mgr.on_block_received(block0);

        // Should transition to Done.
        assert_eq!(*mgr.state(), SyncState::Done);
    }

    #[test]
    fn next_action_returns_sync_complete_when_done() {
        let mut mgr = SyncManager::new();
        mgr.state = SyncState::Done;

        let action = mgr.next_action(10, || vec![]);
        assert!(matches!(action, SyncAction::SyncComplete));
        assert_eq!(*mgr.state(), SyncState::Idle);
    }

    #[test]
    fn next_action_requests_block_when_downloading() {
        let mut mgr = SyncManager::new();
        let p = peer(1);
        mgr.best_peer = Some((p, PeerTip { height: 10, hash: Hash256([0xAA; 32]) }));
        // Also register in the peers map so next_actions can distribute work.
        mgr.peers.insert(
            p,
            PeerState {
                tip: PeerTip { height: 10, hash: Hash256([0xAA; 32]) },
                in_flight: 0,
                failures: 0,
                last_request_at: None,
                banned: false,
            },
        );
        mgr.best_peer_id = Some(p);

        let h0 = sample_header(0, Hash256::ZERO);
        mgr.on_headers_received(vec![h0.clone()]);

        let action = mgr.next_action(0, || vec![]);
        match action {
            SyncAction::RequestBlock { peer, hash } => {
                assert_eq!(peer, p);
                assert_eq!(hash, h0.hash());
            }
            _ => panic!("expected RequestBlock, got {:?}", action),
        }
    }

    // -------------------------------------------------------------------------
    // New tests
    // -------------------------------------------------------------------------

    /// Blocks are distributed across multiple peers in parallel.
    #[test]
    fn multi_peer_block_distribution() {
        let mut mgr = SyncManager::new();

        let p1 = peer(1);
        let p2 = peer(2);
        let p3 = peer(3);

        // Register peers with ascending tips.
        mgr.on_peer_tip(p1, 100, Hash256([0x01; 32]));
        mgr.on_peer_tip(p2, 110, Hash256([0x02; 32]));
        mgr.on_peer_tip(p3, 120, Hash256([0x03; 32]));

        // Build a chain of 6 linked headers.
        let mut headers = Vec::new();
        let mut prev = Hash256::ZERO;
        for i in 0..6u64 {
            let h = sample_header(i, prev);
            prev = h.hash();
            headers.push(h);
        }

        mgr.on_headers_received(headers);
        assert_eq!(mgr.blocks_to_download.len(), 6);

        let actions = mgr.next_actions(0, || vec![]);

        // We should get 6 RequestBlock actions (one per hash).
        let block_actions: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, SyncAction::RequestBlock { .. }))
            .collect();
        assert_eq!(block_actions.len(), 6, "expected 6 RequestBlock actions");

        // Collect which peers were assigned work.
        let assigned_peers: std::collections::HashSet<PeerId> = block_actions
            .iter()
            .filter_map(|a| {
                if let SyncAction::RequestBlock { peer, .. } = a {
                    Some(*peer)
                } else {
                    None
                }
            })
            .collect();

        // At least 2 distinct peers should have been used (likely 3).
        assert!(
            assigned_peers.len() >= 2,
            "expected blocks distributed across at least 2 peers, got {}",
            assigned_peers.len()
        );
    }

    /// Disconnecting a peer reassigns its in-flight blocks to the download queue.
    #[test]
    fn disconnect_reassigns_blocks() {
        let mut mgr = SyncManager::new();
        let p = peer(1);

        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));

        // Queue a single header / block.
        let h0 = sample_header(0, Hash256::ZERO);
        let hash = h0.hash();
        mgr.on_headers_received(vec![h0]);

        // Simulate assignment by calling next_actions.
        let actions = mgr.next_actions(0, || vec![]);
        assert!(actions.iter().any(|a| matches!(a, SyncAction::RequestBlock { .. })));

        // Hash should now be in-flight.
        assert!(mgr.in_flight.contains_key(&hash));

        // Disconnect the peer — block should return to queue.
        mgr.on_peer_disconnected(p);

        assert!(!mgr.in_flight.contains_key(&hash), "hash should leave in-flight");
        assert!(
            mgr.blocks_to_download.contains(&hash),
            "hash should be back in download queue"
        );
    }

    /// Timed-out requests are reassigned and the peer's failure count increases.
    #[test]
    fn timeout_reassigns_blocks() {
        let mut mgr = SyncManager::new();
        let p = peer(1);

        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));

        let h0 = sample_header(0, Hash256::ZERO);
        let hash = h0.hash();
        mgr.on_headers_received(vec![h0]);

        // Assign the block via next_actions.
        mgr.next_actions(0, || vec![]);

        // Manually set last_request_at to a time well in the past.
        if let Some(ps) = mgr.peers.get_mut(&p) {
            ps.last_request_at = Some(
                std::time::Instant::now()
                    - std::time::Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS + 5),
            );
        }

        mgr.check_timeouts();

        // Block should be back in queue.
        assert!(
            mgr.blocks_to_download.contains(&hash),
            "timed-out hash should return to download queue"
        );
        assert!(!mgr.in_flight.contains_key(&hash), "hash should not be in-flight");

        // Peer failure count should be 1.
        assert_eq!(mgr.peers[&p].failures, 1);
    }

    /// A peer is banned after exceeding the maximum failure threshold.
    #[test]
    fn ban_after_max_failures() {
        let mut mgr = SyncManager::new();
        let p = peer(1);

        mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));

        // Manually drive failures to the limit via repeated timeout cycles.
        for _ in 0..DEFAULT_MAX_FAILURES {
            let h = BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 1_700_000_000,
                difficulty_target: u64::MAX,
                nonce: rand_nonce(),
            };
            let hash = h.hash();

            // Put the hash directly into in-flight so check_timeouts picks it up.
            mgr.in_flight.insert(hash, p);
            if let Some(ps) = mgr.peers.get_mut(&p) {
                ps.in_flight += 1;
                ps.last_request_at = Some(
                    std::time::Instant::now()
                        - std::time::Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS + 5),
                );
            }

            mgr.check_timeouts();
        }

        assert!(mgr.peers[&p].banned, "peer should be banned");

        // Now queue blocks and verify the banned peer gets no assignments.
        let h0 = sample_header(0, Hash256::ZERO);
        mgr.on_headers_received(vec![h0]);

        let actions = mgr.next_actions(0, || vec![]);

        // All RequestBlock actions should NOT go to the banned peer.
        for action in &actions {
            if let SyncAction::RequestBlock { peer: assigned, .. } = action {
                assert_ne!(
                    *assigned, p,
                    "banned peer should not receive block requests"
                );
            }
        }
    }

    /// `next_action()` returns the same first element as `next_actions()`.
    #[test]
    fn compat_shim_returns_first_action() {
        let mut mgr1 = SyncManager::new();
        let mut mgr2 = SyncManager::new();

        let p = peer(1);
        // Both managers in DiscoveringPeers state with a known peer.
        for mgr in [&mut mgr1, &mut mgr2] {
            mgr.on_peer_connected(p);
            mgr.on_peer_tip(p, 10, Hash256([0xAA; 32]));
        }

        let first_of_actions = mgr1.next_actions(0, || vec![]).into_iter().next();
        let shim_action = mgr2.next_action(0, || vec![]);

        // Both should be RequestChainTip.
        assert!(
            matches!(first_of_actions, Some(SyncAction::RequestChainTip(_))),
            "next_actions first element should be RequestChainTip"
        );
        assert!(
            matches!(shim_action, SyncAction::RequestChainTip(_)),
            "next_action shim should be RequestChainTip"
        );
    }

    /// best_peer is updated when a peer with a higher tip is seen.
    #[test]
    fn best_peer_updates_on_higher_tip() {
        let mut mgr = SyncManager::new();

        let p1 = peer(1);
        let p2 = peer(2);

        mgr.on_peer_tip(p1, 10, Hash256([0x01; 32]));
        assert_eq!(mgr.best_peer.as_ref().map(|(id, _)| *id), Some(p1));
        assert_eq!(mgr.best_peer.as_ref().map(|(_, t)| t.height), Some(10));

        mgr.on_peer_tip(p2, 20, Hash256([0x02; 32]));
        assert_eq!(
            mgr.best_peer.as_ref().map(|(id, _)| *id),
            Some(p2),
            "best_peer should switch to p2 with higher tip"
        );
        assert_eq!(mgr.best_peer.as_ref().map(|(_, t)| t.height), Some(20));
    }

    /// Helper: produce a unique nonce for each synthetic block header in tests.
    fn rand_nonce() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(42)
    }
}
