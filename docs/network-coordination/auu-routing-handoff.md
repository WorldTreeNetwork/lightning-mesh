# auu — Routing handoff: iroh tunnel death → babel-over-L2 → the duplicate-IP nexthop blocker

Status as of 2026-06-25. Bead: `mjolnir-mesh-auu`. Hardware: two MikroTik
routers, `192.168.0.181` (router-1, id `81f5…`, backhaul `10.254.23.7`) and
`192.168.0.113` (router-2, id `9b8c…`, backhaul `10.254.3.43`). Both run the
**provably identical** build `d043fdc` (verified via the startup banner stamp).

This documents what the network actually looks like, the routing methods tried,
the remaining blocker, and the options — for picking up in a fresh agent window.

---

## 1. Network topology (as observed on hardware + from the deploy `.rsc`)

Per node (`deploy/mikrotik/container-net-lan.rsc`):

```
  RouterOS host                         container (meshd + babeld)
  ┌────────────────────────┐           ┌──────────────────────────────┐
  │ br-mesh  172.20.0.1/24  │           │ eth0 / "veth-mesh":          │
  │   ├─ port: veth-mesh ───┼──L2 bridge┤   172.20.0.2/24 (PLACEHOLDER)│
  │   └─ port: $meshLink ───┼─┐         │   10.254.<id>/16 (meshd adds)│
  │ route 10.42.0.0/16      │ │         │   fe80::… (link-local)       │
  │      → 172.20.0.2        │ │         └──────────────────────────────┘
  └────────────────────────┘ │
                             │  $meshLink = the physical port on the
                             │  SHARED L2 segment (switch / WiFi backhaul)
                             ▼
            ══════════ shared L2 broadcast domain ══════════
            (every node's br-mesh is bridged onto this one segment)
```

Confirmed live on BOTH routers:

| node | host addr on br-mesh | container addr | unique backhaul (meshd) |
|------|----------------------|----------------|-------------------------|
| router-1 | `172.20.0.1/24` | `172.20.0.2` | `10.254.23.7/16` |
| router-2 | `172.20.0.1/24` | `172.20.0.2` | `10.254.3.43/16` |

RouterOS route on both: `10.42.0.0/16 → 172.20.0.2` (`client-routing.rsc`).

### The core defect this exposes

`172.20.0.1` (hosts) and `172.20.0.2` (containers) are **hard-coded identical on
every node** (`container-net-lan.rsc:44` and the host bridge addr), yet every
node's `br-mesh` is bridged onto **one shared L2 broadcast domain**. So those
addresses are **duplicated across the segment**. They were only ever a
"placeholder so the interface is valid/up" (per the script's own comment) and a
local host↔container hop — fine in the *old* model where inter-node traffic went
through iroh tunnels (`10.255.x` /31s), never over `172.20.x`.

The **only unique, segment-routable inter-node address is `10.254.x/16`**
(meshd-assigned, derived from node id).

---

## 2. The remaining routing blocker (precise)

We now route babel directly over the shared L2 (see history below). babel peers
and propagates fine, BUT the installed next-hop is wrong:

```
router-1: 10.42.3.0/24  … via veth-mesh neigh fe80::4a74…  nexthop 172.20.0.2  metric 65535
router-2: 10.42.23.0/24 … via veth-mesh neigh fe80::e25…   nexthop 172.20.0.2  metric 96 (feasible)
```

- babeld advertises **`172.20.0.2` as the IPv4 next-hop** (the veth's primary/
  configured address) instead of the unique `10.254.x`. `172.20.0.2` is the
  receiver's *own* container address → forwarding there is a no-op / wrong host.
- The **metric asymmetry** (one side `96 feasible`, the other `65535` = babel
  infinity) is almost certainly a downstream symptom of the ambiguous/duplicate
  next-hop + ARP confusion on the shared L2, not a separate root cause.

Net: **peering + route propagation work; actual packet forwarding does not**,
because the next-hop address is duplicated/placeholder, not the unique backhaul.

---

## 3. Options to fix the next-hop (pick in the new window)

All must satisfy: (a) babel installs the unique `10.254.x` as the inter-node
next-hop; (b) host↔container client transit (`10.42/16` ↔ LAN) still works;
(c) no duplicate IPs on the shared L2.

- **Option A — Drop the `172.20.x` placeholder; babel uses `10.254.x`.**
  Make `10.254.x` the only IPv4 on the babel interface so babeld advertises it
  as next-hop. Requires reworking host↔container transit: the host's
  `10.42/16 → 172.20.0.2` route must instead target the container's (derived)
  `10.254.x`. Awkward because `10.254.x` is per-node-derived, not a static
  `.rsc` constant — would need the deploy to template it from the node id.
  *Smallest mental model, but the static-config rework is fiddly.*

- **Option B — Separate the two links (clean redesign).** Give the container
  TWO interfaces: a **private** host↔container veth (`172.20.0.1/.2`, NOT bridged
  to the shared L2) for client transit, and a **backhaul** interface bridged to
  the shared L2 carrying ONLY `10.254.x`. babel runs on the backhaul iface →
  unique next-hop; no duplicate IPs on the segment. *Cleanest; needs
  container-net-lan.rsc + meshd backhaul-iface handling changes.* **Recommended.**

- **Option C — babel IPv4-via-IPv6 (RFC 5549) next-hops.** Have babeld use the
  per-node-unique IPv6 link-local (`fe80::…`) as the next-hop for IPv4 routes,
  bypassing the IPv4 next-hop entirely (`10.42.3.0/24 via fe80::… dev veth-mesh`).
  babeld does this when the babel interface has no usable IPv4 addr (or via
  config); needs the bogus `172.20.0.2` removed from that iface and kernel/
  iproute2 onlink-v6-nexthop support (modern Linux has it). *Elegant, lightest
  touch if babeld + kernel cooperate; verify on the armv7/musl babeld build.*

- **Option D — Make `172.20.x` unique per node.** Derive `172.20.<n>.x` per node
  so the placeholder becomes a real segment address. *Rejected-ish: you'd then
  have two parallel unique schemes (`172.20.x` and `10.254.x`) on one L2 — just
  collapse to `10.254.x` (Option A).*

Cross-cutting cleanup (independent of the above):
- **babeld `-d 1` floods RouterOS `/log`** (route dumps every 1–2s), evicting
  meshd's own INFO lines from the ring in ~2 min even at `memory-lines=50000`.
  Consider dropping babeld verbosity (`supervisor.rs` passes `-d 1`) or routing
  its stdio elsewhere. Made on-hardware diagnosis painful this session.

---

## 4. Short history of routing methods tried

1. **Per-peer iroh TUN tunnels** (`mj-peer-*` /31 in `10.255.x`), babel
   `type tunnel` over each. Original design. **Failed:** iroh 1.0 path-manager
   churn — a redundant-path prune hit `MultipathNotNegotiated`, the selected path
   stopped carrying traffic, the QUIC connection idled out (~36s), looped forever
   (`serve_tunnel` drops the TUN on close → babeld goes empty → flap).
   - *Sub-fix 1:* raised iroh connection idle timeout to 60s (`TUNNEL_MAX_IDLE`).
     Only deferred death ~36s→~63s — confirmed the idle-timeout/prune mechanism.
   - *Sub-fix 2 (commit `a1afd90`):* bound the iroh socket to the unique
     `10.254.x` backhaul so it stopped advertising the bogus `172.20.0.2` as a
     second candidate path. Improved to 65–95s cycles, single advertised addr —
     but still `TimedOut` + residual `MultipathNotNegotiated`. **Wall:** iroh
     1.0.0's public API can't disable multipath OR holepunching (both clamp to a
     min of 8), so the transient-extra-path prune can't be turned off from config.
   - Ruled out binary/version skew cold: added a git **build-stamp banner**
     (commit `c7f64f1`) + deploy verification; both nodes confirmed identical.

2. **babel directly over the shared L2** (`veth-mesh`, `type wired`), no per-peer
   tunnels (commit `d043fdc`). iroh + gossip kept only for the CRDT control plane.
   **Result:** death loop GONE, zero `MultipathNotNegotiated`, babeld stable,
   healthy babel neighbour (`reach ffff`), both `/24`s propagate. **But** the
   installed next-hop is the duplicated `172.20.0.2` placeholder → no forwarding.
   That's the topology blocker in §2/§3.

---

## 5. Pointers

- Bead `mjolnir-mesh-auu` — full chronological notes (`bd show auu`).
- Commits on `main` (pushed): `c7f64f1` (build-stamp + idle discriminator),
  `a1afd90` (iroh backhaul bind), `d043fdc` (babel-over-L2).
- Code: `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs` (LAN mode skips dialers,
  passes resolved backhaul iface to the babel reconciler as `type wired`),
  `crates/mjolnir-mesh/src/babel/config.rs` (`l2_interfaces` renderer).
- Deploy: `deploy/mikrotik/container-net-lan.rsc` (the `172.20.0.2` placeholder),
  `deploy/mikrotik/client-routing.rsc` (`10.42/16 → 172.20.0.2`),
  `deploy/mikrotik/deploy-mesh.sh` (ships one tar to both, verifies build stamp).
- Build+deploy: `bash deploy/mikrotik/build.sh` (stamps git sha), then
  `bash deploy/mikrotik/deploy-mesh.sh`.
- Debugging tip: meshd's own log lines are `mjolnir_meshd` (underscore);
  `/log/print where message~"mjolnir_meshd"` filters out babeld's route-dump spam.
