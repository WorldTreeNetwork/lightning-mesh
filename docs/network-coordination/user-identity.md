# User Identity & the Front Desk (`hello.mesh`)

**Status:** Design spec — decided at the principle level, build sequenced after `0yb`/`e21`
**Bead:** `mjolnir-mesh-rp9` | **Date:** 2026-07-03
**Companion docs:** [membership-enrollment](membership-enrollment.md) (node identity, bead `met`),
[ipv6-addressing-decision](ipv6-addressing-decision.md) (identity lives in keys; IP is access plumbing),
[philosophical-outcomes](../vision/philosophical-outcomes.md) §5 (the network as a projection of keys)

This spec extends identity-by-key from **nodes** to **people**. It is the design
that makes "the network is a projection of a set of keys" user-facing: membership,
service access, and publish rights answered by keys and webs of trust — never by
MAC filters, shared passwords, or a server the protocol depends on.

> **Why not a captive portal?** An earlier draft framed the enrollment surface as
> a captive portal. That was the wrong mechanism, structurally: a captive portal
> only *appears* by blocking — the OS pops the login sheet when its connectivity
> probe is intercepted — so on a genuinely open network it never surfaces at all.
> And when it does surface, the Captive Network Assistant is the worst browser
> context on the device: sandboxed, storage-evicting, deep-link-hostile — exactly
> where key material shouldn't live. Deeper than the mechanics: a captive portal
> is the architecture of networks that treat users as suspects at a checkpoint.
> This mesh treats them as **guests who can walk up to the front desk whenever
> they choose**. The front desk is `hello.mesh` (§4). True captive behavior
> survives only as the enforcement face of an *optional* per-node gated policy
> (§4.5).

---

## 1. Governing principles (decided 2026-07-03)

1. **The net is open.** Basic connectivity is plug-and-play and permissionless.
   Identity *unlocks more*; it never gates the on-ramp. Anonymous use is a
   supported mode, not a degraded one.
2. **The protocol verifies only signatures.** No component of the mesh ever
   verifies a server, consults an issuer, or phones home. Every trust decision
   is a stateless, offline check of a signature chain — the same property the
   Papyrus SP-02 spike arrived at, and the same shape the `met` node
   web-of-trust already uses.
3. **Custody is a spectrum the user chooses.** Where the private key lives is a
   user decision on a sovereignty gradient (§3), from "no key at all" to
   "hardware-backed key I alone hold." Services see one uniform thing — a key
   and valid signatures — regardless of which rung the user stands on. Users
   never have to leave their comfort zone unless they want the benefits.
4. **No node ever holds a user's private key.** Custody is the user's — in
   their browser (soft) or their app/hardware (hard) — or a custodian they
   consciously chose and authenticate to. A node is a *bridge and a bootstrap*,
   never a keyholder. (This retires the earlier "node-custodied" rung; see §3.)
5. **No custom software required.** Every rung below full self-custody works
   with a stock browser. The IdentiKey app is the *best* experience, not the
   *required* one (it also doesn't fully exist yet).
6. **One trust machinery.** User identity reuses the `met` web-of-trust CRDT
   shapes (endorsements, revocations, per-device allow/block/threshold). Users
   are one more kind of subject, not a second system.
7. **Front desk, not checkpoint.** The front desk is a well-known place users
   visit by choice, never an interception they must clear. Nothing is ever
   injected into a user's traffic to summon it.

## 2. Identity model — the Papyrus/identikey-core attestation chain

Adopted from Papyrus (SP-02 spike + collaboration PRD FR1–FR7), which is being
ported into `identikey-core` on its own track:

- A **user identity** is an Ed25519 keypair (the IdentiKey root).
- A **device** holds its own keypair (iroh `EndpointId` for mesh-speaking
  devices; a browser-origin key at `hello.mesh` for legacy devices, §4).
- The identity key signs an **attestation over the device key**:
  `sig(identity, canonical(device_pubkey, issued_at, epoch, caps))` — a
  verifiable, offline-checkable link from *person* to *device*.
- Verification anywhere in the mesh is a stateless signature check. No RP
  server, no JWKS fetch, no issuer contact. (WebAuthn/passkeys were evaluated
  and rejected for exactly this reason: a passkey is bound to a DNS-domain
  Relying-Party ID and platform-vendor infrastructure — structurally
  centralized. See Papyrus `docs/spikes/SP-02-webauthn-tauri-passkey/report.md`.)
- Growth path (Papyrus FR30): per-device keys HD-derived from the identity
  root, so multi-device is a derivation, not a re-enrollment.

**CRDT shape.** A `/users/{identity_pubkey}` record mirrors
`/members/{subject}`: a grow-only set of device attestations plus
endorsements from other identities/nodes, with the same epoch-gated
revocation semantics (`membership-enrollment.md` §5). Node-trust policy
(allow/block/threshold) evaluates user endorsements identically. Guilds
(bead `6t7`, Papyrus FR34) become sets of identity keys later — same record
shapes.

## 3. The custody spectrum

The user-facing core of the design. Every rung yields "a key + signatures" to
the rest of the system; rungs differ only in who holds the private key and
what a compromise costs. **No rung places a user's private key on a node** —
that is invariant (§1.4).

| Rung | Who holds the key | Custody | UX | What it's for |
|---|---|---|---|---|
| **0 · Anonymous** | Nobody — no key | — | Connect and surf; nothing to set up | Permissionless baseline. A right, not a fallback. |
| **1 · Browser key** | The user's browser, at the `hello.mesh` origin (WebCrypto, non-extractable) | **Soft self-custody** | One tap at `hello.mesh`: "create an identity" — instant, no install | Everyday self-sovereign identity for app-less devices; ephemeral or persistent. |
| **3 · Custodial** | A key manager the user *consciously chose to trust*, authenticated OAuth/OIDC-style | Delegated | Ordinary web login (redirect from `hello.mesh` or any `.mesh` service) | "Identity for free" without self-custody burden. Hosted `identikey-core` is *one such custodian* — never a protocol dependency; its outage degrades *its* users' signing, not the mesh. |
| **4 · Self-custodied** | The user's own app / hardware key (Secure Enclave / TPM per SP-02) | **Hard self-custody** | Scan a QR at `hello.mesh`, or the app just speaks mesh natively; biometric-gated signing | Full sovereignty: endorse others, publish services, own `.mesh` names, multi-device via HD derivation. |

*(Rung numbering keeps 1/3/4 to avoid churning references; the former rung 2,
"node-custodied session key," is retired — it was the only rung that made a
node a keyholder, and WebCrypto Ed25519 now covers essentially every modern
browser, so rung 1 subsumes its niche.)*

**Soft vs. hard self-custody.** Rung 1 is *real* self-custody — the key is in
your browser, marked non-extractable so page JS cannot export it — but it is
**soft**: because any node can serve the plain-HTTP `hello.mesh` origin, the
node currently serving it is same-origin with your key and could make it sign
things *while you are on the page* (it cannot walk away with the key, but it
can misuse it in-session). Rung 4 is **hard**: the app holds the key, verifies
the node's `EndpointId` cryptographically, and no web origin is ever in the
trust base. The UI must not present soft custody as equivalent to hard.

**Climbing.** An anonymous, ephemeral, or custodial identity can be *upgraded*
by cross-signing from a higher-rung key (the old key endorses the new; service
bindings migrate). Climbing down is just abandonment.

## 4. The front desk — `hello.mesh`

`hello.mesh` is the **local landing page of the mesh you just joined**, served
per-node for its own /24 (sovereignty is structural — no shared surface, no
shared authority). A front desk does three things: it greets you and shows you
around (the **directory**, §4.1), it lets you become someone (**identity**,
§4.2–4.4), and — only where a community chooses it — it can be a door
(**gated mode**, §4.5). Users arrive in their *real* browser (full WebCrypto,
persistent storage, camera, working app deep-links), never a captive-portal
sandbox.

Per the IPv6 decision, IP is access plumbing; `hello.mesh` is the
**legacy-device bridge for identity**, exactly as the HTTP gateway is the
legacy bridge for services. Mesh-native clients (rung-4 apps, other nodes)
need it for nothing — they authenticate on the wire. It exists for the
app-less browser, which cannot speak the mesh at all.

### 4.1 The directory — making "a projection of keys" visible

`hello.mesh` renders, read-only, this node's view of the gossip address book
(`0yb`) and service registry (`e21`): who is here (identities / nodes present)
and what is published (`.mesh` services). This is a projection of converged
gossip state — no authority, per-node, converges as gossip converges. It is
also the literal, watchable form of the thesis: the page *is* the current set
of keys and services projected into this mesh. (The neighbor list is the human
front-end to the same directory the routing layer uses.)

### 4.2 Why one stable name is *required*, not just tidy

A browser-held key (rung 1) lives in the IndexedDB of **one origin**, and the
same-origin policy makes it invisible to every other origin. `wiki.mesh` and
`chat.mesh` are different origins and **cannot** read a key minted elsewhere.
So a single browser-held identity usable across mesh sites is only possible if
**one stable origin owns the key and vends assertions** to the others,
OIDC-style (relying-party sites get a token via redirect; they never touch the
key). That origin is `hello.mesh`. Consequences:

- **`hello.mesh` is the identity-provider origin** for rung-1 keys — its role
  is load-bearing, not cosmetic. This is *why* the front desk must have one
  stable name and must not be split across two (`id.mesh` + `hello.mesh` would
  fragment the keystore).
- **Free roaming persistence.** IndexedDB is keyed by origin *name*, not by
  which node answered. Every node serves the same name, so your rung-1 key
  persists in your browser as you roam between nodes.
- **This is the source of soft custody** (§3): the serving node controls the
  origin. The mitigation is honesty + non-extractable keys, and offering
  rung 4 (app) to anyone who wants the key out of the browser's reach.

### 4.3 How users find it

- **`hello.mesh`** resolves on every node — the stable, memorable front-desk
  address. Signage-friendly, sayable out loud at a venue.
- **RFC 8910 + RFC 8908** (DHCP option 114 / RA option + Captive Portal API):
  the network *advertises* the front desk's URL at lease time, so the OS can
  show a **non-blocking affordance** — "this network has a page" — without a
  single packet ever being intercepted. Discovery without gating; the
  standards world's own correction of the captive-portal mechanism.
- **Printed QR on the router** and operator signage encoding
  `http://hello.mesh` — the physical front desk.

### 4.4 What's on offer (any device, any time — not just at join)

- **"Just browse"** → nothing to do; the net was never gated (rung 0).
- **"Create an identity"** → the browser generates a keypair via WebCrypto at
  the `hello.mesh` origin, non-extractable (rung 1); ephemeral by default,
  persistent if the user opts in.
- **"Sign in"** → standard OAuth/OIDC-style web auth against the user's chosen
  custodian (rung 3); the custodian returns an attestation binding this
  session/device to the user's identity key. `.mesh` services may redirect
  here or straight to the custodian.
- **"Scan with IdentiKey"** → the page shows a QR carrying a challenge + the
  node's `EndpointId`; the app signs with the user's real key and delivers the
  attestation over the mesh (mirrors the `met` enrollment handshake, roles
  reversed) (rung 4). Rung-4 users never *need* the front desk — the app
  speaks mesh natively — but the QR upgrades any browser session to hard
  custody.
- **Operator guest QRs** (venue mode): pre-minted, scoped, expiring guest
  identities printed on paper — scan to join as that guest.

The node relays the resulting attestation into `/users/…` (or holds a
lease-scoped ephemeral record for rung 1) so the identity appears in the
directory and to services. Because the front desk is a real site and not a
join-time sheet, enrollment, upgrades, and identity management stay available
for the whole session — the desk doesn't close after check-in.

### 4.5 Gated mode — the one honest captive portal

A mesh (or a single node, for its own /24) may *choose* an identity-required
policy — a private community network. Only there does true captive behavior
turn on: connectivity probes are answered with the redirect, the OS sheet
points at the same `hello.mesh` front desk, and access opens when an acceptable
rung is presented. Blocking is legitimate exactly when blocking *is* the chosen
policy — the checkpoint exists only where the community has decided to have a
door. Open mode remains the default and the ethos.

## 5. What identity unlocks

- **Named presence**: your entry in the gossip address book (`0yb`) carries
  your identity, not just a lease — reachable by name across islands and
  sites, and visible in every node's `hello.mesh` directory.
- **Service publishing** (`e21`): a `.mesh` name claim is a signature by an
  identity key; name ownership is key ownership. Publish rights on a node are
  a caps grant in the endorsement, evaluated by that node's local policy.
- **Service ACLs**: services gate by key/guild with one verification path,
  regardless of the visitor's custody rung.
- **Cross-site reach**: your identity travels with you — the same key that
  joined at home is recognized at the venue, at whatever trust level local
  nodes' policies grant it.
- **Later — routing trust** (`661`): route-origin validation ("only the
  identity that hashes to / holds the CRDT claim on a block may announce it")
  reuses this exact attestation machinery. Most meshes cannot build this;
  we get it because the identity layer exists.

## 6. Sequencing & open items

**Build order:** after `0yb` (gossip address book — identity records and the
directory ride the same state) and the `e21` architecture pass (services are
what identity unlocks, and the directory renders the service registry;
specifying ACLs before the service model lands would be speculative). The
`identikey-core` port of Papyrus's key-based auth proceeds on its own track;
this spec consumes its `identikey-client` crate (attestation create/verify,
secure storage) as a dependency.

**Open items for the build-phase design pass:**

1. Rung-1 browser key mechanics: WebCrypto Ed25519 non-extractable keys across
   stock mobile browsers; persistence vs. private-browsing eviction; making
   the soft-custody limitation legible in the UI without scaring users off it.
2. Custodian protocol (rung 3): the attestation-request/response wire format a
   custodian implements — must be simple enough that a self-hosted custodian
   is an afternoon project (that's the anti-lock-in test); and how the
   OIDC-style redirect back into a `.mesh` origin works over plain HTTP.
3. Cross-origin assertion flow: how a `.mesh` service (e.g. `wiki.mesh`) gets a
   token from the `hello.mesh` origin — redirect + signed token vs. a scoped
   postMessage channel; token format services verify statelessly.
4. Lease↔identity binding lifetime and re-auth cadence; roaming a rung-1 key's
   *session* across nodes within an island (the key itself already roams via
   the stable origin; dovetails with the 802.11r/FT key-management spike).
5. Guest-QR minting UX and scoping vocabulary (time, bandwidth, service set).
6. Privacy: ephemeral identities must not be linkable across visits unless the
   user upgrades them deliberately; rung-1 ephemeral records are island-local,
   never gossiped mesh-wide.
7. RFC 8910/8908 client behavior survey: which OSes render the non-blocking
   affordance today, and how it degrades where unsupported (answer: signage
   and `hello.mesh` still work — the affordance is progressive enhancement).

**DWeb demo slice** (when build begins): `hello.mesh` on each node showing a
live neighbor/service directory; all rungs present (anonymous, browser key, QR
handshake; custodian mocked); publish `wiki.mesh` from a phone and watch it
appear in everyone's directory. The moment "projection of a set of keys" stops
being a slide and becomes a page the audience refreshes.
