# Design Principles — What We Optimize For

**Status:** Normative | **Date:** 2026-07-06

This document states the criteria we use to *judge* architecture decisions. When a
proposal is on the table (a new routing behaviour, an addressing scheme, a roaming
mechanism), these are the axes we weigh it against. They are not aspirations layered on
top of the design; they are the reasons the design has the shape it has.

There are two load-bearing principles. The first is already documented across the vision
docs; it is restated here so the two sit together. The second — **no-config** — is the
one this document exists to make first-class, because it is the criterion we reach for
most often and it has never been written down as a rule.

---

## Principle 1 — The L3 overlay is the invariant; links are plumbing

Heterogeneous link islands stitched together by a common L3 routing layer. The overlay —
identity, routing, shared state — is permanent; everything below it (802.11s, ethernet,
60GHz, LTE, QUIC over any egress) is an interchangeable link with a metric. This is the
Internet's founding bet (Cerf & Kahn's catenet), and it is the only network design that
has scaled across five orders of magnitude without a migration.

See [philosophical-outcomes](../vision/philosophical-outcomes.md) §1–2 and
[network-architecture](network-architecture.md). This principle is settled; the rest of
this document assumes it.

---

## Principle 2 — No-config is a first-class constraint

> **The rule.** A person who is not a networking expert must be able to power on a node
> and get a working mesh — with no manual addressing, no designated controller, no
> per-node configuration. And two independently-owned meshes must be able to merge by
> proximity, with neither side reconfiguring.

This is not a usability nicety. It is the criterion that makes the network
**permissionless** and **ownerless**. A network you have to configure is a network
someone has to *administer* — and an administrator is an authority, and an authority is
the single point the whole architecture exists to remove. No-config and no-central-authority
are the same requirement viewed from two sides: *you can only remove the administrator if
the nodes configure themselves.*

So every architecture decision is weighed against two questions:

1. **Plug-in-and-it-works** — can a non-expert add a node with zero networking decisions?
2. **Merge-on-contact** — do two strangers' fleets play nicely when they come into range,
   without either side changing settings?

If a proposal breaks either, it is suspect regardless of its other merits.

### How we satisfy it: symmetric nodes + a coordination layer

The mechanism is **symmetric, non-authoritative nodes plus a coordination layer (CRDT
gossip over iroh) that replaces central authority in the *control plane*.** Everywhere a
conventional network needs a human or a server to make a decision, we derive or negotiate
it instead:

| Conventional network needs… | We replace it with… |
|---|---|
| An admin to allocate addresses | `blake3(node_id)`-derived backhaul addresses — no allocation step at all |
| A DHCP server / IPAM authority | Client `/24`s **claimed** from a shared space via the subnet-claim CRDT |
| An election or designated controller | **HLC first-writer-wins** conflict resolution — no leader, no quorum |
| Manual segment/VLAN assignment | Membership derived from the coordination plane (proximity, observed adjacency) |
| A supernode / concentrator to join sites | iroh's NAT-traversing, identity-addressed overlay — no distinguished node |

Election *would be acceptable* — a leader chosen automatically is still no-config. But
CRDT coordination is strictly better: there is no leader to fail, no term to expire, no
quorum to lose. A designated-but-auto-elected server is a soft authority; a symmetric
CRDT is no authority. We prefer the latter wherever a CRDT can carry the decision.

### The boundary: coordination is the control plane, not the data plane

This is the subtle half of the principle, and getting it wrong produces bad designs that
*look* principled.

**Coordination replaces authority in the control plane** — allocation, membership,
identity, who-owns-what. **The data plane uses the right primitive at each layer** — L2
within a local island (so a client keeps its IP when it roams across the room), babel
between islands, iroh across the internet.

We do **not** force the coordination layer to *become* the data plane. The temptation is
real: because every node already holds the full CRDT address book, it is tempting to make
the CRDT carry, say, per-client roaming — re-issue a client its old IP on whatever node it
lands on. That works on paper and fails in practice: it turns into per-host `/32` mobility
routes flooding babel, a routing-convergence gap on every handoff, and gateway-proxy
hacks. A lower layer (shared-L2 within an island) does the same job instantly and for
free.

The correct expression of "coordination tech buys no-config" is: **use the coordination
layer to *scope and configure* the data plane, not to replace it.** Auto-island formation
is the canonical example — the CRDT (control) decides the L2 boundary from observed
proximity and roaming; L2 (data) does the fast forwarding inside it; babel routes between
islands; iroh stitches sites. Each layer does what it is best at, and the coordination
layer's job is to wire them up without a human.

### What it buys (and what it costs)

Because the nodes configure themselves, **a non-expert operates the network, there is no
concentrator or supernode to run, and independently built meshes merge by linking at a
single node** (see [philosophical-outcomes](../vision/philosophical-outcomes.md) §2–3;
contrast Freifunk's fastd-tunnels-to-supernodes model in [prior-art](prior-art.md) §5–6).

The cost is honest and worth stating: our scaling limits become **administrative — address
space, trust/enrollment, directory size** — rather than physical. Those are the limits you
engineer through on a live network, and several of them (backhaul address-space size,
cross-fleet trust, subnet sizing for merges) are open questions precisely *because* we
insist on no-config. That trade — administrative limits we can grow through, in exchange
for never needing an administrator — is the one we are deliberately choosing.

---

## Applying the principles

For any proposal, run the checklist:

- [ ] **Links are plumbing** — does it assume a specific radio/link, or does it work at L3
      over any link with a metric?
- [ ] **Plug-in-and-it-works** — does adding a node require a human networking decision?
- [ ] **No designated node** — does it need a leader, controller, or concentrator (even an
      auto-elected one, where a CRDT would do)?
- [ ] **Merge-on-contact** — does it survive two independently-owned fleets meeting in
      range?
- [ ] **Right layer** — is coordination scoping the data plane, or trying to *be* it?

A design that passes all five is aligned. A design that fails one needs a very good reason.
