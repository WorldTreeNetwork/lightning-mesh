# Identity-Gated Peering — Requirements

**Status:** Source document (requirements, not design) | **Date:** 2026-07-06

## Purpose

This document captures the concrete peering and security **needs** that surfaced while
designing auto-island formation (see [island-formation](island-formation.md)), so the
identity system can be designed against real requirements instead of in a vacuum. It
states *what we need and why* — each requirement traces to a specific scenario we hit —
and defers *how* to the identity design pass.

It feeds the existing identity/trust cluster: `rp9` (IdentiKey user identity), `met`
(DPP-style QR membership enrollment / per-device roots), `661` (authenticated babel
announcements), `e21.5` (web-of-trust `.mesh` name arbitration). It is governed by the
no-config / permissionless criterion in [design-principles](design-principles.md) §2.

## The core problem

The founding premise is a **permissionless, no-config mesh**: a stranger's node fleet can
plug in near yours, peer over the shared backhaul, and play nicely — reachable both over
local backhaul and over iroh. That premise puts *untrusted peers on a shared backhaul*,
and today we have no way to make that safe. Two current mechanisms are dead ends for it:

- **The shared 802.11s fleet secret (SAE passphrase) is insider-complete.** SAE encrypts
  each *link*, but the passphrase is the *entire* trust boundary. Anyone holding it is a
  **full mesh peer** — not merely able to decrypt. It is not per-fleet separable; one leak
  admits anyone; and encryption gives *nothing* against a peer that has the secret. Good
  for a single-owner fleet, structurally unsafe for mixed ownership.
- **`blake3(node_id)` address derivation always had an expiration date** — a 16-bit
  backhaul host space (`10.254.0.0/16`) hits ~50% birthday-collision odds around 300
  nodes. Fine per-fleet, insufficient as a global permissionless identity/address basis.

**Identity is the unlock.** iroh already gives us an Ed25519 **node id**, and the CRDT
gossip plane over iroh is *already transport-authenticated* by it (a sniffer can't read
gossip; a joiner can't forge an id). The gap is not authentication of the iroh plane — it
is (a) **authorization** (what an authenticated identity is *allowed to do*) and (b) the
**802.11s / babel local plane**, which does not ride iroh and is currently unauthenticated.

## Requirements

Each is stated with the concrete need that produced it.

### R1 — Per-identity peer authentication (replace the shared secret)

Peers must authenticate by their **own** cryptographic identity (Ed25519 node id), not by
possession of a shared passphrase. **Why:** the shared-secret model can't separate fleets,
can't survive a leak, and treats every secret-holder as fully trusted. Per-identity auth is
the precondition for every authorization decision below.

### R2 — Per-identity *authorization*, not binary join

Joining must not be all-or-nothing. An authenticated identity needs distinct, separately
grantable authorizations, because we found distinct attack surfaces each needing its own
gate:

- **R2a — Route announcement (babel).** babel has no crypto auth by default; a peer can
  advertise/withdraw routes and blackhole or redirect *any* traffic. **Gate:** authenticated
  babel with route-origin validation by node id. *This is bead `661`, and it is the
  foundational gate — see "Gating open backhaul" below.*
- **R2b — Subnet / prefix claim (CRDT).** An untrusted peer can claim or poison client
  subnets in the subnet-claim CRDT. **Gate:** only authorized identities' claims are
  accepted / merged.
- **R2c — Client-L2 / island VNI membership.** Joining an island's shared client L2
  exposes client traffic and broadcast to the joiner. Membership must therefore be an
  **authorized set**, not a physical fact (whoever is in RF range). **Why this matters:**
  it is the hinge decision for the island data plane — it is exactly why we chose an
  authorized-VNI overlay (VXLAN/EVPN-lite) over batman-adv, whose fabric membership is
  unauthenticated and physical. See [island-formation](island-formation.md).
- **R2d — CRDT lane writes generally.** Address book, `/services/`, `/dns/` names — all
  poisonable/squattable by an untrusted writer. **Gate:** per-identity write authorization,
  composing with the existing HLC first-writer-wins (identity-scoped FWW).

### R3 — Signed capability / compatibility descriptor

Before committing to a merge, a node must learn whether a mesh it hears is *compatible*
(protocol version, address-space constants, CRDT topic) and *trusted*. **Why:** ad-hoc
plug-in peering can otherwise cause silent address collisions or unsafe joins. Requirement:
a **signed** descriptor, advertised **pre-association** (802.11s beacon vendor IE, so a scan
sees "mjolnir vN, fleet X, topic Y" before joining) and **confirmed** in the iroh/CRDT
handshake, driving a decision tree: compatible+trusted → merge; compatible+untrusted →
gated (enrollment offer); incompatible/untrusted → don't merge L2, degrade to L3-gateway
+ NAT (which is the same border-node pattern as foreign-mesh interop, e.g. Freifunk —
`prior-art.md` §5–6). *This is the capability-beacon bead.*

### R4 — Enrollment / trust establishment

There must be a way for an identity to *become* authorized, permissionlessly but with
mitigations. **Why:** admitting strangers is the whole point; doing it safely needs an
onboarding path. Existing primitives: `met` (DPP-style QR, per-device roots), web-of-trust.
Manual authorization is acceptable as the interim; automated admission is a later goal.

### R5 — Reputation / revocation hook

Authorization is not static — bad actors must be **evictable**, and the eviction must
propagate. **Why:** VNI/island membership and `.mesh` name ownership both need "you're out"
to be enforceable after the fact. A **reputation layer** (planned, currently undocumented)
is the intended automated mechanism — governing island membership eviction *and* DNS name
ownership (relates to `e21.5` WoT name arbitration). Requirement on identity: revocation is
a first-class, CRDT-propagated state, not just an allow-list add.

### R6 — Graceful degradation, not hard rejection

An unauthorized or incompatible peer should not simply be refused — it should fall back to
**L3 gateway + NAT** interop. **Why:** this preserves connectivity, and it is the *same*
mechanism we use to interoperate with foreign meshes. Rejection and interop are one code
path.

## Gating open backhaul

Open (unencrypted, join-anyone) backhaul is attractive for permissionless peering, and its
danger is *narrower than it looks* — the client SSID is already open, so first-hop client
air traffic is exposed regardless; the CRDT plane (over iroh) and cross-site data (E2E over
iroh) are already safe. The real exposure is concentrated in the **control plane**.
Therefore:

> **Open backhaul is acceptable once, and only once:**
> 1. **babel control is authenticated** — route-origin validation by node id (`661` / R2a),
>    so an RF joiner cannot inject routes; **and**
> 2. **CRDT writes are identity-authorized** — subnet claims and lane writes gated by
>    identity (R2b, R2d), so a joiner cannot poison shared state.

Until both hold, backhaul must stay closed (shared-secret, trusted-only). These two
conditions are the concrete unblock list for open backhaul.

## What we already have vs. the gap

| Have | Gap |
|---|---|
| Ed25519 node id (iroh) | Authorization model (what an id may *do*) |
| CRDT gossip transport-authenticated over iroh | 802.11s/babel local plane is unauthenticated (`661`) |
| HLC first-writer-wins conflict resolution | Identity-scoped FWW (whose write wins by *authorization*, not just time) |
| Enrollment primitive (`met`, WoT) | Reputation / revocation propagation (R5) |

## Cross-references

- [island-formation](island-formation.md) — the design pass that produced these needs
- [design-principles](design-principles.md) §2 — the no-config / permissionless criterion
- [prior-art](prior-art.md) §5–6 — foreign-mesh interop (the R6 degradation target)
- Beads: `rp9`, `met`, `661`, `e21.5`, `2km`
