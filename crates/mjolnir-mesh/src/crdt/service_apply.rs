//! Lib-side apply layer for v2 service gossip (bead e21.2.2).
//!
//! This is the seam S2.3 (the daemon dispatch arm) calls into: given a
//! decoded `GossipMessage::ServicePublishV2` / `ServiceUnpublishV2` payload
//! plus the current [`ServiceBookV2`] and [`ServiceTombstoneBook`], compute
//! the merge/tombstone outcome and mutate both stores accordingly. Pure and
//! transport-free, same seam shape as [`crate::crdt::merge`].
//!
//! Tombstone semantics (FR31, decision D-004):
//! - A publish for a name with no tombstone and no live entry is a normal
//!   [`merge_service_v2`] insert.
//! - A publish for a name that already has a live entry is a normal
//!   [`merge_service_v2`] call against that entry (the tombstone, if any, is
//!   stale — the name was already revived or never actually vacated).
//! - A publish for a name that is tombstoned (no live entry) only succeeds
//!   if the publisher is the tombstone's own owner AND the publish's
//!   `updated_at` is newer than the tombstone's `hlc` — this is the FR31
//!   "revive" path. Any other publish against a tombstoned, vacant name is
//!   rejected: an older HLC from the same owner is a stale replay, and a
//!   different owner cannot claim the name until the tombstone is GC'd
//!   (deferred, bead 99f) — the owner-bound TOFU model extends past
//!   unpublish.
//! - An unpublish only takes effect if its `owner_node_id` matches the live
//!   entry's owner (a non-owner tombstone is ignored — conflicting owner
//!   claims go through [`merge_service_v2`], not unpublish). If there is no
//!   live entry, the message either refreshes an existing tombstone from the
//!   same owner (HLC-ordered, newer wins) or, if no tombstone exists yet,
//!   records a fresh one.

use crate::crdt::merge::{merge_service_v2, MergeResult, ReservedServiceName};
use crate::crdt::service::{ServiceBookV2, ServiceEntryV2, ServiceTombstone, ServiceTombstoneBook};

/// Outcome of applying a `ServicePublishV2` message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// Applied via the normal owner-bound merge (name had a live entry, or
    /// had neither a live entry nor a tombstone).
    Merged(Box<MergeResult<ServiceEntryV2>>),
    /// The name was tombstoned and this publish revived it (same owner,
    /// newer HLC than the tombstone).
    Revived,
    /// The name is tombstoned and this publish does not qualify to revive
    /// it — either a stale replay from the tombstone's own owner, or a
    /// different owner attempting to claim a vacant-but-tombstoned name.
    RejectedByTombstone,
}

/// Outcome of applying a `ServiceUnpublishV2` message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpublishOutcome {
    /// The live entry (owned by the sender) was removed and a tombstone
    /// recorded.
    Unpublished,
    /// No live entry existed; recorded a new tombstone for the sender.
    TombstoneRecorded,
    /// No live entry existed, but a tombstone already existed from the same
    /// owner with an equal-or-newer HLC — this message is a stale replay.
    Unchanged,
    /// Ignored: the sender does not own the live entry (or, when there is
    /// no live entry, does not own the existing tombstone).
    IgnoredNotOwner,
}

/// Apply an incoming `ServicePublishV2` (`name`, `incoming`) to `book`,
/// consulting/updating `tombstones` per the tombstone-vs-publish rules
/// above. Mutates `book` in place when the merge/revival succeeds.
///
/// Reserved-name rejection (shared with [`merge_service_v2`]) is surfaced
/// as `Err` before any tombstone logic runs.
pub fn apply_service_publish_v2(
    book: &mut ServiceBookV2,
    tombstones: &ServiceTombstoneBook,
    name: &str,
    incoming: ServiceEntryV2,
) -> Result<PublishOutcome, ReservedServiceName> {
    if let Some(local) = book.get(name) {
        // Live entry present: tombstone (if any) is moot, go through the
        // normal owner-bound merge.
        let result = merge_service_v2(name, Some(local), &incoming)?;
        apply_merge_result(book, name, &incoming, &result);
        return Ok(PublishOutcome::Merged(Box::new(result)));
    }

    match tombstones.get(name) {
        None => {
            let result = merge_service_v2(name, None, &incoming)?;
            apply_merge_result(book, name, &incoming, &result);
            Ok(PublishOutcome::Merged(Box::new(result)))
        }
        Some(tombstone) => {
            if incoming.owner_node_id == tombstone.owner_node_id
                && incoming.updated_at > tombstone.hlc
            {
                book.insert(name.to_string(), incoming);
                Ok(PublishOutcome::Revived)
            } else {
                Ok(PublishOutcome::RejectedByTombstone)
            }
        }
    }
}

fn apply_merge_result(
    book: &mut ServiceBookV2,
    name: &str,
    incoming: &ServiceEntryV2,
    result: &MergeResult<ServiceEntryV2>,
) {
    match result {
        MergeResult::Inserted | MergeResult::Updated => {
            book.insert(name.to_string(), incoming.clone());
        }
        MergeResult::Conflict { winner, .. } => {
            book.insert(name.to_string(), winner.clone());
        }
        MergeResult::Unchanged => {}
    }
}

/// Apply an incoming `ServiceUnpublishV2` (`name`, `owner_node_id`, `hlc`) to
/// `book` and `tombstones`.
pub fn apply_service_unpublish_v2(
    book: &mut ServiceBookV2,
    tombstones: &mut ServiceTombstoneBook,
    name: &str,
    owner_node_id: &str,
    hlc: crate::crdt::hlc::HLC,
) -> UnpublishOutcome {
    if let Some(local) = book.get(name) {
        if local.owner_node_id != owner_node_id {
            return UnpublishOutcome::IgnoredNotOwner;
        }
        book.remove(name);
        tombstones.insert(
            name.to_string(),
            ServiceTombstone { owner_node_id: owner_node_id.to_string(), hlc },
        );
        return UnpublishOutcome::Unpublished;
    }

    match tombstones.get(name) {
        None => {
            tombstones.insert(
                name.to_string(),
                ServiceTombstone { owner_node_id: owner_node_id.to_string(), hlc },
            );
            UnpublishOutcome::TombstoneRecorded
        }
        Some(existing) => {
            if existing.owner_node_id != owner_node_id {
                return UnpublishOutcome::IgnoredNotOwner;
            }
            if hlc > existing.hlc {
                tombstones.insert(
                    name.to_string(),
                    ServiceTombstone { owner_node_id: owner_node_id.to_string(), hlc },
                );
                UnpublishOutcome::TombstoneRecorded
            } else {
                UnpublishOutcome::Unchanged
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;
    use crate::crdt::hlc::HLC;

    fn hlc(wall_clock: u64, counter: u32, node_id: &str) -> HLC {
        HLC { wall_clock, counter, node_id: node_id.to_string() }
    }

    fn entry(owner: &str, wall_clock: u64, counter: u32, node_id: &str) -> ServiceEntryV2 {
        ServiceEntryV2 {
            owner_node_id: owner.to_string(),
            first_claimed_at: hlc(wall_clock, counter, node_id),
            updated_at: hlc(wall_clock, counter, node_id),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
            port: 631,
            protocol: "_ipp._tcp".to_string(),
            txt: BTreeMap::new(),
            host_mac: None,
        }
    }

    // --- publish: no tombstone, no local entry ---

    #[test]
    fn publish_inserted_when_no_local_no_tombstone() {
        let mut book = ServiceBookV2::new();
        let tombstones = ServiceTombstoneBook::new();
        let incoming = entry("router-a", 1_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming.clone()).unwrap();
        assert_eq!(result, PublishOutcome::Merged(Box::new(MergeResult::Inserted)));
        assert_eq!(book.get("printer"), Some(&incoming));
    }

    // --- publish: live entry present (tombstone, if any, is moot) ---

    #[test]
    fn publish_merges_normally_when_live_entry_present() {
        let mut book = ServiceBookV2::new();
        let local = entry("router-a", 1_000, 0, "router-a");
        book.insert("printer".to_string(), local);
        let tombstones = ServiceTombstoneBook::new();

        let incoming = entry("router-a", 2_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming.clone()).unwrap();
        assert_eq!(result, PublishOutcome::Merged(Box::new(MergeResult::Updated)));
        assert_eq!(book.get("printer"), Some(&incoming));
    }

    #[test]
    fn publish_unchanged_ignored_when_live_entry_present_and_older() {
        let mut book = ServiceBookV2::new();
        let local = entry("router-a", 2_000, 0, "router-a");
        book.insert("printer".to_string(), local.clone());
        let tombstones = ServiceTombstoneBook::new();

        let incoming = entry("router-a", 1_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming).unwrap();
        assert_eq!(result, PublishOutcome::Merged(Box::new(MergeResult::Unchanged)));
        assert_eq!(book.get("printer"), Some(&local));
    }

    // --- publish: tombstoned name, vacant (no live entry) ---

    #[test]
    fn publish_older_than_tombstone_rejected_same_owner() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") },
        );

        // Same owner, but stale HLC (older than the tombstone) — replay, rejected.
        let incoming = entry("router-a", 1_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming).unwrap();
        assert_eq!(result, PublishOutcome::RejectedByTombstone);
        assert!(book.get("printer").is_none());
    }

    #[test]
    fn publish_equal_to_tombstone_hlc_rejected() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") },
        );

        let incoming = entry("router-a", 2_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming).unwrap();
        assert_eq!(result, PublishOutcome::RejectedByTombstone);
    }

    #[test]
    fn publish_newer_than_tombstone_same_owner_revives() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") },
        );

        let incoming = entry("router-a", 3_000, 0, "router-a");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming.clone()).unwrap();
        assert_eq!(result, PublishOutcome::Revived);
        assert_eq!(book.get("printer"), Some(&incoming));
    }

    #[test]
    fn publish_newer_than_tombstone_different_owner_rejected() {
        // A different owner cannot claim a tombstoned name, even with a
        // newer HLC than the tombstone — only the tombstone's own owner may
        // revive; GC (deferred, 99f) is what eventually reopens the name.
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") },
        );

        let incoming = entry("router-b", 9_000, 0, "router-b");
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming).unwrap();
        assert_eq!(result, PublishOutcome::RejectedByTombstone);
        assert!(book.get("printer").is_none());
    }

    #[test]
    fn publish_reserved_name_rejected_even_with_tombstone() {
        let mut book = ServiceBookV2::new();
        let tombstones = ServiceTombstoneBook::new();
        let incoming = entry("router-a", 1_000, 0, "router-a");
        let err = apply_service_publish_v2(&mut book, &tombstones, "hello", incoming).unwrap_err();
        assert_eq!(err, ReservedServiceName("hello".to_string()));
    }

    // --- publish: conflict path (different owner, no tombstone, no local -> handled above as Inserted;
    // different owner WITH local entry goes through merge_service_v2's Conflict arm) ---

    #[test]
    fn publish_conflict_installs_the_merge_winner() {
        let mut book = ServiceBookV2::new();
        let local = entry("router-a", 1_000, 0, "router-a"); // earlier first_claimed_at
        book.insert("printer".to_string(), local.clone());
        let tombstones = ServiceTombstoneBook::new();

        let incoming = entry("router-b", 2_000, 0, "router-b"); // later first_claimed_at, loses
        let result = apply_service_publish_v2(&mut book, &tombstones, "printer", incoming.clone()).unwrap();
        match result {
            PublishOutcome::Merged(ref boxed) if matches!(**boxed, MergeResult::Conflict { .. }) => {
                let MergeResult::Conflict { ref winner, ref loser } = **boxed else { unreachable!() };
                assert_eq!(winner.owner_node_id, "router-a");
                assert_eq!(loser.owner_node_id, "router-b");
            }
            other => panic!("expected Merged(Conflict), got {:?}", other),
        }
        // The book keeps the winner (the original owner), not the incoming loser.
        assert_eq!(book.get("printer"), Some(&local));
    }

    // --- unpublish: owner matches live entry ---

    #[test]
    fn unpublish_by_owner_removes_entry_and_tombstones() {
        let mut book = ServiceBookV2::new();
        book.insert("printer".to_string(), entry("router-a", 1_000, 0, "router-a"));
        let mut tombstones = ServiceTombstoneBook::new();

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-a", hlc(2_000, 0, "router-a"));
        assert_eq!(result, UnpublishOutcome::Unpublished);
        assert!(book.get("printer").is_none());
        assert_eq!(
            tombstones.get("printer"),
            Some(&ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") })
        );
    }

    #[test]
    fn unpublish_by_non_owner_of_live_entry_ignored() {
        let mut book = ServiceBookV2::new();
        book.insert("printer".to_string(), entry("router-a", 1_000, 0, "router-a"));
        let mut tombstones = ServiceTombstoneBook::new();

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-b", hlc(2_000, 0, "router-b"));
        assert_eq!(result, UnpublishOutcome::IgnoredNotOwner);
        // Neither the book nor the tombstone store is touched.
        assert!(book.get("printer").is_some());
        assert!(tombstones.get("printer").is_none());
    }

    // --- unpublish: no live entry ---

    #[test]
    fn unpublish_with_no_live_entry_and_no_tombstone_records_one() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-a", hlc(1_000, 0, "router-a"));
        assert_eq!(result, UnpublishOutcome::TombstoneRecorded);
        assert_eq!(
            tombstones.get("printer"),
            Some(&ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(1_000, 0, "router-a") })
        );
    }

    #[test]
    fn unpublish_refresh_from_same_owner_updates_tombstone_hlc() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(1_000, 0, "router-a") },
        );

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-a", hlc(2_000, 0, "router-a"));
        assert_eq!(result, UnpublishOutcome::TombstoneRecorded);
        assert_eq!(tombstones.get("printer").unwrap().hlc, hlc(2_000, 0, "router-a"));
    }

    #[test]
    fn unpublish_stale_replay_from_same_owner_is_unchanged() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(2_000, 0, "router-a") },
        );

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-a", hlc(1_000, 0, "router-a"));
        assert_eq!(result, UnpublishOutcome::Unchanged);
        assert_eq!(tombstones.get("printer").unwrap().hlc, hlc(2_000, 0, "router-a"));
    }

    #[test]
    fn unpublish_from_different_owner_than_existing_tombstone_ignored() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();
        tombstones.insert(
            "printer".to_string(),
            ServiceTombstone { owner_node_id: "router-a".to_string(), hlc: hlc(1_000, 0, "router-a") },
        );

        let result = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-b", hlc(9_000, 0, "router-b"));
        assert_eq!(result, UnpublishOutcome::IgnoredNotOwner);
        assert_eq!(tombstones.get("printer").unwrap().owner_node_id, "router-a");
    }

    // --- full lifecycle: publish -> unpublish -> revive ---

    #[test]
    fn full_lifecycle_publish_unpublish_revive() {
        let mut book = ServiceBookV2::new();
        let mut tombstones = ServiceTombstoneBook::new();

        // 1. First publish.
        let published = entry("router-a", 1_000, 0, "router-a");
        let r1 = apply_service_publish_v2(&mut book, &tombstones, "printer", published).unwrap();
        assert_eq!(r1, PublishOutcome::Merged(Box::new(MergeResult::Inserted)));

        // 2. Owner unpublishes.
        let r2 = apply_service_unpublish_v2(&mut book, &mut tombstones, "printer", "router-a", hlc(2_000, 0, "router-a"));
        assert_eq!(r2, UnpublishOutcome::Unpublished);
        assert!(book.get("printer").is_none());

        // 3. A stale, pre-unpublish republish (older HLC) must not resurrect it.
        let stale = entry("router-a", 1_500, 0, "router-a");
        let r3 = apply_service_publish_v2(&mut book, &tombstones, "printer", stale).unwrap();
        assert_eq!(r3, PublishOutcome::RejectedByTombstone);
        assert!(book.get("printer").is_none());

        // 4. A different owner cannot claim the vacated name.
        let intruder = entry("router-b", 5_000, 0, "router-b");
        let r4 = apply_service_publish_v2(&mut book, &tombstones, "printer", intruder).unwrap();
        assert_eq!(r4, PublishOutcome::RejectedByTombstone);
        assert!(book.get("printer").is_none());

        // 5. The original owner republishes with a newer HLC than the tombstone: revives.
        let revived = entry("router-a", 3_000, 0, "router-a");
        let r5 = apply_service_publish_v2(&mut book, &tombstones, "printer", revived.clone()).unwrap();
        assert_eq!(r5, PublishOutcome::Revived);
        assert_eq!(book.get("printer"), Some(&revived));
    }
}
