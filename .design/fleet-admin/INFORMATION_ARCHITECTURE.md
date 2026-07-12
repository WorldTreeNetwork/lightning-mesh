# Information Architecture: Mjolnir Fleet Admin

> Structural layer for the hidden operator console layered on the public
> `hello.mesh` front desk. Produced via the information-architecture skill.
> No prior `DESIGN_BRIEF.md` existed; the framing below is derived from the
> operator request and codebase exploration and should be promoted into a
> brief if visual design follows.

## Context & Constraints (from codebase exploration)

The IA is bounded by what the mesh already is. These are load-bearing facts, not
preferences:

- **The front desk is public and symmetric.** `mjolnir-hello` serves a SvelteKit
  bundle + a read-only Rust API (`/api/health`, `/api/directory`, `/api/node`,
  `/api/radio`, `/api/captive-portal`) plus write endpoints for identity
  (`/api/challenge`, `POST /api/identity`, `POST /api/name-claim`). Everyone on
  the open SSID reaches it.
- **A signed-challenge primitive already exists.** `GET /api/challenge` issues a
  single-use nonce; `POST /api/identity` validates an ed25519 signature over it.
  This is the crypto seam admin auth is built on — no new auth scheme needed,
  only a new *authorization* check (is the signer an owner?).
- **Nodes are reached over the overlay.** Each node has a derived
  `10.254.<blake3(node_id)>` address, routed mesh-wide over babel; the LAN
  gateway is `10.42.<x>.1`. CORS is already enabled on `GET /api/*` precisely so
  one node's browser view can aggregate other nodes' telemetry.
- **You land on an arbitrary node.** Connecting to the shared open SSID
  associates you with whichever node has the best signal — possibly a foreign
  operator's node. The console must work from *any* entry node.
- **No ownership model exists yet.** Fleet membership is implicit (shared mesh
  key + CRDT topic). `owners` claims and STA/client wireless mode are **new
  capabilities** this IA assumes will be built; they do not exist today.
- **Config changes are transactional.** Disruptive changes go through
  `mjolnir-apply` (snapshot → apply → health-gate → auto-rollback). The IA's
  "revert" affordances map onto this, never onto a live inline mutation.

### Decisions locked in the interview

| Decision | Choice |
|----------|--------|
| Admin entry | **Secret-knock reveal**, then key-signed challenge. No visible admin link; a gesture/keyword on the public page reveals the entry, which still requires an operator-key signature. |
| Fleet model | **Symmetric peer-controller.** The entry node renders the whole fleet from the CRDT directory and proxies admin actions to any owned node over the `10.254.x` overlay. No elected controller. |
| Ownership | **Operator-key ownership claim.** Each node records `owners` pubkeys, gossiped via CRDT. "My nodes" = nodes whose `owners` include one of your keys. Foreign nodes are visible in topology but refuse admin. |
| First job | **Foreign-SSID uplink → Ethernet.** A bridge node joins a foreign WiFi (operator enters SSID + password) as a station and feeds that uplink over Ethernet to a Lightning Mesh node. |

### Primary users

- **Fleet operator (you).** Holds one or more operator keys. Needs to find,
  reach, and reconfigure owned nodes from any entry point, safely and
  reversibly. ~100% of admin-console usage.
- **Guest / resident (public).** Never sees the console. Uses the front desk to
  get online, claim a `.mesh` name, and see who/what is nearby. Present here only
  to establish the boundary the admin area hides behind.

---

## Site Map

Public and hidden surfaces share one origin (`http://hello.mesh/` on the entry
node's LAN gateway). Hidden routes render nothing without a valid operator
session; they are unlisted, not merely styled differently.

- **Front desk (public)** `/`
  - Hero + custody notice — *(existing)*
  - Identity / name-claim — *(existing `IdentityManager`)*
  - Routers / People / Services / Radio panels — *(existing)*
  - _Secret-knock target_ — reveals admin entry (no URL of its own)
- **Admin entry** `/admin` *(unlisted; revealed by knock)*
  - Challenge + key-signature prompt → establishes operator session
- **Fleet console** `/admin/fleet` *(the 80% view)*
  - Fleet map / node list — owned nodes first, foreign nodes muted
  - Fleet-wide health & alerts strip
- **Node detail** `/admin/node/:nodeId`
  - Overview — identity, addresses, uplink, health
  - Wireless `/admin/node/:nodeId/wireless`
    - **Bridge uplink** `/admin/node/:nodeId/wireless/bridge` — *(the first job)*
  - Network `/admin/node/:nodeId/network` — LAN `/24`, DHCP, routes
  - Radio `/admin/node/:nodeId/radio` — AP + mesh backhaul settings
  - Ownership `/admin/node/:nodeId/owners` — operator keys allowed to admin
  - Change history `/admin/node/:nodeId/changes` — apply/rollback log
- **Fleet settings** `/admin/settings`
  - My keys — operator keypairs held in this browser
  - Inventory / naming — labels for owned nodes
- **Session** `/admin/session` — sign out, active session info

---

## Navigation Model

- **Primary navigation (console):** A persistent left rail once inside `/admin`:
  **Fleet**, **Settings**, **Session**. Deliberately tiny — the work happens in
  the fleet map and node detail, not in top-level sections.
- **Secondary navigation:** Within **Node detail**, a horizontal tab set
  (Overview · Wireless · Network · Radio · Owners · Changes). The **node
  switcher** (a combobox in the header listing owned nodes) is the primary
  hop-between-nodes control — selecting a node re-targets the current tab at that
  node's overlay address via the entry node's proxy. This is the UniFi/RouterOS
  "device picker" pattern.
- **Utility navigation:** Header shows **you-are-here** (entry node id + "you are
  connected to this node"), the **operator-key indicator** (which key is signed
  in), and **sign out**. The public front desk keeps its own unchanged nav; the
  console nav only exists behind auth.
- **Mobile navigation:** The operator is often on a phone on the open SSID. Left
  rail collapses to a bottom bar (Fleet · Node · Settings); node-detail tabs
  become a horizontally scrollable segmented control; the node switcher becomes a
  full-screen searchable sheet. The bridge-uplink form is single-column and
  thumb-reachable — it is the one thing you may need to do standing next to a
  router.

**Depth budget:** 3 levels of interactive depth max (Fleet → Node → Tab). The
bridge job sits one click deeper (Tab → Bridge) but is also surfaced as a
first-class action card on Node Overview so the common job is never buried.

---

## Content Hierarchy

### Fleet console (`/admin/fleet`) — the 80% view
1. **Owned nodes, by health** — name, node id, online state, uplink kind,
   client count. The operator's whole job starts here; sick nodes float up.
2. **Fleet health strip** — count online/offline, any node in rollback, any
   degraded uplink. Answers "is anything on fire?" at a glance.
3. **Entry-node banner** — "You are connected through **node B**." Sets the
   mental model that actions are proxied, and warns if the entry node itself is
   foreign (you're a guest here but administering your own nodes elsewhere).
4. **Foreign nodes (muted, collapsed)** — visible for topology/context, not
   actionable. Reinforces "not everything here is yours."

### Node detail — Overview (`/admin/node/:nodeId`)
1. **Identity & reachability** — name, node id, `10.254.x` overlay + `10.42.x.1`
   LAN, online/last-seen. Confirms you're about to change the *right* box.
2. **Uplink status** — how this node reaches the internet (mesh backhaul / wired
   / **bridge uplink**), with the bridge-uplink action card promoted here.
3. **Health & current transaction** — health-gate status; a prominent
   **"pending change — auto-reverts in Ns"** state if an apply is mid-flight.
4. **Owners** — which operator keys may admin this node (link to Owners tab).
5. **Recent changes** — last few apply/rollback events (link to Changes).

### Bridge uplink (`/admin/node/:nodeId/wireless/bridge`) — the first job
1. **Current mode** — plain statement of what the node is now (e.g. "Mesh node,
   internet via backhaul") vs. the target ("Wireless bridge to an upstream SSID").
2. **Uplink WiFi form** — **SSID** (with a scan/pick-from-list helper),
   **password**, band/security. The core inputs the operator asked for.
3. **Downstream target** — which Ethernet port carries the uplink to which
   Lightning Mesh node (the neighbor being fed). Confirms the physical topology.
4. **Apply & safety** — an **Apply** that runs through `mjolnir-apply` with an
   explicit health-gate window, and a persistent, always-visible **Revert to
   mesh node** control. Reversibility is co-equal with configuration, not a
   footnote — the operator explicitly asked to "easily revert."
5. **Draft/status feedback** — connecting → got IP → gate passed / gate failed →
   auto-reverted, streamed so the operator watching the router knows what happened.

### Owners (`/admin/node/:nodeId/owners`)
1. **Authorized keys** — operator pubkeys that may admin this node.
2. **Add/remove** — add a key (paste/scan), remove a key, with a guard against
   removing your last own key (self-lockout).
3. **Provenance** — when/by which key each owner was added (from CRDT).

---

## User Flows

### Reveal & enter the admin console
1. Operator opens `http://hello.mesh/` on the open SSID — the ordinary public
   front desk.
2. Operator performs the **secret knock** (defined gesture/keyword — e.g. a
   tap-sequence on the logo or a typed keyword in the name field).
   - If knock unrecognized → nothing happens; page stays fully public (no hint
     the admin area exists).
   - If recognized → admin entry (`/admin`) is revealed.
3. Entry node issues a **challenge** (`GET /api/challenge`).
4. Operator's browser signs it with an **operator key** held locally and submits
   (`POST /api/identity`-style admin variant).
   - If signature invalid → refused, back to public page.
   - If valid → **operator session** established; redirect to `/admin/fleet`.
5. Operator lands on the **Fleet console**.

### Hop to one of my nodes (symmetric peer-controller)
1. From Fleet, operator sees owned nodes drawn from the **CRDT directory**.
2. Operator selects **node K** (via the map or the node switcher).
3. The entry node **proxies** the admin request to `10.254.K` over the overlay,
   re-attaching the operator's session/authorization.
   - If K's `owners` include the operator's key → K returns config; Node detail
     renders.
   - If not owned (foreign) → admin refused; node shown read-only in topology.
   - If K unreachable over overlay → "node offline / unreachable" state, retry.
4. Operator works on K without ever having physically associated to K's radio.

### Configure a wireless bridge uplink (the first job) + revert
1. Operator opens **Node detail → Bridge uplink** for the node that will become
   the bridge.
2. Operator reads **Current mode** ("Mesh node") and confirms the **downstream
   target** (Ethernet → the Lightning Mesh node being fed).
3. Operator enters the **upstream SSID** (optionally via scan-and-pick) and
   **password**.
4. Operator taps **Apply**.
   - Change is staged and applied via `mjolnir-apply` (snapshot taken first).
   - **Health gate** watches for a working uplink within the window.
     - Gate passes → node is now a bridge; status shows "Bridge active, uplink
       via <SSID>"; the snapshot is retained for manual revert.
     - Gate fails (bad password, AP gone, no route) → **auto-rollback** to the
       snapshot; status shows "Reverted — bridge did not come up," with the
       failure reason. The operator is never stranded.
5. Later, operator taps **Revert to mesh node** → node restored to its prior
   snapshot; confirmation shown. (This is the same rollback path, invoked
   manually rather than by gate failure.)

### Grant another operator key admin of a node
1. Operator opens **Node detail → Owners**.
2. Adds a pubkey (paste/scan) → change gossiped via CRDT to the node's `owners`.
3. That key can now pass the admin authorization check on this node.
   - Guard: removing your own last key requires explicit confirm (self-lockout
     warning).

---

## Naming Conventions

Pick one word per concept and use it everywhere — the public and admin surfaces
must not drift.

| Concept | Label in UI | Notes |
|---------|-------------|-------|
| A physical router | **Node** | Consistent with the codebase; never "device" or "router" in-console (RouterOS calls them devices — we don't). |
| The node you associated to | **Entry node** / "You're connected through …" | Distinguishes the proxy from the target. |
| The node you're editing | **Selected node** | Set by the node switcher. |
| The whole set of your nodes | **Fleet** | Matches operator/industry language (UniFi "fleet"). |
| Nodes you may admin | **My nodes** / **Owned** | Determined by `owners` key match. |
| Nodes you can see but not admin | **Foreign** | Neutral, not "someone else's" — just not yours. |
| The operator identity | **Operator key** | Not "account" — there is no account, only keys. |
| Joining a foreign WiFi for uplink | **Bridge uplink** | Not "WISP" / "repeater" / "STA" in UI; those are implementation terms. |
| The foreign network being joined | **Upstream WiFi** / **Upstream SSID** | "Upstream" = toward the internet. |
| Undoing a change | **Revert** | One word for both manual revert and auto-rollback (label auto-rollback "Reverted automatically"). |
| The safe apply mechanism | **Apply** (verb) | The health-gated transaction; never a silent inline save. |

---

## Component Reuse Map

| Component | Used on | Behavior differences |
|-----------|---------|---------------------|
| Existing `shadcn` card / badge / button / collapsible | Whole console | Same primitives as the public front desk — one visual system across public + hidden. |
| **NodeCard** | Fleet map, node switcher, foreign list | Owned → actionable + health; foreign → muted, non-interactive. |
| **NodeSwitcher** (combobox) | Every node-detail header | Lists owned nodes; drives the overlay-proxy re-target. |
| **HealthGate status** | Node overview, Bridge apply, Changes | Same "pending / passed / reverting" states everywhere an apply happens. |
| **KeySignaturePrompt** | Admin entry, Owners (add key) | Reuses the existing `/api/challenge` + identity flow; entry = authenticate, Owners = authorize a new key. |
| **RadioGraph / topology** (existing) | Public front desk **and** Fleet map | Public shows neighbors read-only; Fleet map overlays ownership + admin affordances on the same graph. |
| **ProxyBanner** ("connected through node B") | Every admin view | Constant reminder that actions are proxied, not local. |
| **RevertControl** | Bridge uplink, any transactional tab | Always visible while a snapshot exists; identical semantics per surface. |

Reusing the public front desk's component set (cards, badges, RadioGraph) keeps
the hidden console from looking like a bolted-on second app and lets the topology
view do double duty.

---

## Content Growth Plan

- **Nodes (grows with the fleet):** Fleet map + list must scale from 1 to
  dozens. Provide **search/filter** in the node switcher and Fleet list (by name,
  online state, uplink kind) and group **My nodes** above **Foreign**. Foreign
  nodes are collapsed by default so a busy venue doesn't drown the operator's own.
- **Config surfaces (grows with capability):** Node-detail tabs are the extension
  point. Bridge uplink is the first Wireless sub-page; future jobs (band steering,
  channel plan, guest-network policy) become sibling sub-pages without disturbing
  the top-level nav. Keep top-level nav fixed at Fleet/Settings/Session.
- **Change history (append-only, per node):** The Changes log grows monotonically;
  paginate and keep only a rolling window surfaced, with older entries behind
  "load more." Rollback events are first-class entries, not hidden.
- **Operator keys (grows slowly):** Settings → My keys and node Owners lists stay
  short; no special scaling needed, but design for 2–5 keys (rotation, a second
  device) not just one.

---

## URL Strategy

- **Pattern:** `/admin/<section>` and `/admin/node/<nodeId>/<tab>`. Public front
  desk stays at `/` and its existing routes; the admin tree is a sibling subtree.
- **Node identity in URL:** Use the **node id** (stable, blake3-derived), not the
  overlay IP, so a link survives address recomputation and is portable across
  entry nodes. The entry node resolves id → `10.254.x` at proxy time.
- **Dynamic segments:** `:nodeId` (the selected/target node). Everything else is
  static section/tab names.
- **Query parameters:** `?filter=`, `?online=`, `?uplink=` on the Fleet list for
  the growth case; not used for auth state (auth lives in the session, never in
  the URL).
- **Unlisted, not secret-via-URL:** `/admin*` renders only with a valid operator
  session; the secret **knock** is what reveals the entry, and the **key
  signature** is what authorizes. The URL being guessable is fine — it's inert
  without the signature. Do not encode any secret in the path.
- **Robots:** `/admin*` stays out of any listing; the existing `robots.txt`
  should disallow it (defense in depth; it's already unlinked).

---

## Open Questions for the Build (flagged, not resolved here)

These are capability gaps the IA depends on — surface them into beads before
implementation:

1. **`owners` CRDT claim** — schema, who may set the *first* owner (trust on
   first use? physical-console bootstrap?), and how self-lockout is prevented.
2. **STA/client wireless mode** — the bridge uplink requires wpa_supplicant
   client + bridge/route to Ethernet on OpenWrt; none of this exists today.
3. **Admin proxy over overlay** — the entry node must forward authorized admin
   calls to `10.254.x` peers; today `/api/*` is read-only. Needs an authenticated
   node-to-node admin channel (not the public CORS GET surface).
4. **Secret-knock definition** — exact gesture/keyword and its discoverability
   trade-off (must be memorable to you, invisible to guests).
5. **Session model** — how the signed challenge becomes a bearer/session for the
   duration of an admin visit on a public-facing origin.
