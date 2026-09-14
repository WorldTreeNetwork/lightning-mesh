# Publishing an app on the mesh

**Bead:** `mjolnir-mesh-5ll`

> **Names in this doc.** `keyed` is a worked **example** name used throughout;
> nothing called `keyed.mesh` is deployed. The app actually running on the
> live mesh is `walkie-talkie.mesh`, which publishes itself through Lane 2
> (key-owned name).
>
> **Command name.** The on-router binary is **`mjolnir-meshd`**. Older design
> docs write `meshd publish`; there is no `meshd` alias on a node.

There are two ways a `.mesh` name reaches DNS: an **operator** SSHes into a
router and runs `mjolnir-meshd publish`, or an app **self-serves** a name it
owns by key (no SSH). Both are built. Pick the lane that fits your
deployment. Both end up resolving fleet-wide and listed in `hello.mesh`'s
Services panel.

User-facing, step-by-step version of this page:
[`docs/join/publish/01-publish-a-service.md`](../join/publish/01-publish-a-service.md).

## Lane 1 — operator publish (works today)

### How it works

`mjolnir-meshd publish` / `mjolnir-meshd unpublish` (the `Command::Publish` /
`Command::Unpublish` subcommands in `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`)
are thin localhost HTTP clients: they `POST` to `127.0.0.1:5380` on the
router's own running `mjolnir-meshd`, and never touch CRDT state directly.
The control API only binds loopback — you must run the command **on the
router**, over SSH.

```
mjolnir-meshd publish  →  POST /v0/publish  (127.0.0.1:5380)
                               ↓
                      ServiceBookV2 CRDT (crates/mjolnir-mesh/src/crdt/service.rs)
                               ↓
                            gossip (fleet-wide)
                               ↓
          ServiceTable DNS projection (dns_responder.rs) → <name>.mesh
                               ↓
            services[] in every node's directory.json (GET /api/directory)
```

`<name>.mesh` then resolves on **every** node in the mesh (any router
answers the query), and the entry shows up in `hello.mesh`'s Services panel
on every AP without further action — that panel just renders
`directory.json`'s `services[]`.

**FR29 — the IP is pinned, not chosen.** A plain `mjolnir-meshd publish <name>
--port N` always publishes with `ip` = the *publishing node's own* LAN gateway
address (its `10.42.<x>.1`), never an arbitrary host. So: **run the publish
on the router whose LAN your app host is attached to**, unless your app runs
on the router's own gateway IP. For a stationary device elsewhere on that
router's `/24` (a NAS, a Pi, a printer), use `--ip` instead — see below.

### Copy-pasteable example: publish `keyed` on port 3000

SSH to the router over its overlay address (find it from your fleet
inventory or `service mjolnir-meshd diag`):

```bash
ssh root@10.254.<node-id-hash>   # overlay address, not the LAN gateway
```

If `keyed`'s app process is reachable at *this router's own* client gateway
(e.g. it's proxied through the router, or the router itself hosts it):

```bash
mjolnir-meshd publish keyed --port 3000
# published keyed.mesh  ip=10.42.7.1 port=3000
```

If `keyed` instead runs on a stationary box at a known IP on this router's
LAN (the common case — a NAS, a mini-PC, a Pi running the app), publish it
as a device instead so the entry carries that box's real IP, scoped under
this node so it can never collide with another node's device names:

```bash
mjolnir-meshd publish keyed --ip 10.42.7.42 --port 3000 --mac aa:bb:cc:dd:ee:ff
# published keyed.<node>.mesh  ip=10.42.7.42 port=3000
```

Note the printed name for a `--ip` publish: it is scoped to
`<name>.<node>.mesh`, not bare `<name>.mesh` — that's the actual name to
give out. Add TXT metadata with repeatable `--txt`:

```bash
mjolnir-meshd publish keyed --port 3000 --txt path=/app --txt proto=http
```

`--txt proto=http` (or `https`) is what makes hello.mesh render the entry as
a clickable link: the CLI stores the record's protocol as `_tcp`, and the
directory projection (`directory_protocol` in `mjolnir-meshd.rs`) promotes a
`proto=http|https` TXT value to the listed protocol. Without it the entry
lists as a bare address.

To release:

```bash
mjolnir-meshd unpublish keyed          # flat/node-hosted publish
mjolnir-meshd unpublish keyed --device # --ip device publish (re-derives the scoped key)
```

Reserved names (`hello`, `id`) are rejected. A name already owned by another
node's key comes back as a 409 with the winning owner — publish is
first-writer-wins with last-writer-wins refresh on the current owner, not a
free-for-all.

### meshctl

`meshctl` is SSH-only today (the control API is deliberately
`127.0.0.1`-bound — `CONTROL_API_PORT` in `mjolnir-meshd.rs` — and no operator
wrapper endpoint exists yet). Until one lands,
`ssh <router> mjolnir-meshd publish ...` as above is the supported operator
path.

## Lane 2 — self-serve, key-owned leased names (router side built)

For an app that holds its own name — no SSH, no operator — the mesh has a
second lane: a name owned by an Ed25519 key instead of by a node. The router
side is built and in use: `walkie-talkie.mesh` publishes this way, signing
claims server-side in its republish loop. What's still pending is a reusable
reference client library, including a browser-held key (bead `8tk`).

```
POST /api/name-claim (hello.mesh, any node's LAN gateway)
        ↓ (signature verified at ingest, spooled)
crates/mjolnir-hello/src/routes.rs  submit_name_claim
        ↓
spool_dir/names/<pubkey>.json
        ↓
meshd's name-claim sweep (bead 71x, ~5s cadence)
        ↓
LeasedNameTable DNS projection (dns_responder.rs) → <name>.mesh
```

A leased name resolves only while its owner keeps renewing (fade window
`LEASED_NAME_RESOLVE_STALE_MS` = 90s; the underlying ownership lease is 1h —
losing connectivity briefly doesn't lose the name, going dark for good does).
One name per key. A different key can only take over an **expired** lease,
never an actively-held one.

**Trust today:** only the router that ingests a claim verifies its signature.
The signature is carried on the gossiped record so every node *can*
re-verify it, but re-verification on gossip apply isn't built yet
(`crates/mjolnir-mesh/src/crdt/leased_name.rs`, "Trust"). Treat key-owned
names as convenient, not as proof of identity.

### Ceremony (implement this in your client)

1. `GET /api/challenge` on any node's `hello.mesh` (LAN gateway,
   `10.42.<x>.1:80`) → `{"challenge":"<hex nonce>"}` (single-use, valid 5 min).
2. Sign the domain-separated preimage with your Ed25519 key
   (`name_claim_signing_message` in `crates/mjolnir-hello/src/routes.rs`):

   ```
   "mjolnir-name-claim:v1\n<challenge_hex>\n<name>\n<port>"
   ```

   `<name>` must be **pre-normalized**: a single lowercase DNS label
   (`mjolnir_mesh::normalize_device_host`), not reserved (`hello`, `id`).
   The server verifies byte-for-byte against what you signed — normalizing
   server-side would invalidate the signature. `<port>` is `0` if you're
   publishing an A-only record.
3. `POST /api/name-claim`:

   ```json
   {
     "pubkey": "<64-hex Ed25519 pubkey>",
     "sig": "<128-hex Ed25519 signature>",
     "challenge": "<hex nonce from step 1>",
     "name": "keyed",
     "port": 3000,
     "ip": "10.42.7.42",
     "scheme": "https"
   }
   ```

   `ip` and `scheme` are optional and **not** covered by the signature
   (self-reported, node-vouched only). Omit `ip` and meshd falls back to your
   request's source address. Set `scheme` to `http` or `https` so hello.mesh
   lists the name as a clickable link; without it the name lists as a bare
   address. The challenge is single-use; get a fresh one per claim and per
   renewal.
4. **Renewal is on you**: re-run the ceremony (fresh challenge, fresh
   signature) well inside the 90s fade window. The `walkie-talkie` app's
   republish loop (`src/lib/server/mesh.ts` in that app) is a working
   server-side example.

This endpoint only authenticates the request; it never holds your key.
Losing the key means the name fades and frees up after the lease lapses —
no permanent lock, no support ticket required.

## Consuming the directory (from your app or its landing page)

`GET /api/directory` on **any** node's LAN gateway (`10.42.<x>.1:80`)
returns the fleet-wide snapshot: `node`, `neighbors`, `identities[]`,
`services[]`. Every `GET /api/*` on `hello.mesh` sends
`Access-Control-Allow-Origin: *` (`crates/mjolnir-hello/src/routes.rs`, the
`cors` flag on GET routes), so a page on one node's `hello.mesh` can
cross-origin-poll another node's directory to render mesh-wide topology.

- `identities[]` entries carry `last_seen_unix` (ms epoch of the identity's
  last CRDT write) — use it for a recency indicator, not a strict presence
  signal yet (push/liveness wiring is tracked in bead `9vb`/`bux`).
- `services[]` entries carry `name`, `ip`, `port`, `protocol`, and optional
  `txt`/`host_mac` — everything you published in Lane 1 or 2 comes back here.
- No push transport exists yet (bead `9vb`) — poll on a ~5s cadence, matching
  `hello.mesh`'s own frontend.

## Walk-up discovery

Once published (either lane), `keyed.mesh` resolves from any mesh AP — a
phone that joins any node's wifi and asks for `keyed.mesh` gets an answer,
same as `hello.mesh` itself. It also appears automatically in
`hello.mesh`'s Services panel (which renders `directory.json`'s
`services[]`) on every node, with no separate registration step — publish
is the only action required for both DNS resolution and UI discoverability.

## Mini-apps (coming soon)

> **Status: coming soon — not usable on the mesh yet.**
> - The shared contract module (`hello-mesh-web/src/lib/miniapp/contract.ts`)
>   and the router-side manifest fetcher behind `GET /api/apps`
>   (`crates/mjolnir-hello/src/apps.rs`) are merged (commit `503cd17`).
> - The Apps shelf UI that would render them is **not built and not
>   deployed** (`add-app-shelf`, bead `ncy.3`), and no deployed build has been
>   checked to include `/api/apps`. Today every service, app-marked or not,
>   shows as a plain Services entry.
>
> Design: [`openspec/changes/add-mini-app-contract`](../../openspec/changes/add-mini-app-contract/)
> (accepted). User-facing summary:
> [`docs/join/publish/02-mini-apps.md`](../join/publish/02-mini-apps.md).
> Epic: `mjolnir-mesh-ncy`.

A mini-app is a published service that hello.mesh can show as an app card.
The contract:

- **Marker.** Add `app=v1` to the service's TXT record, and optionally
  `path=/entry` for the entry path. Works with today's publish, no schema
  change: `mjolnir-meshd publish keyed --port 3000 --txt proto=http --txt app=v1 --txt path=/app`.
  (A `--app` convenience flag is planned, `ncy.2`.)
- **Manifest.** The app serves `/.well-known/mesh-app.json`:

  ```json
  {"v":1,"name":"Keyed","description":"Tasks for the house","icon":"/icon.png","embed":"card","height":480}
  ```

  **Routers fetch it, not browsers.** Each node's `mjolnir-hello` refreshes
  manifests and icons in the background (only the record's own `ip` inside
  `10.42.0.0/16` or `10.254.0.0/16`, no redirects, 3 s timeout, 128 KiB cap,
  5 min refresh, last-good kept 1 h) and serves the results from memory at
  `GET /api/apps`. A visitor's browser never contacts an app host before the
  visitor opens the app, and apps need no CORS. A missing or invalid manifest
  degrades to a link tile. Manifest values are rendered as text or image
  sources, never as HTML.
- **Insertion.** `embed: card` becomes a tap-to-load, sandboxed iframe on the
  app's own origin, with an open-in-new-tab control. Everything else is a
  link. Cards are allowed **only for apps on a non-reserved `.mesh` name
  host**: IP-literal hosts (including every node's gateway address), the
  reserved names `hello`/`id`, and the current page origin always link out,
  because app script must never run in an origin that can hold a visitor's
  hello.mesh key.
- **Bridge.** A versioned `postMessage` envelope `{"mesh":"mini-app/v1",...}`
  with origin-pinned send and receive. v1 types: app→host `ready`,
  `resize {height}` (clamped 120–640 px), `open {url}`; host→app `init`.
  Identity over the bridge is a separate accepted design, not built:
  [identity-assertion.md, "Planned: embedded transport"](../network-coordination/identity-assertion.md#8-planned-embedded-transport-not-built).

What you can do today: publish with `--txt app=v1` and serve a valid
manifest. Nothing displays it yet, but both are already valid under the
contract. To sign visitors in now, use the full-page `/assert` redirect.

## Identity

For the person-level identity ceremony that name-claims build on (nonce
issuance, signing, spooling — same shape, different preimage), and the
`/assert` handoff model, see
[`identity-assertion.md`](../network-coordination/identity-assertion.md)
and [`user-identity.md`](../network-coordination/user-identity.md).
