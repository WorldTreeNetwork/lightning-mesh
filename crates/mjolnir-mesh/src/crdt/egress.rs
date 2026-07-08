//! Internet-egress advertisement carried on the liveness beacon plane
//! (mjolnir-mesh-5lw, step 7z5).
//!
//! A node is a *live local gateway* when it can reach the internet through a
//! real uplink of its own. Today that fact is inferred implicitly by babeld
//! redistributing whatever default route sits in the kernel FIB — which lets a
//! stale `proto babel` default masquerade as a gateway (mjolnir-mesh-5lw). This
//! module makes egress a **positively-asserted fact**: each node classifies its
//! own uplink and rides the answer on its [`LivenessBeacon`], so every peer can
//! build a live-gateway set it can *positively expire* via the same
//! [`LivenessTracker`](crate::crdt::liveness::LivenessTracker) staleness the
//! rest of the ephemeral plane uses.
//!
//! The classifier here is **pure** — it takes already-read default-route
//! candidates and decides. The rtnetlink read that produces the candidates is a
//! thin Linux shim wired in at the reconciler (a later 5lw step); keeping the
//! decision pure means it unit-tests without a kernel FIB.

use serde::{Deserialize, Serialize};

/// Interfaces that must never count as an internet uplink: the 802.11s backhaul
/// bridge and the overlay TUN. A default route out of either is the mesh itself
/// (or, for `mode=internet`/buw.7, the overlay riding its own uplink) — treating
/// it as egress would re-announce the mesh's own path back into the mesh.
pub const EXCLUDED_EGRESS_IFACES: &[&str] = &["br-mesh", "mjolnir0"];

/// A node's advertised internet-egress capability for one beacon tick. Rides
/// [`LivenessBeacon`](crate::crdt::gossip::GossipMessage::LivenessBeacon) as
/// `Option<EgressAd>` — `None` means "not a gateway this tick". Never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressAd {
    /// The uplink passed its reachability check. Until the 42j probe lands this
    /// is set from route presence alone (a route exists => assumed healthy);
    /// once the probe exists, a dead/captive lease yields `healthy: false` and
    /// consumers skip this gateway.
    pub healthy: bool,
    /// Advisory cost of egressing through this node (lower = better), mirroring
    /// the babel metric headroom so nearest-exit selection can use it as a
    /// tie-breaker. Informational for now — babel's own metric still decides the
    /// installed route.
    pub cost_hint: u16,
}

/// One candidate default route read from the kernel FIB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultRoute {
    /// Output interface name (e.g. `wan`, `eth1`, `br-mesh`, `mjolnir0`).
    pub oif: String,
    /// True if this route was installed by babel (`proto babel`). A babel
    /// default is a *learned* mesh path, never our own uplink — accepting it
    /// would let a learned/stale default hijack the gateway role (5lw).
    pub proto_babel: bool,
}

/// Decide whether this node is a local internet gateway from its default-route
/// candidates. A route qualifies iff it is NOT `proto babel` and its output
/// interface is not in `excluded` (the backhaul / overlay). Returns the egress
/// advertisement to beacon, or `None` if no candidate qualifies.
///
/// `cost_hint` is `0` for a directly-attached uplink (this is the exit); a probe
/// step may later raise it. Pure and platform-free.
pub fn classify_egress<'a, I>(candidates: I, excluded: &[&str]) -> Option<EgressAd>
where
    I: IntoIterator<Item = &'a DefaultRoute>,
{
    let qualifies = candidates
        .into_iter()
        .any(|r| !r.proto_babel && !excluded.contains(&r.oif.as_str()));
    qualifies.then_some(EgressAd {
        healthy: true,
        cost_hint: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(oif: &str, proto_babel: bool) -> DefaultRoute {
        DefaultRoute {
            oif: oif.to_string(),
            proto_babel,
        }
    }

    #[test]
    fn real_wan_default_is_a_gateway() {
        let routes = [route("wan", false)];
        assert_eq!(
            classify_egress(&routes, EXCLUDED_EGRESS_IFACES),
            Some(EgressAd {
                healthy: true,
                cost_hint: 0
            })
        );
    }

    #[test]
    fn no_default_route_is_not_a_gateway() {
        let routes: [DefaultRoute; 0] = [];
        assert_eq!(classify_egress(&routes, EXCLUDED_EGRESS_IFACES), None);
    }

    #[test]
    fn learned_babel_default_is_not_a_gateway() {
        // The whole 5lw hazard: a node that only LEARNED the default (proto
        // babel) must never advertise itself as a gateway, or it re-exports a
        // path back into the mesh and can hijack the real uplink.
        let routes = [route("br-mesh", true)];
        assert_eq!(classify_egress(&routes, EXCLUDED_EGRESS_IFACES), None);
    }

    #[test]
    fn default_out_the_backhaul_or_overlay_is_excluded() {
        // Even a non-babel default out br-mesh/mjolnir0 is the mesh's own path,
        // not an uplink (buw.7 mode=internet self-announce guard).
        assert_eq!(
            classify_egress(&[route("br-mesh", false)], EXCLUDED_EGRESS_IFACES),
            None
        );
        assert_eq!(
            classify_egress(&[route("mjolnir0", false)], EXCLUDED_EGRESS_IFACES),
            None
        );
    }

    #[test]
    fn a_real_uplink_wins_even_beside_a_learned_default() {
        // Multi-default node: it learned a mesh default AND has its own WAN.
        // Its own WAN qualifies -> it is a gateway.
        let routes = [route("br-mesh", true), route("wan", false)];
        assert!(classify_egress(&routes, EXCLUDED_EGRESS_IFACES).is_some());
    }
}
