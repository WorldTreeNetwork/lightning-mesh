//! Ephemeral per-node liveness plane (bead e21.9).
//!
//! Staleness for the self-announced lanes (services / `.mesh` names / the 0yb
//! address book) does NOT ride the durable CRDT. Encoding liveness into an
//! entry's HLC would force a flash write every anti-entropy tick (the churn
//! tracked by `7bf`). Instead each node emits a tiny [`LivenessBeacon`] once per
//! tick — never merged into a book, never persisted, never relayed — and every
//! other node keeps an in-memory [`LivenessTracker`] of when it last heard a
//! *newer* beacon from each origin. A record is stale iff its owner's beacon has
//! not advanced within [`LivenessTracker::stale_threshold_ms`].
//!
//! The design rationale (why a heartbeat and not a re-stamped HLC, why the
//! timestamp is receiver-local, why `incarnation` handles restart with zero
//! persisted state, and the partition-blindness handed off to the `4hl` SWIM
//! upgrade) lives in `docs/network-coordination/lane-staleness.md`.
//!
//! ## The clock is deliberately weaker than an HLC
//!
//! The beacon orders nothing — it only proves recency of contact — so it sheds
//! the HLC's wall clock and node-ordering. Freshness is judged by the
//! *receiver's* local clock (`now_ms - received_at_ms`), never by any timestamp
//! carried in the beacon, which is what makes the whole plane immune to clock
//! skew between nodes. The only ordering the beacon needs is "is this beacon
//! newer than the last one I accepted from this origin", answered by the
//! `(incarnation, counter)` pair:
//!
//! - `counter` is a per-boot monotonic sequence (`+1` each emitted beacon).
//! - `incarnation` is the origin's boot wall-clock time in ms. A reboot yields a
//!   later boot time -> a strictly greater incarnation, so a restarted node whose
//!   `counter` reset to 0 is still accepted (its incarnation dominates). No
//!   incarnation is persisted; it is read from the system clock once at boot.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Instant;

use crate::crdt::egress::EgressAd;

/// Process-monotonic milliseconds since the first call. The liveness plane
/// judges staleness purely by *receiver-local* elapsed time (never a remote
/// wall clock), so a monotonic source is exactly right — it cannot jump
/// backwards under NTP steps the way `SystemTime` can. All callers (beacon
/// ingest, the read-side DNS filter, the anti-entropy sweep) share this one
/// clock so their deltas are comparable.
pub fn monotonic_now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// One node's most recently *accepted* beacon, plus the receiver-local time it
/// arrived. `received_at_ms` is the only field staleness is computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seen {
    incarnation: u64,
    counter: u64,
    received_at_ms: u64,
    /// The origin's internet-egress advertisement from this beacon
    /// (mjolnir-mesh-5lw): `Some` iff it announced itself a live gateway.
    /// Refreshed on every accepted beacon, so a gateway that stops advertising
    /// egress (drops to `None`) is reflected on its next beacon, and one that
    /// stops beaconing entirely goes stale via `received_at_ms` like any record.
    egress: Option<EgressAd>,
}

impl Seen {
    /// True if `(incarnation, counter)` strictly succeeds this record — a
    /// higher incarnation (restart) or, at the same incarnation, a higher
    /// counter (next tick). Equal or older is a duplicate/replay and must not
    /// refresh liveness (a stale replay is not evidence the origin is alive).
    fn superseded_by(&self, incarnation: u64, counter: u64) -> bool {
        incarnation > self.incarnation
            || (incarnation == self.incarnation && counter > self.counter)
    }
}

/// In-memory liveness view: origin node id -> last accepted beacon. Never
/// persisted; rebuilt from scratch each boot (every entry loaded from disk is
/// implicitly stale until its owner's first post-boot beacon arrives, which is
/// correct — a restart is not evidence any peer died, and the local node grants
/// itself and everyone a fresh start via [`LivenessTracker::touch`] at boot /
/// self-announce).
#[derive(Debug, Clone)]
pub struct LivenessTracker {
    seen: BTreeMap<String, Seen>,
    /// A record whose owner has not produced a newer beacon within this many ms
    /// stops resolving / reads as stale. Sized at a few anti-entropy intervals.
    pub stale_threshold_ms: u64,
    /// After this many ms with no newer beacon, the owner's records may be
    /// dropped from their books entirely (unbounded-growth guard). Much larger
    /// than `stale_threshold_ms`.
    pub hard_expiry_ms: u64,
}

impl LivenessTracker {
    pub fn new(stale_threshold_ms: u64, hard_expiry_ms: u64) -> Self {
        Self {
            seen: BTreeMap::new(),
            stale_threshold_ms,
            hard_expiry_ms,
        }
    }

    /// Ingest a beacon observed at receiver-local time `now_ms`. Returns `true`
    /// if it was newer than what we held (and therefore refreshed liveness),
    /// `false` if it was a duplicate/older replay we ignored. Carries no egress
    /// advertisement — thin wrapper over [`observe_with_egress`] for callers
    /// (and the many liveness tests) that don't care about the gateway lane.
    pub fn observe(&mut self, node_id: &str, incarnation: u64, counter: u64, now_ms: u64) -> bool {
        self.observe_with_egress(node_id, incarnation, counter, None, now_ms)
    }

    /// Like [`observe`], but also records the origin's internet-egress
    /// advertisement (mjolnir-mesh-5lw). The egress is stored only when the
    /// beacon is accepted (strictly newer), so a stale replay can neither
    /// refresh liveness nor resurrect a withdrawn gateway. Passing `None` clears
    /// the gateway status on the next accepted beacon — the mechanism by which a
    /// node that loses its uplink stops being counted as a gateway.
    pub fn observe_with_egress(
        &mut self,
        node_id: &str,
        incarnation: u64,
        counter: u64,
        egress: Option<EgressAd>,
        now_ms: u64,
    ) -> bool {
        match self.seen.get_mut(node_id) {
            Some(prev) if !prev.superseded_by(incarnation, counter) => false,
            Some(prev) => {
                *prev = Seen {
                    incarnation,
                    counter,
                    received_at_ms: now_ms,
                    egress,
                };
                true
            }
            None => {
                self.seen.insert(
                    node_id.to_string(),
                    Seen {
                        incarnation,
                        counter,
                        received_at_ms: now_ms,
                        egress,
                    },
                );
                true
            }
        }
    }

    /// Refresh a node's liveness to `now_ms` without a beacon comparison. Used
    /// for THIS node's own id (we know we are alive; we need not receive our own
    /// gossip), so records this node owns never read as stale locally. Preserves
    /// any known `(incarnation, counter)` so a later real beacon still orders
    /// correctly; seeds a zero pair if none is known yet.
    pub fn touch(&mut self, node_id: &str, now_ms: u64) {
        match self.seen.get_mut(node_id) {
            Some(prev) => prev.received_at_ms = now_ms,
            None => {
                self.seen.insert(
                    node_id.to_string(),
                    Seen {
                        incarnation: 0,
                        counter: 0,
                        received_at_ms: now_ms,
                        egress: None,
                    },
                );
            }
        }
    }

    /// The set of nodes currently believed to be **live internet gateways**:
    /// origins that are not stale and whose last accepted beacon advertised a
    /// healthy egress (mjolnir-mesh-5lw). This is the positively-expiring
    /// live-gateway set — a gateway that stops beaconing drops out via
    /// [`is_stale`], and one that withdraws its uplink drops out when its next
    /// beacon carries `egress: None` (or `healthy: false`). Consumers use it to
    /// decide/withdraw their own default route instead of trusting babeld's
    /// implicit redistribution.
    pub fn live_gateways(&self, now_ms: u64) -> Vec<(String, EgressAd)> {
        self.seen
            .iter()
            .filter(|(id, _)| !self.is_stale(id, now_ms))
            .filter_map(|(id, s)| s.egress.filter(|ad| ad.healthy).map(|ad| (id.clone(), ad)))
            .collect()
    }

    /// Elapsed ms since the last accepted beacon from `node_id`, or `None` if we
    /// have never heard from it.
    fn age_ms(&self, node_id: &str, now_ms: u64) -> Option<u64> {
        self.seen
            .get(node_id)
            .map(|s| now_ms.saturating_sub(s.received_at_ms))
    }

    /// The receiver-local time (ms, this process's monotonic domain) we last
    /// accepted a newer beacon from `node_id`, or `None` if never. A higher
    /// value means more recently heard from — used by `f8b` to rank gossip
    /// bootstrap candidates so a rejoin prefers peers seen recently over
    /// long-quiet ones. Comparable only against other values from this same
    /// process run (it is `monotonic_now_ms`-based, not wall clock).
    pub fn last_seen_ms(&self, node_id: &str) -> Option<u64> {
        self.seen.get(node_id).map(|s| s.received_at_ms)
    }

    /// True if records owned by `node_id` should stop resolving: either we have
    /// never heard a beacon from it, or the last one is older than
    /// `stale_threshold_ms`. The read-side filter (DNS, status) keys off this.
    pub fn is_stale(&self, node_id: &str, now_ms: u64) -> bool {
        match self.age_ms(node_id, now_ms) {
            None => true,
            Some(age) => age > self.stale_threshold_ms,
        }
    }

    /// True if records owned by `node_id` may be dropped from their books
    /// entirely — the owner has been silent past `hard_expiry_ms`. A node we
    /// have never heard from is NOT hard-expired (no `received_at` to age from):
    /// hard-expiry only reclaims things that were once live and then went
    /// silent, so a freshly loaded, never-beaconed entry gets its full grace
    /// window first via the owner's next beacon.
    pub fn is_hard_expired(&self, node_id: &str, now_ms: u64) -> bool {
        match self.age_ms(node_id, now_ms) {
            None => false,
            Some(age) => age > self.hard_expiry_ms,
        }
    }

    /// Drop a node from the view (e.g. after its records were hard-expired and
    /// its addr-book entry removed) so the map does not retain ids for departed
    /// nodes forever.
    pub fn forget(&mut self, node_id: &str) {
        self.seen.remove(node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STALE: u64 = 60_000; // 3 * 20s
    const HARD: u64 = 3_600_000; // 1h

    fn tracker() -> LivenessTracker {
        LivenessTracker::new(STALE, HARD)
    }

    #[test]
    fn first_beacon_is_accepted_and_fresh() {
        let mut t = tracker();
        assert!(t.observe("x", 100, 1, 1_000));
        assert!(!t.is_stale("x", 1_000));
        assert!(!t.is_stale("x", 1_000 + STALE)); // exactly at threshold: not yet stale
    }

    #[test]
    fn unknown_node_is_stale() {
        let t = tracker();
        assert!(t.is_stale("ghost", 5_000));
        assert!(!t.is_hard_expired("ghost", 5_000)); // never heard -> not reclaimable yet
    }

    #[test]
    fn goes_stale_after_threshold() {
        let mut t = tracker();
        t.observe("x", 100, 1, 1_000);
        assert!(!t.is_stale("x", 1_000 + STALE));
        assert!(t.is_stale("x", 1_001 + STALE)); // one ms past
    }

    #[test]
    fn newer_counter_refreshes_liveness() {
        let mut t = tracker();
        t.observe("x", 100, 1, 1_000);
        // A later tick at the same incarnation, right before staleness.
        assert!(t.observe("x", 100, 2, 1_000 + STALE));
        assert!(!t.is_stale("x", 1_000 + 2 * STALE)); // re-based on the newer beacon
    }

    #[test]
    fn older_or_equal_beacon_is_ignored_and_does_not_refresh() {
        let mut t = tracker();
        t.observe("x", 100, 5, 1_000);
        // A duplicate/replayed older beacon arriving much later must NOT count
        // as liveness — otherwise a stale replay would resurrect a dead node.
        assert!(!t.observe("x", 100, 5, 50_000)); // equal -> ignored
        assert!(!t.observe("x", 100, 3, 50_000)); // older counter -> ignored
        // Liveness is still based on the original 1_000 receipt.
        assert!(t.is_stale("x", 1_001 + STALE));
    }

    #[test]
    fn restart_with_reset_counter_is_accepted_via_incarnation() {
        let mut t = tracker();
        t.observe("x", 100, 500, 1_000);
        // Node reboots: incarnation jumps (later boot time), counter resets to 0.
        // Must be accepted despite counter 0 < 500.
        assert!(t.observe("x", 200, 0, 90_000));
        assert!(!t.is_stale("x", 90_000));
    }

    #[test]
    fn stale_owner_returns_and_unstales() {
        let mut t = tracker();
        t.observe("x", 100, 1, 1_000);
        assert!(t.is_stale("x", 1_000 + 10 * STALE)); // long gone
        // Owner comes back with a fresh beacon.
        assert!(t.observe("x", 100, 2, 1_000 + 10 * STALE));
        assert!(!t.is_stale("x", 1_000 + 10 * STALE));
    }

    #[test]
    fn hard_expiry_is_past_stale() {
        let mut t = tracker();
        t.observe("x", 100, 1, 1_000);
        assert!(t.is_stale("x", 1_000 + STALE + 1));
        assert!(!t.is_hard_expired("x", 1_000 + STALE + 1)); // stale but not reclaimable
        assert!(!t.is_hard_expired("x", 1_000 + HARD)); // exactly at threshold
        assert!(t.is_hard_expired("x", 1_001 + HARD));
    }

    #[test]
    fn touch_keeps_self_fresh_without_a_beacon() {
        let mut t = tracker();
        t.touch("self", 1_000);
        assert!(!t.is_stale("self", 1_000 + STALE));
        // Re-touching each tick keeps it perpetually fresh.
        t.touch("self", 1_000 + STALE);
        assert!(!t.is_stale("self", 1_000 + 2 * STALE));
    }

    #[test]
    fn touch_preserves_incarnation_ordering_for_a_later_real_beacon() {
        let mut t = tracker();
        // We touched self (seeds 0,0), then later actually observe our own
        // beacon relayed back — a real (incarnation, counter) must still be
        // accepted over the seeded zero pair.
        t.touch("x", 1_000);
        assert!(t.observe("x", 50, 1, 2_000));
        // And a subsequent older-than-that beacon is ignored.
        assert!(!t.observe("x", 50, 1, 3_000));
    }

    #[test]
    fn forget_drops_the_node() {
        let mut t = tracker();
        t.observe("x", 100, 1, 1_000);
        t.forget("x");
        assert!(t.is_stale("x", 1_000)); // back to unknown
        assert!(!t.is_hard_expired("x", 1_000 + HARD));
    }

    fn ad() -> EgressAd {
        EgressAd {
            healthy: true,
            cost_hint: 0,
        }
    }

    #[test]
    fn live_gateway_appears_and_expires_with_liveness() {
        let mut t = tracker();
        // Non-gateway beacon: never listed.
        t.observe("plain", 100, 1, 1_000);
        assert!(t.live_gateways(1_000).is_empty());
        // Gateway beacon: listed while fresh...
        t.observe_with_egress("gw", 100, 1, Some(ad()), 1_000);
        assert_eq!(
            t.live_gateways(1_000),
            vec![("gw".to_string(), ad())],
            "a freshly-beaconed gateway is live"
        );
        // ...and drops out once its beacon goes stale (positive expiry — the
        // 5lw point: a gateway that stops beaconing is KNOWN gone).
        assert!(t.live_gateways(1_001 + STALE).is_empty());
    }

    #[test]
    fn gateway_withdraws_by_beaconing_none() {
        let mut t = tracker();
        t.observe_with_egress("gw", 100, 1, Some(ad()), 1_000);
        assert_eq!(t.live_gateways(1_000).len(), 1);
        // Uplink lost: the next (newer) beacon carries egress: None.
        assert!(t.observe_with_egress("gw", 100, 2, None, 2_000));
        assert!(
            t.live_gateways(2_000).is_empty(),
            "a withdrawn uplink drops the node from the live-gateway set"
        );
    }

    #[test]
    fn unhealthy_egress_is_not_a_live_gateway() {
        let mut t = tracker();
        let sick = EgressAd {
            healthy: false,
            cost_hint: 0,
        };
        t.observe_with_egress("gw", 100, 1, Some(sick), 1_000);
        assert!(
            t.live_gateways(1_000).is_empty(),
            "a dead/captive uplink (healthy: false) must not be selected"
        );
    }

    #[test]
    fn stale_replay_cannot_resurrect_a_withdrawn_gateway() {
        let mut t = tracker();
        t.observe_with_egress("gw", 100, 1, Some(ad()), 1_000);
        // Withdraw at counter 2.
        t.observe_with_egress("gw", 100, 2, None, 2_000);
        // A replayed OLD gateway beacon (counter 1) must be ignored — it neither
        // refreshes liveness nor brings the gateway back.
        assert!(!t.observe_with_egress("gw", 100, 1, Some(ad()), 3_000));
        assert!(t.live_gateways(3_000).is_empty());
    }

    #[test]
    fn tracks_multiple_nodes_independently() {
        let mut t = tracker();
        t.observe("a", 100, 1, 1_000);
        t.observe("b", 100, 1, 40_000);
        // At 1_000 + STALE + 1, a is stale but b (seen at 40_000) is fresh.
        let now = 1_001 + STALE;
        assert!(t.is_stale("a", now));
        assert!(!t.is_stale("b", now));
    }
}
