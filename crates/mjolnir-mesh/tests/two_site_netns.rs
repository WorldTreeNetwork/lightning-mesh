//! Full two-site cross-mesh reachability test using Linux network namespaces.
//!
//! ALL TESTS HERE ARE `#[ignore]`. They require:
//!   - Linux host (uses netns, veth, real TUN)
//!   - root or CAP_NET_ADMIN
//!   - `babeld` binary on PATH
//!   - Daemon wiring layer that binds iroh-gossip ↔ CRDT store ↔ babeld
//!     supervisor together. **This wiring layer does not yet exist in
//!     mjolnir-mesh as of bead mjolnir-mesh-mab.1's closure** — it's the
//!     next epic.
//!
//! Run explicitly with: `cargo test -p mjolnir-mesh --test two_site_netns -- --ignored`
//!
//! The scenario this test should eventually cover:
//!
//! 1. Two netns "site-a" and "site-b" with a veth pair between them.
//! 2. mjolnir-mesh daemon running in each, sharing a CRDT store via gossip
//!    over the veth (mock iroh transport).
//! 3. site-a claims 10.42.1.0/24, site-b claims 10.42.2.0/24.
//! 4. Within ~5s, babeld in both netns has installed a kernel route to the
//!    other's /24 via its `mj-peer-*` interface.
//! 5. ICMP echo from a dummy host on site-a to a dummy host on site-b
//!    succeeds.
//! 6. Killing the site-b daemon: within ~30s (Babel hello/IHU timeout), the
//!    10.42.2.0/24 route disappears from site-a's kernel.
//! 7. Restarting site-b: within ~5s, the route is reinstalled.

#[test]
#[ignore = "requires Linux + root + babeld + daemon wiring (deferred)"]
fn two_site_routes_install_and_converge() {
    // Stub. See module-level doc.
    panic!("not implemented — see module doc for prerequisites");
}

#[test]
#[ignore = "requires Linux + root + babeld + daemon wiring (deferred)"]
fn site_b_death_withdraws_route_within_30s() {
    panic!("not implemented — see module doc for prerequisites");
}

#[test]
#[ignore = "requires Linux + root + babeld + daemon wiring (deferred)"]
fn site_b_restart_reconverges_within_5s() {
    panic!("not implemented — see module doc for prerequisites");
}
