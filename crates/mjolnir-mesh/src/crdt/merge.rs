use crate::crdt::subnet::SubnetClaim;
use std::cmp::Ordering;

/// Result of merging an incoming entry into a CRDT store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeResult<T> {
    /// Key did not exist locally; inserted.
    Inserted,
    /// Key existed and incoming was strictly newer; replaced.
    Updated,
    /// Key existed and incoming was equal or older; discarded.
    Unchanged,
    /// Conflict on a domain invariant (e.g., same CIDR claimed by two owners).
    /// Caller is responsible for taking conflict-recovery action.
    Conflict { winner: T, loser: T },
}

/// First-writer-wins: lower HLC = first claimer wins.
/// Pure, deterministic. Both routers seeing the same (a, b) pair reach the same verdict.
///
/// Tie-break order: HLC.wall_clock → HLC.counter → HLC.node_id (lexicographic).
/// Inherits Ord from `HLC`.
pub fn resolve_subnet_conflict<'a>(
    a: &'a SubnetClaim,
    b: &'a SubnetClaim,
) -> (&'a SubnetClaim, &'a SubnetClaim) {
    match a.claimed_at.cmp(&b.claimed_at) {
        Ordering::Less => (a, b),
        Ordering::Greater => (b, a),
        Ordering::Equal => {
            // Identical HLC → tie-break on owner_node_id.
            // This branch is hit only when wall_clock + counter + hlc.node_id are
            // all equal but owner_node_id differs. Extremely rare in practice.
            if a.owner_node_id <= b.owner_node_id {
                (a, b)
            } else {
                (b, a)
            }
        }
    }
}

/// Pure function: given the local entry (if any) and an incoming entry for the
/// same CIDR, return the merge result.
///
/// Note: this function does not enforce that `local.cidr == incoming.cidr` —
/// the caller must look up local by CIDR before calling.
pub fn merge_subnet_claim(
    local: Option<&SubnetClaim>,
    incoming: &SubnetClaim,
) -> MergeResult<SubnetClaim> {
    match local {
        None => MergeResult::Inserted,
        Some(existing) => {
            // Same owner means this is a refresh/update, not a conflict.
            if existing.owner_node_id == incoming.owner_node_id {
                match incoming.claimed_at.cmp(&existing.claimed_at) {
                    Ordering::Greater => MergeResult::Updated,
                    _ => MergeResult::Unchanged,
                }
            } else {
                // Different owners → conflict on the claim.
                let (winner, loser) = resolve_subnet_conflict(existing, incoming);
                MergeResult::Conflict {
                    winner: winner.clone(),
                    loser: loser.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ipnet::IpNet;

    use super::*;
    use crate::crdt::hlc::HLC;

    fn cidr() -> IpNet {
        IpNet::from_str("10.42.1.0/24").unwrap()
    }

    fn claim(owner: &str, wall_clock: u64, counter: u32, hlc_node: &str) -> SubnetClaim {
        SubnetClaim {
            cidr: cidr(),
            owner_node_id: owner.to_string(),
            site_name: None,
            claimed_at: HLC {
                wall_clock,
                counter,
                node_id: hlc_node.to_string(),
            },
        }
    }

    #[test]
    fn inserted_when_no_local() {
        let incoming = claim("router-a", 1_000, 0, "router-a");
        assert!(matches!(merge_subnet_claim(None, &incoming), MergeResult::Inserted));
    }

    #[test]
    fn unchanged_on_duplicate() {
        let entry = claim("router-a", 1_000, 0, "router-a");
        assert!(matches!(
            merge_subnet_claim(Some(&entry), &entry),
            MergeResult::Unchanged
        ));
    }

    #[test]
    fn updated_on_newer_from_same_owner() {
        let local = claim("router-a", 1_000, 0, "router-a");
        let incoming = claim("router-a", 2_000, 0, "router-a");
        assert!(matches!(
            merge_subnet_claim(Some(&local), &incoming),
            MergeResult::Updated
        ));
    }

    #[test]
    fn unchanged_on_older_from_same_owner() {
        let local = claim("router-a", 2_000, 0, "router-a");
        let incoming = claim("router-a", 1_000, 0, "router-a");
        assert!(matches!(
            merge_subnet_claim(Some(&local), &incoming),
            MergeResult::Unchanged
        ));
    }

    #[test]
    fn conflict_resolves_lower_hlc_wins() {
        let a = claim("router-a", 1_000, 0, "router-a");
        let b = claim("router-b", 2_000, 0, "router-b");
        let result = merge_subnet_claim(Some(&a), &b);
        match result {
            MergeResult::Conflict { winner, loser } => {
                assert_eq!(winner.owner_node_id, "router-a");
                assert_eq!(loser.owner_node_id, "router-b");
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn conflict_tie_break_by_counter() {
        // Equal wall_clock, lower counter wins.
        let a = claim("router-a", 1_000, 0, "router-a");
        let b = claim("router-b", 1_000, 1, "router-b");
        let result = merge_subnet_claim(Some(&a), &b);
        match result {
            MergeResult::Conflict { winner, loser } => {
                assert_eq!(winner.owner_node_id, "router-a");
                assert_eq!(loser.owner_node_id, "router-b");
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn conflict_tie_break_by_hlc_node_id() {
        // Equal wall_clock and counter, lower hlc.node_id wins.
        let a = claim("router-a", 1_000, 0, "aaa");
        let b = claim("router-b", 1_000, 0, "zzz");
        let result = merge_subnet_claim(Some(&a), &b);
        match result {
            MergeResult::Conflict { winner, loser } => {
                assert_eq!(winner.owner_node_id, "router-a");
                assert_eq!(loser.owner_node_id, "router-b");
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn conflict_tie_break_by_owner_node_id_when_hlc_equal() {
        // HLC fully equal (same wall_clock, counter, node_id) → tie-break on owner_node_id.
        let a = claim("aaa-owner", 1_000, 0, "shared-node");
        let b = claim("zzz-owner", 1_000, 0, "shared-node");
        let result = merge_subnet_claim(Some(&a), &b);
        match result {
            MergeResult::Conflict { winner, loser } => {
                assert_eq!(winner.owner_node_id, "aaa-owner");
                assert_eq!(loser.owner_node_id, "zzz-owner");
            }
            other => panic!("expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn deterministic_across_arg_order() {
        let a = claim("router-a", 1_000, 0, "router-a");
        let b = claim("router-b", 2_000, 0, "router-b");
        let (w1, l1) = resolve_subnet_conflict(&a, &b);
        let (w2, l2) = resolve_subnet_conflict(&b, &a);
        assert_eq!(w1.owner_node_id, w2.owner_node_id);
        assert_eq!(l1.owner_node_id, l2.owner_node_id);
    }
}
