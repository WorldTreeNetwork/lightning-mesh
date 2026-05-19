//! Tracks the first-boot cooldown window during which a router waits for a
//! local peer to appear before publishing a subnet claim.

use std::time::{Duration, Instant};

/// Default cooldown window: 10 seconds. Matches the local-peer detection
/// window in network-architecture.md.
pub const DEFAULT_COOLDOWN: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct ClaimCooldown {
    started_at: Instant,
    duration: Duration,
    peer_detected: bool,
}

impl ClaimCooldown {
    pub fn new(duration: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            duration,
            peer_detected: false,
        }
    }

    pub fn default_cooldown() -> Self {
        Self::new(DEFAULT_COOLDOWN)
    }

    /// Signal that a local peer was detected. The router must abandon
    /// its claim plan and join the existing site instead.
    pub fn note_peer_detected(&mut self) {
        self.peer_detected = true;
    }

    pub fn peer_detected(&self) -> bool {
        self.peer_detected
    }

    /// Whether the cooldown has elapsed *and* no peer was detected.
    /// Caller should publish the claim only when this returns true.
    pub fn should_publish(&self) -> bool {
        !self.peer_detected && self.started_at.elapsed() >= self.duration
    }

    /// Whether the router should abandon its claim plan — peer detected.
    pub fn should_abort(&self) -> bool {
        self.peer_detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_publish_after_window_no_peer() {
        let cd = ClaimCooldown::new(Duration::from_millis(20));
        assert!(!cd.should_publish());
        std::thread::sleep(Duration::from_millis(25));
        assert!(cd.should_publish());
        assert!(!cd.should_abort());
    }

    #[test]
    fn peer_detected_aborts() {
        let mut cd = ClaimCooldown::new(Duration::from_millis(20));
        cd.note_peer_detected();
        std::thread::sleep(Duration::from_millis(25));
        assert!(!cd.should_publish());
        assert!(cd.should_abort());
    }

    #[test]
    fn no_publish_before_window() {
        let cd = ClaimCooldown::new(Duration::from_secs(10));
        assert!(!cd.should_publish());
    }
}
