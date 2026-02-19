//! Per-address and per-IP rate limiting for faucet requests.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Tracks last-claim timestamps per address and IP to enforce cooldowns.
pub struct RateLimiter {
    addresses: HashMap<String, Instant>,
    ips: HashMap<IpAddr, Instant>,
    cooldown: Duration,
}

impl RateLimiter {
    pub fn new(cooldown: Duration) -> Self {
        Self {
            addresses: HashMap::new(),
            ips: HashMap::new(),
            cooldown,
        }
    }

    /// Check whether `address` and `ip` are eligible for a claim.
    ///
    /// Returns `Ok(())` if the claim is allowed, or `Err` with a human-readable
    /// reason and the number of seconds remaining in the cooldown.
    pub fn check(&self, address: &str, ip: IpAddr) -> Result<(), (String, u64)> {
        if let Some(last) = self.addresses.get(address) {
            let elapsed = last.elapsed();
            if elapsed < self.cooldown {
                let remaining = (self.cooldown - elapsed).as_secs();
                return Err((
                    format!("Address already claimed. Try again in {remaining}s."),
                    remaining,
                ));
            }
        }
        // Only apply IP rate limit for routable (non-loopback) addresses.
        if !ip.is_loopback() && !ip.is_unspecified() {
            if let Some(last) = self.ips.get(&ip) {
                let elapsed = last.elapsed();
                if elapsed < self.cooldown {
                    let remaining = (self.cooldown - elapsed).as_secs();
                    return Err((
                        format!("IP already claimed. Try again in {remaining}s."),
                        remaining,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Record a successful claim for `address` and `ip`.
    pub fn record(&mut self, address: &str, ip: IpAddr) {
        let now = Instant::now();
        self.addresses.insert(address.to_string(), now);
        self.ips.insert(ip, now);
    }

    /// Seconds remaining in the cooldown for `address`, or 0 if eligible.
    pub fn seconds_remaining_for_address(&self, address: &str) -> u64 {
        self.addresses
            .get(address)
            .and_then(|last| {
                let elapsed = last.elapsed();
                if elapsed < self.cooldown {
                    Some((self.cooldown - elapsed).as_secs())
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }
}
