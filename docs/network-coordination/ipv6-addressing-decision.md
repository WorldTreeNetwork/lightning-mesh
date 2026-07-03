# Decision: IPv6 overlay vs IPv4 subnet claims

**Bead:** `mjolnir-mesh-bsa` · **Status:** PROPOSED (awaiting ratification) · **Date:** 2026-07-02
**Blocks:** `e21` (service-mesh architecture pass)

## Problem

Addressing is a limited resource in the shipped IPv4 design, in three separate places:

1. **Client subnets.** Nodes claim whole `/24`s out of `10.42.0.0/16` — at most **256
   sites** per mesh, ~254 devices each, with waste per claim (a 3-device site burns a
   /24). Expanding to `10.0.0.0/8` raises the ceiling to 65 536 sites but keeps the
   claim/collision machinery and the scarcity mindset.
2. **Backhaul addresses.** `10.254.<blake3(node_id)[0..2]>/16` gives each node a
   16-bit derived identity. Birthday math: ~50 % chance of a collision at **~300
   nodes**, with no resolution protocol — a silent duplicate-address failure at scale.
3. **Service addressing (the e21 phase).** The service mesh wants every node *and
   every service* stably addressable, locally and over the iroh overlay. Carving
   stable service addresses out of already-scarce RFC 1918 space multiplies the
   squatting/collision problem the CRDT exists to police.

The founding constraints are hard requirements: no implicit authority, ad-hoc
join/leave, censorship-resistant, scalable. Scarce address space is *inherently* a
coordination problem — and coordination is exactly what a no-authority network is
worst at. IPv6 removes the scarcity instead of policing it.

Known costs pulling the other way: IPv6 is materially harder to eyeball, debug, type,
and support; dual-stack doubles the surface during any transition; consumer client
devices and apps still assume IPv4 works.

## Requirements (from the e21 service-mesh phase)

| # | Requirement | Source |
|---|---|---|
| R1 | Every node has a stable, identity-derived address with **negligible** collision probability, no claim round needed | backhaul today collides at ~300 nodes |
| R2 | Sites-per-mesh not capped at 256; per-site device count not boxed at claim time | `/24` claims from a /16 |
| R3 | Services individually addressable (`.mesh` names → stable addresses), locally and via the iroh overlay | e21 deliverable (2) |
| R4 | Conflict resolution stays CRDT-shaped (deterministic, local, no quorum) — and ideally has *less* to resolve | ethos |
| R5 | Consumer clients (phones, laptops, IoT) keep working with zero configuration | product reality |
| R6 | Operators can debug it: readable addresses, `ip route` output a human can follow | support cost, explicitly flagged in the bead |
| R7 | Interop posture with the existing mesh ecosystem (LibreMesh et al are IPv6-first) | bead |
| R8 | Incremental migration — the deployed 4-node fleet keeps working at every step | field reality |

## Options

### Option A — IPv4 status quo, widen to 10.0.0.0/8 when needed

Keep `/24` claims, expand the base prefix at deployment time, keep the derived
`10.254/16` backhaul.

- **For:** zero new work; the UX everyone knows; the claim CRDT is field-validated.
- **Against:** fails R1 (backhaul birthday collisions have *no* fix inside 16 bits),
  R3 (no room for per-service addresses), and only postpones R2. The /8 expansion is
  a mesh-wide flag-day config (`Address space configuration is set in the CRDT root
  document`) — exactly the kind of coordinated global change the ethos says we can't
  rely on. Scarcity also invites squatting games as meshes federate (`yau`).

### Option B — IPv6-only overlay (v6 everywhere, NAT64/DNS64 at the client edge)

ULA (or GUA) prefix per mesh; nodes, services, *and clients* are v6; IPv4-only client
traffic crosses a NAT64.

- **For:** one stack, maximal simplification of the core, every address
  identity-derived.
- **Against:** fails R5 hard — IPv4-only IoT/game/legacy clients break or need
  NAT64/DNS64 on every node (new moving part, worse debuggability, DNSSEC pain).
  Fails R6 for the people we support first. Highest migration risk (violates R8
  in spirit: big-bang client-side change).

### Option C — IPv6 spine, IPv4 edge (dual-stack where it's cheap, v6 where it pays) ← **RECOMMENDED**

The mesh **fabric** (backhaul, node identity, service addressing, cross-site overlay)
moves to IPv6. The **client edge** keeps the shipped IPv4 `/24`-per-node model
unchanged, and client LANs *additionally* get a v6 /64 for v6-capable devices.
Concretely:

1. **Mesh prefix:** each mesh derives a ULA `/48` per RFC 4193:
   `fd` + 40-bit Global ID = `blake3(mesh_topic_or_psk)[0..5]`. Meshes get distinct
   prefixes for free → federation (`yau`) without renumbering.
2. **Node identity address:** each node gets a `/128` from
   `<mesh /48>:0::<blake3(node_id)[0..8]>` — 64 bits of derived identity.
   Collision probability at 10⁶ nodes: ~10⁻⁷. **R1 solved arithmetically, not by
   protocol.** This replaces `10.254.x` as the management-plane address
   (SSH inventory moves to names/AAAA over time; `10.254.x` stays during transition).
3. **Backhaul next-hops: v4-via-v6 (RFC 9229).** babeld ≥ 1.12 routes IPv4 prefixes
   with IPv6 link-local next-hops; the fleet ships **babeld 1.13**. The 802.11s
   `br-mesh` already has fe80:: on every node for free. The derived-IPv4 backhaul
   stops being load-bearing → the ~300-node collision cliff disappears without
   renumbering anything client-visible.
4. **Client LANs:** keep claimed IPv4 `/24`s exactly as shipped (R5, R8). Each node
   *also* announces a v6 `/64` on its LAN (`<mesh /48>:<subnet-id>::/64` via SLAAC),
   where the 16-bit subnet-id reuses the **existing claim CRDT** — same merge rule,
   same FWW, new keyspace `/subnets6/`. 65 536 node-slots per mesh vs 256 today (R2).
   v6-capable clients get end-to-end mesh v6; v4-only clients notice nothing.
5. **Services (e21):** service records in the CRDT carry AAAA (node-derived or
   per-service addresses out of the node's `/64` — 2⁶⁴ per node, zero scarcity) plus
   A when the backing service is v4. `.mesh` resolution prefers AAAA on-mesh (R3).
6. **Cross-site overlay:** `mjolnir0` carries v6 the same way it carries v4 — babeld
   announces the v6 prefixes over the same adjacencies. iroh is the transport either
   way; it does not care what the inner packets are.

- **For:** meets R1–R8. Every scarcity problem lands on the v6 side where space is
  effectively infinite; every UX-sensitive surface stays v4. The claim CRDT is
  *reused*, not discarded — but with 65 536 slots and hash-derived preferences,
  conflicts become a formality. babel was IPv6-native first; this is its home turf.
- **Against:** dual-stack means two address families in `ip route` output during the
  (long) transition — R6 is *managed*, not free. SLAAC/RA on client LANs is a new
  surface (odhcpd config). v4-via-v6 needs a kernel with RFC 9229 support (OpenWrt
  ≥ 22.03 / kernel ≥ 5.2 — the mt7986 fleet qualifies; verify per-target in Phase 1).

### Option D — IPv4 + NAT tricks (per-site NAT, overlapping /24s, NAT at mjolnir0)

Let sites reuse overlapping RFC 1918 space and NAT between them at the overlay
boundary.

- **Against (fatal):** NAT is an *authority* — the box that owns the mapping owns
  the conversation. It breaks end-to-end addressing (R3 impossible: a service's
  address differs per observer), breaks the CRDT model (no global fact to agree on),
  and makes `.mesh` naming a lie. Rejected without a scoring pass; it fails the
  ethos, not just the requirements.

## Scorecard

| Requirement | A: v4+/8 | B: v6-only | C: v6 spine/v4 edge | D: NAT |
|---|---|---|---|---|
| R1 stable collision-free node addr | ✗ | ✓ | ✓ | ✗ |
| R2 >256 sites, no size-boxing | ~ | ✓ | ✓ | ✓ (via overlap) |
| R3 per-service addressing | ✗ | ✓ | ✓ | ✗ |
| R4 less to CRDT-arbitrate | ✗ | ✓ | ✓ | ✗ |
| R5 zero-config v4 clients | ✓ | ✗ | ✓ | ✓ |
| R6 debuggable by humans | ✓ | ✗ | ~ (dual-stack) | ✗ (NAT state) |
| R7 ecosystem interop | ✗ | ✓ | ✓ | ✗ |
| R8 incremental from shipped fleet | ✓ | ✗ | ✓ | ✗ |

## Decision

**Option C: IPv6 spine, IPv4 edge.** The mesh fabric — node identity, backhaul
next-hops, service addressing, federation — is IPv6 (ULA /48 per mesh, RFC 4193
derived); the client edge keeps the shipped IPv4 `/24` model indefinitely, gaining a
parallel v6 /64. NAT tricks are rejected on ethos. v6-only is rejected on client
reality. Status-quo IPv4 is rejected because the backhaul collision cliff and
per-service addressing have no answer inside 32 bits.

The one-line version: **IPv4 is an access technology; IPv6 is the mesh.**

## Migration phases (each independently shippable, fleet keeps working — R8)

1. **P1 — v6 spine bring-up:** derive mesh ULA /48 + node /128; assign to `br-mesh`;
   babeld announces v6; flip IPv4 client-/24 announcements to v4-via-v6 next-hops.
   Validate on the 4-node fleet: v4 client traffic unchanged, `10.254.x` now
   redundant (kept as alias). *Exit criterion: fleet routes v4 with only fe80
   next-hops.*
2. **P2 — management plane on v6:** SSH inventory → node /128s; `status` subcommand
   prints both; docs/runbooks updated. `10.254.x` demoted to recovery alias
   (same playbook as the 192.168.1.1 demotion in `659`).
3. **P3 — client /64s:** `/subnets6/` claim lane + SLAAC RA per node LAN. v6-capable
   clients get mesh-wide v6.
4. **P4 — services (e21 proper):** AAAA-first `.mesh` resolution, per-service
   addresses from the node /64.

Phases 1–2 have no client-visible change. Phase 3+ land with the e21 design pass.

## Open questions (for the e21 pass, not blockers to this decision)

- **ULA vs GUA:** ULA chosen here (no upstream dependency, censorship-resistant by
  construction). If public reachability of mesh services ever matters, a GUA prefix
  can be *added* — ULA and GUA coexist; nothing here forecloses it.
- **Per-service addresses vs node-address+port:** /64-per-node makes per-service
  /128s free; whether services get their own address or share the node's is an e21
  service-model question.
- **v4-via-v6 kernel support on every fleet target:** verify `ip route add
  <v4> via inet6 fe80::… dev br-mesh` on each shipped image during P1.
- **Does the 16-bit `/subnets6/` subnet-id reuse the v4 claim slot** (one claim
  covering both families) or claim independently? One-claim-both-families is
  simpler and keeps v4/v6 topology congruent — default answer unless e21 finds a
  reason otherwise.

## References

- RFC 4193 (ULA), RFC 9229 (v4-via-v6 in Babel), babeld ≥ 1.12 release notes
- `network-architecture.md` (shipped model), `gossip-and-crdt.md` (claim CRDT)
- Beads: `e21` (consumer), `0yb` (gossip address book — carries AAAA from day one),
  `yau` (federation — per-mesh /48s make it a prefix-exchange problem), `99f`
  (stale claims — v6 abundance lowers the stakes)
