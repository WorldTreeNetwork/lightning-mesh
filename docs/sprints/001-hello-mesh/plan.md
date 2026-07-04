---
sprint: sprint-001
slug: hello-mesh
product: hello.mesh
ceremony: standard
mode: beads
status: planned
created: 2026-07-04
prd: docs/products/hello.mesh/prd.md
architecture: docs/network-coordination/hello-mesh-service.md
---
# Sprint 001 — hello.mesh Front Desk (MVP)

**Goal:** ship the DWeb-demo MVP of `hello.mesh` — a node-hosted front desk whose
directory shows mesh-wide neighbors, identities, and services propagated across
the **802.11s island**, with anonymous access and one-tap soft identity, fully
offline. Cross-*site* over iroh is Growth, not a demo gate.

Work items live in **beads** (epic `bc7`); this doc is the planning narrative
(standard ceremony). It records the story map, dependency order, key decisions,
and readiness — it does not duplicate the PRD (`prd.md`) or the architecture
(`hello-mesh-service.md`).

## Key decisions (ADR-lite)

- **D1 [HIGH] Cross-mesh propagation is MVP, island-first.** A node-local
  directory is worthless; the product is the mesh making distant people/services
  present. Rides the field-validated gossip transport (subnet claims already
  converge mesh-wide). Cross-site over iroh is Growth (likely near-free).
- **D2 [HIGH] `mjolnir-hello` is a separate crate/binary,** not folded into
  `mjolnir-meshd`. Keeps the router core lean; links the iroh-free `mjolnir-mesh`
  library types to decode state.
- **D3 [HIGH] Daemon is the single writer of gossip state;** `hello` only reads
  (`directory.json`) and relays identity submissions to a `pending/` spool. No
  node holds a user private key.
- **D4 [MEDIUM] File seams, not IPC:** a written `directory.json` projection and
  a read spool dir, matching the daemon's existing atomic-persist pattern.
- **D5 [MEDIUM] `tiny_http` + `rust-embed`** for a small aarch64 static binary
  (axum is the noted ergonomic alternative).
- **D6 [HIGH] Soft-custody honesty is a hard AC:** the browser key is pure-JS
  `@noble/ed25519` (plain HTTP is an insecure context — no WebCrypto), stored in
  IndexedDB, and the UI must label it as soft (extractable by the serving node).
- **D7 [dependency] DNS/`.mesh` resolution is owned by `e21.1`;** the MVP is
  reachable by the node's LAN IP so it does not block on that track.

## Story map (epic `bc7`)

**Track A — Daemon: gossip record types + propagation (critical path):**
- `2xd` spike — prove one new record type (`/users`) gossips A→B. *(assumed pass)*
- `S1` — `/users` CRDT record type + gossip sync + LWW merge (productionize the spike).
- `p6u` — spool ingest: watch `pending/`, verify, write `/users`, remove.
- `S2` — service-record gossip: replicate `ServiceEntry` mesh-wide (focused `e21` slice).

**Track B — Daemon: read seam:**
- `avs` — write versioned read-only `directory.json` = {node, neighbors, identities, services}, atomic.

**Track C — Server (`mjolnir-hello`):**
- `bl2` — new crate scaffold: serve embedded bundle (rust-embed) + `tiny_http` + procd + `/api/health`.
- `S3` — read-only API: `/api/directory`, `/api/node` from `directory.json`.
- `S4` — identity API: `/api/challenge`, `POST /api/identity` (verify → spool).

**Track D — Frontend (SvelteKit SSG):**
- `gad` — SSG scaffold + offline bundle (no external hosts) + embed pipeline.
- `S5` — directory page: mesh-wide neighbors/identities/services + "you are here".
- `S6` — identity affordances: anonymous browse; create soft key; challenge/submit; honesty labeling.

**Track E — Integration:**
- `S7` — LAN-IP reachability + deploy/procd wiring + health-gate.
- `S8` — E2E cross-mesh demo validation on a two-node fleet (acceptance).

## Dependency order

```
2xd ─┬─▶ S1 ─┬─▶ p6u ─────────────┐
     └─▶ S2 ─┴─▶ avs ─▶ S3 ─▶ S5 ─┤
bl2 ─┬─▶ S3                        ├─▶ S8 (e2e acceptance)
     ├─▶ S4 ─▶ S6                  │
     └─▶ S7 ────────────────────── ┘
gad ─┬─▶ S5
     └─▶ S6
```

Parallel at start (once `2xd` passes): Track A (S1/S2), `bl2`, and `gad` run
concurrently. `gad`/frontend can develop against a mocked API until S3/S4 land.

## Status (2026-07-04)

- **Spike `2xd`: DONE ✅** (commit `ed011d2`). The `/users` record type, gossip
  `UserUpdate`, `merge_user()` LWW, and the e2e convergence/LWW/stale-discard
  tests all landed in the **lib** — Mac-testable, no daemon feature. `S1` (`zhg`)
  is therefore shrunk to **daemon wiring only** (apply `UserUpdate` → `UserBook`
  + anti-entropy re-broadcast in the meshd run loop).
- **Ready now:** `zhg` (S1 wiring), `7jb` (S2), `bl2`, `gad`.
- **Build constraint (`hg0`):** `mjolnir-meshd` is **Linux-only** (deep
  rtnetlink/tun deps) — daemon-track stories build/test on Linux or via
  `deploy/openwrt/build.sh`; their record/merge/projection *logic* lives in the
  lib and is Mac-testable (as the spike proved). `mjolnir-hello` + frontend build
  on any host (the lib is iroh-free). `hg0`'s "rename" diagnosis was wrong (a
  platform-cfg papercut, not a regression); left for the user to wontfix-by-design.
- **Field validation** of `/users` on physical 802.11s is filed as `2uq` (the
  analog of what `0yb` got for the address book), depending on the daemon-wiring
  stories.

## Readiness

- **Gate:** the spike gate is cleared. Execution proceeds on user confirmation.
  If daemon wiring hits trouble, the fallback remains: talk leads + island-local
  directory-only demo (drop S1/p6u/S2 from the demo path).
- **FR coverage:** every MVP FR (FR1–22, FR28) maps to ≥1 story — see the
  traceability note on epic `bc7`.
- **Biggest risk:** Track A (propagation) is the critical path and the only
  unbuilt-foundations work; it is front-loaded and gated on the spike.
- **Test tiers:** Track A + S8 = thorough (gossip correctness / acceptance);
  server + frontend = smoke.
