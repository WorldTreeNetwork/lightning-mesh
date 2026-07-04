# `hello.mesh` — Implementation Design

**Status:** Implementation design (build-floor) | **Bead:** `mjolnir-mesh-rp9` (parent), epic below
**Date:** 2026-07-04
**Design spec:** [user-identity.md](user-identity.md) (what `hello.mesh` *is* and why)
**Depends on (external track):** `.mesh` name propagation / DNS resolution — being
designed separately. This doc stops at the seam (§5) and does **not** design the resolver.

This is the concrete build design for the front desk: the static frontend, the
thing that serves it, and the read-only HTTP/REST API over mesh state. It is
deliberately minimal and demo-oriented (DWeb), and it keeps the router daemon
(`mjolnir-meshd`) untouched except for one read-only state projection and one
ingest spool.

---

## 1. Components (three, plus two daemon seams)

```
  browser ──HTTP──▶  mjolnir-hello (new binary)
                      ├─ serves embedded static bundle (SvelteKit SSG)
                      └─ /api/*  read-only JSON + identity ingest
                              │  reads          │ writes
                              ▼                 ▼
                     directory.json        pending/*.json      ◀── daemon seams
                       (daemon writes)     (daemon ingests)
                              ▲                 │
                              └──── mjolnir-meshd (unchanged core) ───┘
                                    owns all gossip/CRDT state
```

1. **Frontend** — a SvelteKit app built with `@sveltejs/adapter-static`
   (`prerender = true`): a pure static HTML/CSS/JS bundle, no Node runtime. It
   renders the directory and the identity front desk client-side by calling
   `/api/*`.
2. **`mjolnir-hello`** — a new, small Rust binary (new workspace crate
   `crates/mjolnir-hello`) that serves the embedded static bundle and the
   `/api/*` endpoints. Separate from the daemon so it never bloats the router
   image's core and can be built/deployed/skipped independently.
3. **REST API** — read-only JSON over mesh state, plus one write endpoint that
   spools identity submissions for the daemon to ingest.

**Daemon seams (small, additive, non-breaking):**
- **Read:** `mjolnir-meshd` periodically writes a read-only **`directory.json`**
  — a projection of its live gossip snapshot (neighbors now; services as `e21`
  lands). Single writer, atomic replace, same pattern as `persist_claims`.
- **Write:** `mjolnir-hello` drops identity submissions into a **`pending/`
  spool dir**; the daemon ingests, validates, gossips, and removes. Keeps
  `hello` a thin file-shuffler with no IPC protocol to design.

Why files, not an IPC socket: the daemon already is the single writer of a
postcard snapshot and has a status/inspection precedent; a JSON projection plus
a spool dir reuses that shape, keeps `hello` a pure reader/relayer, and matches
the spec's framing of the directory as a *read-only projection* of gossip state.

## 2. Frontend — SvelteKit SSG

- **Adapter:** `@sveltejs/adapter-static`, `export const prerender = true`,
  SPA fallback for the client-rendered directory. Output is `build/` — static
  files only.
- **Crypto:** `@noble/ed25519` (tiny, audited, pure-JS). Required because plain
  `http://hello.mesh` is an **insecure context** → no WebCrypto (`crypto.subtle`
  is `undefined`); `crypto.getRandomValues` *is* available, which noble uses.
  This is the soft rung-1 key path (user-identity.md §3). Non-extractable
  WebCrypto keys only exist in the extension/app tiers, which are post-demo.
- **Pages (v1):**
  - `/` — the directory: neighbors (nodes present) and, as `e21` lands,
    services. Polls `/api/directory`.
  - identity affordances inline: "just browse" (nothing), "create an identity"
    (generate a noble keypair, store in IndexedDB, POST to `/api/identity`).
  - "how this works" / honesty copy about soft custody.
- **Bundle discipline:** no external hosts (CSP-friendly, works fully offline);
  everything inlined or same-origin. This is a network that may have no internet.

## 3. `mjolnir-hello` — the server

- **New crate** `crates/mjolnir-hello`, binary `mjolnir-hello`. Depends on the
  `mjolnir-mesh` **library** (default features, iroh-free) for the CRDT type
  definitions so it decodes/serves the same shapes without pulling iroh/tokio's
  daemon stack.
- **Static assets:** embedded via `rust-embed` (compiled into the binary) →
  one self-contained artifact, nothing extra to stage on the router.
- **HTTP:** start with `tiny_http` (minimal deps, small aarch64 static binary,
  blocking + a small worker pool — concurrency needs are a venue's browsers
  hitting a static page and polling JSON, not high throughput). `axum` is the
  ergonomic alternative if we later co-host more; noted, not chosen for v1.
- **Config:** reads paths from the same `/etc/config/mjolnir` UCI / CLI flags
  convention — `--directory-file`, `--spool-dir`, `--static-root` (optional
  override of the embedded bundle for dev), `--bind` (default the node's LAN
  gateway IP:80).
- **procd service** on OpenWrt, alongside `mjolnir-meshd`; optional (a node can
  run the mesh without the front desk).

## 4. REST API (read-only + one ingest)

All JSON, same-origin, no auth for reads (the directory is public by design).

- `GET /api/directory` → `{ node: {...self}, neighbors: [...], services: [...] }`
  — served straight from `directory.json`. Neighbors from the address book
  (`AddrBook`: node_id, addrs, relay, announced_at) + subnet claims; services
  from the service registry (`ServiceEntry`) once `e21` populates it. `hello`
  may cache the file with a short TTL and stat-poll for changes.
- `GET /api/node` → this node's own identity/summary (node_id, claimed /24,
  backhaul addr) for the "you are here" header.
- `GET /api/health` → liveness for the deploy health-gate.
- `POST /api/identity` → body: `{ pubkey, sig, challenge, label? }`. `hello`
  verifies the signature shape, writes `pending/{pubkey}.json`, returns the
  accepted record. The **daemon** does the real ingest (validate → gossip into
  `/users/…`). Rung-1 records may be held node-local for the demo before full
  gossip propagation (a scope choice, §6).
- `GET /api/challenge` → a fresh nonce for the identity ceremony (so a
  submission proves key possession, not replay).

Explicitly **not** in v1: custodian/OIDC redirect (rung 3), the cross-origin
token flow for other `.mesh` services, gated-mode enforcement. Those are
post-demo (user-identity.md §6).

## 5. The DNS / `.mesh` seam (owned by the propagation track)

`hello.mesh` must resolve, on each node, to **that node's own LAN gateway IP**
so a browser reaches the local front desk; and RFC 8910 DHCP option 114 should
advertise `http://hello.mesh`. **This doc does not design that** — name
propagation and resolution are the separate `.mesh` track. The contract `hello`
needs from it:

1. `hello.mesh` (and the apex the front desk uses) resolves to the local node.
2. Ideally option 114 is set so the OS shows the non-blocking affordance.

Until that lands, `hello` is reachable by the node's LAN IP directly, which is
enough to build and test the server and frontend independently.

## 6. Demo scope vs. later

**DWeb demo (build target):**
- `mjolnir-hello` serving the SSG bundle at the node's LAN IP.
- `GET /api/directory` rendering the **neighbor list from real data available
  today** (address book + claims) — this works before `e21`.
- Anonymous browsing + one-tap **soft rung-1 identity** (noble key in
  IndexedDB), submitted to `/api/identity`; node-local directory presence is
  acceptable for the demo if gossiped `/users` propagation isn't wired yet.
- Publish-a-service (`wiki.mesh`) appearing in the directory rides on `e21`;
  if `e21` isn't ready, the demo degrades gracefully to the neighbor directory
  + identity, which is still a strong stage moment.

**After the demo:** services in the directory (`e21`), gossiped `/users`
records + the write-spool ingest, custodian (rung 3) and its redirect flow, the
cross-origin token flow, the browser extension (rung 1e), gated mode.

## 7. Open questions for the build

1. Directory freshness: daemon write cadence for `directory.json` (on-change vs
   interval) and whether `hello` stat-polls or the daemon signals.
2. Spool ingest: does the daemon watch `pending/` (inotify) or sweep on a
   timer; validation + de-dup rules; who removes the file.
3. `directory.json` schema versioning (a `v` field) so `hello` and the daemon
   can evolve independently.
4. Binary size budget on aarch64 with `rust-embed` + `tiny_http` — measure;
   fall back to serving assets from a staged dir if the embed bloats the image.
5. Whether `hello` should also expose the read-only directory to *mesh-native*
   clients or leave that to a native iroh path (probably the latter — `hello`
   is the legacy bridge, not the canonical API).
