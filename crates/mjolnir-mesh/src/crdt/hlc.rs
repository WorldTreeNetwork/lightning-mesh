use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// Hybrid Logical Clock timestamp.
///
/// Ordering: wall_clock → counter → node_id (lexicographic).
/// Lower HLC = earlier writer; used for first-writer-wins conflict resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HLC {
    /// Wall clock time in milliseconds since Unix epoch.
    pub wall_clock: u64,
    /// Monotonic counter, incremented when wall_clock equals last observed max.
    pub counter: u32,
    /// Node ID of the router that generated this timestamp.
    pub node_id: String,
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
}
