use std::cmp::Ordering;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Hybrid Logical Clock timestamp.
///
/// Ordering: wall_clock → counter → node_id (lexicographic).
/// Lower HLC = earlier writer; used for first-writer-wins conflict resolution.
/// Last-writer-wins lanes compare the same way (greater HLC wins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HLC {
    /// Wall clock time in milliseconds since Unix epoch.
    pub wall_clock: u64,
    /// Monotonic counter, incremented when wall_clock does not advance.
    pub counter: u32,
    /// Node ID of the router that generated this timestamp.
    pub node_id: String,
}

/// Per-node HLC. `tick` issues the next stamp; `observe` folds in a peer's
/// stamp so the next tick is strictly later than anything we have seen.
///
/// Counter is the affordance for two writes in the same millisecond: without
/// it, `now_hlc` with `counter: 0` produces equal stamps and LWW treats the
/// second write as `Unchanged`.
#[derive(Debug, Clone)]
pub struct HlcClock {
    node_id: String,
    last: HLC,
}

impl HlcClock {
    pub fn new(node_id: impl Into<String>) -> Self {
        let node_id = node_id.into();
        Self {
            last: HLC {
                wall_clock: 0,
                counter: 0,
                node_id: node_id.clone(),
            },
            node_id,
        }
    }

    pub fn tick(&mut self) -> HLC {
        self.tick_at(physical_ms())
    }

    pub fn tick_at(&mut self, physical_ms: u64) -> HLC {
        if physical_ms > self.last.wall_clock {
            self.last.wall_clock = physical_ms;
            self.last.counter = 0;
        } else {
            self.last.counter = self.last.counter.saturating_add(1);
        }
        self.last.node_id = self.node_id.clone();
        self.last.clone()
    }

    /// Advance so the next [`tick`] is after `other` (and after physical time).
    pub fn observe(&mut self, other: &HLC) {
        self.observe_at(other, physical_ms());
    }

    pub fn observe_at(&mut self, other: &HLC, physical_ms: u64) {
        if physical_ms > self.last.wall_clock && physical_ms > other.wall_clock {
            self.last.wall_clock = physical_ms;
            self.last.counter = 0;
        } else if other.wall_clock > self.last.wall_clock {
            self.last.wall_clock = other.wall_clock;
            self.last.counter = other.counter.saturating_add(1);
        } else if self.last.wall_clock > other.wall_clock {
            self.last.counter = self.last.counter.saturating_add(1);
        } else {
            self.last.counter = self.last.counter.max(other.counter).saturating_add(1);
        }
        self.last.node_id = self.node_id.clone();
    }

    pub fn last(&self) -> &HLC {
        &self.last
    }
}

fn physical_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static PROCESS_CLOCK: Mutex<Option<HlcClock>> = Mutex::new(None);

/// Process-wide [`HlcClock::tick`]. All meshd CRDT writers should use this
/// (or a local `HlcClock`) instead of stamping `counter: 0`.
pub fn tick_hlc(node_id: &str) -> HLC {
    let mut guard = PROCESS_CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut() {
        Some(clock) if clock.node_id == node_id => clock.tick(),
        _ => {
            let mut clock = HlcClock::new(node_id);
            let stamped = clock.tick();
            *guard = Some(clock);
            stamped
        }
    }
}

/// Fold a peer stamp into the process clock. Next [`tick_hlc`] is later.
pub fn observe_hlc(other: &HLC) {
    let mut guard = PROCESS_CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(clock) = guard.as_mut() {
        clock.observe(other);
    }
}

impl Ord for HLC {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall_clock
            .cmp(&other.wall_clock)
            .then(self.counter.cmp(&other.counter))
            .then(self.node_id.cmp(&other.node_id))
    }
}

impl PartialOrd for HLC {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hlc(wall_clock: u64, counter: u32, node_id: &str) -> HLC {
        HLC {
            wall_clock,
            counter,
            node_id: node_id.to_string(),
        }
    }

    #[test]
    fn postcard_roundtrip() {
        let original = hlc(1_700_000_000_000, 42, "router-a");
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: HLC = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn ord_wall_clock() {
        assert!(hlc(1000, 0, "a") < hlc(1001, 0, "a"));
    }

    #[test]
    fn ord_counter_breaks_tie() {
        assert!(hlc(1000, 0, "a") < hlc(1000, 1, "a"));
    }

    #[test]
    fn ord_node_id_breaks_tie() {
        assert!(hlc(1000, 0, "a") < hlc(1000, 0, "b"));
    }

    #[test]
    fn tick_same_millisecond_bumps_counter() {
        let mut c = HlcClock::new("me");
        let a = c.tick_at(1000);
        let b = c.tick_at(1000);
        assert_eq!(a.wall_clock, 1000);
        assert_eq!(a.counter, 0);
        assert_eq!(b.counter, 1);
        assert!(a < b);
    }

    #[test]
    fn tick_later_wall_resets_counter() {
        let mut c = HlcClock::new("me");
        let _ = c.tick_at(1000);
        let _ = c.tick_at(1000);
        let c3 = c.tick_at(1001);
        assert_eq!(c3.wall_clock, 1001);
        assert_eq!(c3.counter, 0);
    }

    #[test]
    fn observe_peer_then_tick_is_strictly_later() {
        let mut c = HlcClock::new("me");
        let _ = c.tick_at(1000);
        c.observe_at(&hlc(5000, 9, "peer"), 1000);
        let next = c.tick_at(1000);
        assert!(next > hlc(5000, 9, "peer"));
        assert_eq!(next.node_id, "me");
    }
}
