use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;

/// A self-announced human router name, keyed by node id (bead mjolnir-mesh-t7i).
///
/// Only the subject node announces its own name (`node_id` == announcer), so
/// merge is pure last-writer-wins on `announced_at` — no conflict arm, exactly
/// like [`PeerAddrEntry`]. Lets an operator give each router a human handle
/// ("attic", "garage") that propagates mesh-wide over gossip and shows up in
/// the front-desk directory / topology instead of a bare 64-hex node id.
///
/// Stored at `/nodenames/{node_id}` in the CRDT node-name book.
///
/// [`PeerAddrEntry`]: crate::crdt::peer_addr::PeerAddrEntry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeNameEntry {
    /// 64-hex iroh node id (subject == announcer).
    pub node_id: String,
    /// Operator-chosen human name for this node.
    pub name: String,
    pub announced_at: HLC,
}

/// Mesh-wide node-name book: node_id → most recent self-announced name.
///
/// The key must equal `entry.node_id`; the apply path enforces that invariant
/// (a node only announces its OWN name), as with
/// [`AddrBook`](crate::crdt::peer_addr::AddrBook).
pub type NodeNameBook = BTreeMap<String, NodeNameEntry>;

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
        let original = NodeNameEntry {
            node_id: "abcd1234".repeat(8),
            name: "attic".to_string(),
            announced_at: hlc(1_700_000_001_000, 0, "abcd1234abcd1234"),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: NodeNameEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn postcard_roundtrip_empty_name() {
        let original = NodeNameEntry {
            node_id: "deadbeef".repeat(8),
            name: String::new(),
            announced_at: hlc(1_700_000_002_000, 1, "deadbeef"),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: NodeNameEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
