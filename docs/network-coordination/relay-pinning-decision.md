# Decision: production n0 relays vs self-hosted relays

**Bead:** `mjolnir-mesh-e21.10` · **Status:** DECIDED 2026-07-09 — **hybrid: ship
n0 Staging relays as the default, keep `--relay` self-hosted override first-class,
build self-hosted relay + discovery only when a deployment demands it**
**Closes part of:** `e21` (service-mesh architecture pass)

## Decision

**Default to n0's relays; make relay-independence a documented, first-class
override, not a speculative build.** Concretely:

- **Local mesh stays relay-free, always.** In `--lan`/mesh mode the iroh endpoint
  is built with `RelayMode::Disabled` and mDNS-only discovery
  (`build_endpoint`, meshd). Same-island 802.11s traffic never touches a relay.
  This decision is *only* about the internet-hop / cross-site overlay.
- **Cross-site default = `RelayMode::Staging`** (real n0 relays on
  `relay.iroh.network`). This is today's shipped default and it stays.
- **`--relay <url>` (repeatable) is the sovereignty path.** It already ships
  (bead `3hs`) and maps to `RelayMode::custom(urls)`. Point it at a self-hosted
  relay and n0 is fully out of the path. `--no-relay` disables relays entirely
  (direct-only).
- **Self-hosted relay *with discovery/bootstrap* is deferred to its own bead**
  (see References), to be built when a real deployment requires relay
  independence — not speculatively.

The ethos cost of defaulting to a third party is **bounded and paid down by
architecture**, which is what makes n0-by-default acceptable for now (see below).

Revisit triggers (any one reopens this):
- A deployment with a **hard relay-independence requirement** (a jurisdiction
  where `relay.iroh.network` is blocked or its operator is untrusted, and the
  sites are NAT-hard enough that direct hole-punching alone is insufficient).
- **n0 relay policy/availability change** — rate limits, paywalling, shutdown of
  the free relays, or a terms change incompatible with the mesh's use.
- A **relay-discovery mechanism** landing elsewhere (e.g. gossiped relay URLs in
  the address book) that removes the bootstrap objection to self-hosting.

## Why the third-party exposure is bounded (the decisive framing)

The reflexive ethos read is "a censorship-resistant, sovereign mesh must not
depend on a third party in the connection path." True in general — but the iroh
relay is a *narrow* dependency, and three architectural facts shrink it to
something the escape hatch can cover:

1. **Relays carry the handshake, never the data plane.** Per
   `network-architecture.md` §Security ("No open relay"), n0 relays are used only
   for NAT-traversal hole-punch assist, not sustained forwarding. The cross-site
   **data plane is end-to-end QUIC / TLS 1.3 between the two router daemons** — no
   relay, not even a mesh router relaying datagrams, can read it. So the relay is
   **not** a confidentiality exposure: it never sees plaintext.
2. **Relay is a *fallback*, not the path.** On the same LAN, mDNS forms a direct
   path with no relay and no internet. Off-LAN, iroh hole-punches a direct path
   whenever the NATs allow, and sustained traffic then rides that direct path even
   though the relay assisted setup. The relay matters only for the residual case:
   two sites that cannot hole-punch directly (symmetric NAT / CGNAT on both ends)
   at **connection establishment** time.
3. **The exposure that remains is metadata + availability, and it already has an
   escape hatch.** What n0 *could* do is (a) observe connection-establishment
   metadata (which node-ids attempt to connect, timing) and (b) go away / be
   blocked, denying hole-punch assist to NAT-hard site pairs. Both are real ethos
   costs — but (a) is traffic-analysis-grade, not content, and (b) degrades only
   NAT-hard *new* connections, not the running data plane. Any operator who can't
   accept either sets `--relay` to their own relay **today**.

Weigh that bounded, escape-hatched cost against the alternative: standing up
self-hosted relay infrastructure now, for a fleet with **no current
relay-independence demand signal**, and solving relay *discovery* — a
bootstrap/chicken-and-egg problem (a node needs a relay to reach the gossip that
would tell it the relay URLs) — before any deployment needs it. That is the same
"deployed it and nothing used it" trap the IPv6 pass rejected.

## Options considered

| Option | Verdict |
|---|---|
| **C — Hybrid: n0 default + documented `--relay` self-hosted override + deferred discovery bead** | **ACCEPTED** — matches shipped code, keeps the sovereignty escape hatch first-class, defers speculative infra |
| A — Pin n0 production relays, no self-hosting story | Rejected: leaves no sovereignty path; makes the third-party dependency load-bearing with no exit |
| B — Self-hosted relay(s) now, as the default | Rejected: no demand signal; unsolved relay-discovery/bootstrap; operational burden (who hosts, uptime, upgrades) paid before it is needed |

Note the parallel to the IPv6 decision (`bsa`): both resolve a founding-ethos
question not by maximalism but by locating where the ethos is *actually* exposed
(there: service identity, already covered by node-ids; here: the data plane,
already E2E-encrypted and relay-independent) and paying only for what remains.

## What we accept, eyes open

- **n0 sees cross-site connection-establishment metadata by default.** Accepted:
  content is E2E-encrypted; operators who can't accept the metadata exposure use
  `--relay`. Documented, not hidden.
- **NAT-hard site pairs depend on n0 availability to *establish* new cross-site
  connections by default.** Accepted: running connections are unaffected;
  relay-independence is one flag away.
- **No relay discovery yet** — a self-hosted `--relay` is static config on each
  node. Accepted: fine for the single-operator deployments we have; the discovery
  bead lands when multi-operator self-hosting is real.

## References

- `network-architecture.md` §Security ("No open relay"; end-to-end vs per-hop
  confidentiality) — the basis for the bounded-exposure framing.
- `ipv6-addressing-decision.md` (`bsa`) — sibling ethos decision, same
  "locate the real exposure, pay only for what remains" method.
- Code: `build_endpoint` (meshd) — `RelayMode::Disabled` in `--lan`, `Staging`
  default, `custom(--relay)` override, `Disabled` under `--no-relay`.
- Beads: `3hs` (the shipped `--relay` override), `e21` (service-mesh pass),
  `e21.5` (WoT naming) and `met` (enrollment) — the same "no implicit authority"
  thread; a **self-hosted-relay + gossiped relay discovery** follow-on bead is
  filed from this decision.
