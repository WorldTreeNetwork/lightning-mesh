use std::collections::BTreeMap;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;

/// A mesh-wide service announcement (mDNS-style).
///
/// Keyed by service name at `/services/{name}`. Service expires when the
/// associated device lease (identified by `host_mac`) expires.
///
/// Merge is last-writer-wins on `updated_at` (see [`merge_service`]). Like
/// [`UserEntry`], a service record has no single authoritative announcer — any
/// node that ingests a service advertisement can write it — so LWW with an HLC
/// tie-break (wall_clock → counter → node_id) is what keeps two nodes
/// convergent without a conflict arm.
///
/// Uses `BTreeMap` instead of `HashMap` for deterministic serialization order,
/// which makes postcard round-trip equality straightforward.
///
/// [`UserEntry`]: crate::crdt::users::UserEntry
/// [`merge_service`]: crate::crdt::merge::merge_service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub hostname: String,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: String,
    pub txt: BTreeMap<String, String>,
    pub host_mac: [u8; 6],
    /// Hybrid logical clock stamp; newest wins on merge.
    pub updated_at: HLC,
}

/// Mesh-wide service directory: service name → most recent record.
///
/// The key is the fully-qualified service name (e.g. `printer._ipp._tcp`);
/// callers enforce that the map key matches the record they insert (as with
/// [`UserBook`](crate::crdt::users::UserBook)).
pub type ServiceBook = BTreeMap<String, ServiceEntry>;

/// Well-known service names that can never be claimed in the `/services/`
/// directory (bead e21.2.1). Matched case-insensitively — names are
/// normalized to lowercase before comparison (see [`is_reserved_service_name`]).
///
/// Shared across the owner-bound merge guard (S2.1), the gossip apply path
/// (S2.2), and the publish surface (S3.1) so all three enforce the same list.
pub const RESERVED_SERVICE_NAMES: &[&str] = &["hello", "id"];

/// True if `name`, compared case-insensitively, is one of
/// [`RESERVED_SERVICE_NAMES`].
pub fn is_reserved_service_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    RESERVED_SERVICE_NAMES.iter().any(|reserved| *reserved == lower)
}

/// Owner-bound service entry (v2, bead e21.2.1) — the upgrade over
/// [`ServiceEntry`] (v1, bead 7jb).
///
/// v1 is pure LWW with no single authoritative announcer. v2 introduces an
/// owning node per service name: the *same* owner may freely refresh its
/// entry (LWW on `updated_at`), but a *different* owner claiming the same
/// name is a conflict resolved first-writer-wins on `first_claimed_at` — the
/// HLC of the *original* claim, which a refresh never changes. This is a
/// deliberate semantics change from v1's cross-owner LWW (PRD FR20 / ADR):
/// a service name is claimed on first sight (owner-bound TOFU), not
/// re-claimable by whoever gossips last.
///
/// The service name itself is not stored on the entry; it is the map key in
/// [`ServiceBookV2`], matching v1's [`ServiceBook`] convention.
///
/// See [`merge_service_v2`](crate::crdt::merge::merge_service_v2) for the
/// merge semantics and [`RESERVED_SERVICE_NAMES`] for names that are always
/// rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEntryV2 {
    /// iroh node id of the claiming/owning node (same encoding as
    /// [`PeerAddrEntry::node_id`](crate::crdt::peer_addr::PeerAddrEntry::node_id)).
    pub owner_node_id: String,
    /// HLC of the original claim. Never updated on refresh; this is what
    /// arbitrates cross-owner conflicts (first-writer-wins).
    pub first_claimed_at: HLC,
    /// HLC of the most recent refresh by the owner. Newer wins the
    /// same-owner LWW comparison.
    pub updated_at: HLC,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: String,
    pub txt: BTreeMap<String, String>,
    pub host_mac: Option<[u8; 6]>,
}

/// Mesh-wide v2 service directory: service name → most recent owner-bound
/// record. Same key convention as [`ServiceBook`].
pub type ServiceBookV2 = BTreeMap<String, ServiceEntryV2>;

/// Tombstone recording that `owner_node_id` unpublished a v2 service name at
/// `hlc` (bead e21.2.2, decision D-004).
///
/// Tombstones are retained indefinitely once written — GC is deferred to
/// bead 99f, so unbounded retention is accepted for now — and gate future
/// publishes to the same name via
/// [`apply_service_publish_v2`](crate::crdt::service_apply::apply_service_publish_v2):
/// a publish older than the tombstone's `hlc` loses (FR31), and only the
/// SAME `owner_node_id` publishing with a newer `hlc` than the tombstone may
/// revive the name. A different owner cannot claim a tombstoned name until
/// the tombstone is GC'd — the owner-bound TOFU model from v2's merge
/// semantics extends past unpublish, not just past publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceTombstone {
    pub owner_node_id: String,
    pub hlc: HLC,
}

/// Tombstone store keyed by service name, same convention as
/// [`ServiceBookV2`].
pub type ServiceTombstoneBook = BTreeMap<String, ServiceTombstone>;

/// Local-only bookkeeping (bead e21.2.4, FR32/FR34): recorded whenever a
/// merge [`Conflict`](crate::crdt::merge::MergeResult::Conflict) makes THIS
/// node the loser for a service name — i.e. some other node's claim on the
/// name is first-writer-wins-earlier than ours. Never gossiped (it is derived
/// purely from local merge outcomes, and every node reaches the same verdict
/// independently from the same CRDT data); persisted alongside the v2
/// book/tombstones purely so a restart doesn't forget a name is lost and
/// briefly allow a doomed republish.
///
/// Gates future local publish attempts to the same name
/// ([`publish_service_v2`](crate::crdt::service_apply::publish_service_v2))
/// so they fail synchronously naming the winner (FR34) instead of silently
/// losing another conflict round-trip, and is kept accessible for a future
/// status/API surface (FR32; not wired to `status` output by this story).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LostName {
    pub winner_node_id: String,
    /// The winner's `first_claimed_at` HLC — the arbitration clock, not
    /// necessarily its latest refresh.
    pub hlc: HLC,
}

/// Lost-name map keyed by service name, same convention as [`ServiceBookV2`].
pub type LostNameMap = BTreeMap<String, LostName>;

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::net::Ipv4Addr;

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
        let mut txt = BTreeMap::new();
        txt.insert("path".to_string(), "/ipp/print".to_string());
        txt.insert("version".to_string(), "2.0".to_string());

        let original = ServiceEntry {
            hostname: "printer".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            port: 631,
            protocol: "_ipp._tcp".to_string(),
            txt,
            host_mac: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01],
            updated_at: hlc(1_700_000_001_000, 0, "router-a"),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn postcard_roundtrip_no_txt() {
        let original = ServiceEntry {
            hostname: "nas".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 60)),
            port: 445,
            protocol: "_smb._tcp".to_string(),
            txt: BTreeMap::new(),
            host_mac: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
            updated_at: hlc(1_700_000_002_000, 3, "router-b"),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    // --- ServiceEntryV2 (bead e21.2.1) ---

    fn v2_entry(owner: &str, wall_clock: u64, counter: u32, node_id: &str) -> ServiceEntryV2 {
        let mut txt = BTreeMap::new();
        txt.insert("path".to_string(), "/ipp/print".to_string());
        txt.insert("version".to_string(), "2.0".to_string());
        ServiceEntryV2 {
            owner_node_id: owner.to_string(),
            first_claimed_at: hlc(wall_clock, counter, node_id),
            updated_at: hlc(wall_clock, counter, node_id),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            port: 631,
            protocol: "_ipp._tcp".to_string(),
            txt,
            host_mac: Some([0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]),
        }
    }

    #[test]
    fn v2_postcard_roundtrip() {
        let original = v2_entry("router-a-node-id", 1_700_000_001_000, 0, "router-a-node-id");
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceEntryV2 = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn v2_postcard_roundtrip_no_txt_no_mac() {
        let original = ServiceEntryV2 {
            owner_node_id: "router-b-node-id".to_string(),
            first_claimed_at: hlc(1_700_000_000_000, 0, "router-b-node-id"),
            updated_at: hlc(1_700_000_002_000, 3, "router-b-node-id"),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 60)),
            port: 445,
            protocol: "_smb._tcp".to_string(),
            txt: BTreeMap::new(),
            host_mac: None,
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceEntryV2 = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    // --- ServiceTombstone (bead e21.2.2) ---

    #[test]
    fn tombstone_postcard_roundtrip() {
        let original = ServiceTombstone {
            owner_node_id: "router-a-node-id".to_string(),
            hlc: hlc(1_700_000_020_000, 0, "router-a-node-id"),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: ServiceTombstone = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn reserved_names_are_case_insensitive() {
        assert!(is_reserved_service_name("hello"));
        assert!(is_reserved_service_name("Hello"));
        assert!(is_reserved_service_name("HELLO"));
        assert!(is_reserved_service_name("id"));
        assert!(is_reserved_service_name("ID"));
        assert!(!is_reserved_service_name("printer"));
        assert!(!is_reserved_service_name("hello2"));
    }
}
