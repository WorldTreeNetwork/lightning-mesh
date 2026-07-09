# Publishing an app on the mesh (`keyed.mesh` walkthrough)

**Bead:** `mjolnir-mesh-5ll`

There are two ways a `.mesh` name reaches DNS: an **operator** SSHes into a
router and runs `meshd publish`, or an app's own client **self-serves** a
name it owns by key (no SSH, still landing). Pick the lane that fits your
deployment. Both end up resolving fleet-wide and listed in `hello.mesh`'s
Services panel.

## Lane 1 — operator publish (works today)

### How it works

`meshd publish`/`meshd unpublish` (`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs:144,168`)
are thin localhost HTTP clients: they `POST` to `127.0.0.1:5380` on the
router's own `mjolnir-meshd`, never touch CRDT state directly. The control
API only binds loopback — you must run the command **on the router**, over
SSH.

```
meshd publish  →  POST /v0/publish  (127.0.0.1:5380)
                       ↓
              ServiceBookV2 CRDT (crates/mjolnir-mesh/src/crdt/service.rs)
                       ↓
                    gossip (fleet-wide)
                       ↓
     ServiceTable DNS projection (dns_responder.rs:188) → <name>.mesh
                       ↓
        services[] in every node's directory.json (GET /api/directory)
```

`<name>.mesh` then resolves on **every** node in the mesh (any router
answers the query), and the entry shows up in `hello.mesh`'s Services panel
on every AP without further action — that panel just renders
`directory.json`'s `services[]`.

**FR29 — the IP is pinned, not chosen.** A plain `meshd publish <name> --port
N` always publishes with `ip` = the *publishing node's own* LAN gateway
address (its `10.42.<x>.1`), never an arbitrary host. So: **run `meshd
publish` on the router whose LAN your app host is attached to**, unless your
app runs on the router's own gateway IP. For a stationary device elsewhere
on that router's `/24` (a NAS, a Pi, a printer), use `--ip` instead — see
below.

### Copy-pasteable example: publish `keyed` on port 3000

SSH to the router over its overlay address (find it from your fleet
inventory or `meshd status`):

```bash
ssh root@10.254.<node-id-hash>   # overlay address, not the LAN gateway
```

If `keyed`'s app process is reachable at *this router's own* client gateway
(e.g. it's proxied through the router, or the router itself hosts it):

```bash
meshd publish keyed --port 3000
# published keyed.mesh  ip=10.42.7.1 port=3000
```

If `keyed` instead runs on a stationary box at a known IP on this router's
LAN (the common case — a NAS, a mini-PC, a Pi running the app), publish it
as a device instead so the entry carries that box's real IP, scoped under
this node so it can never collide with another node's device names:

```bash
meshd publish keyed --ip 10.42.7.42 --port 3000 --mac aa:bb:cc:dd:ee:ff
# published keyed.<node>.mesh  ip=10.42.7.42 port=3000
```

Note the printed name for a `--ip` publish: it is scoped to
`<name>.<node>.mesh`, not bare `<name>.mesh` — that's the actual name to
give out. Add TXT metadata with repeatable `--txt`:

```bash
meshd publish keyed --port 3000 --txt path=/app --txt proto=http
```

To release:

```bash
meshd unpublish keyed          # flat/node-hosted publish
meshd unpublish keyed --device # --ip device publish (re-derives the scoped key)
```

Reserved names (`hello`, `id`) are rejected. A name already owned by another
node's key comes back as a 409 with the winning owner — publish is
first-writer-wins with last-writer-wins refresh on the current owner, not a
free-for-all.

### meshctl

`meshctl` is SSH-only today (the control API is deliberately
`127.0.0.1`-bound, `mjolnir-meshd.rs:4456` — no operator wrapper endpoint
exists yet). Until one lands, `ssh <router> meshd publish ...` as above is
the supported operator path.

## Lane 2 — self-serve, key-owned leased names (client signing pending)

For an app whose *users* claim names for themselves — no SSH, no operator —
the mesh has a second lane: a name owned by an Ed25519 key instead of by a
node. The wire protocol is implemented and documented here so app authors
can build a client today; the piece that's pending is a reference client
that signs and auto-renews (bead `8tk`).

```
POST /api/name-claim (hello.mesh, any node's LAN gateway)
        ↓ (signature verified, spooled)
crates/mjolnir-hello/src/routes.rs:430
        ↓
spool_dir/names/<pubkey>.json
        ↓
meshd's name-claim sweep (bead 71x, ~5s cadence)
        ↓
LeasedNameTable DNS projection (dns_responder.rs:342) → <name>.mesh
```

A leased name resolves only while its owner keeps renewing (fade window
`LEASED_NAME_RESOLVE_STALE_MS` = 90s; the underlying ownership lease is 1h —
losing connectivity briefly doesn't lose the name, going dark for good does).
One name per key. A different key can only take over an **expired** lease,
never an actively-held one.

### Ceremony (implement this in your client)

1. `GET /api/challenge` on any node's `hello.mesh` (LAN gateway,
   `10.42.<x>.1:80`) → `{"challenge":"<hex nonce>"}`.
2. Sign the domain-separated preimage with your Ed25519 key
   (`crates/mjolnir-hello/src/routes.rs:189`):

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
     "ip": "10.42.7.42"
   }
   ```

   `ip` is optional and **not** covered by the signature (self-reported,
   node-vouched only) — omit it and meshd falls back to your request's
   source address. The challenge is single-use; get a fresh one per claim
   and per renewal.
4. **Renewal is on you**: re-run the ceremony (fresh challenge, fresh
   signature) well inside the 90s fade window — the shipped
   `walkie-talkie` client's republish loop (`src/lib/server/mesh.ts`) is the
   reference to extend with signing once `8tk` lands.

This endpoint only authenticates the request; it never holds your key.
Losing the key means the name fades and frees up after the lease lapses —
no permanent lock, no support ticket required.

## Consuming the directory (from your app or its landing page)

`GET /api/directory` on **any** node's LAN gateway (`10.42.<x>.1:80`)
returns the fleet-wide snapshot: `node`, `neighbors`, `identities[]`,
`services[]`. Every `GET /api/*` on `hello.mesh` sends
`Access-Control-Allow-Origin: *` (`crates/mjolnir-hello/src/routes.rs:561-566`),
so a page on one node's `hello.mesh` can cross-origin-poll another node's
directory to render mesh-wide topology.

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

## Identity

For the person-level identity ceremony that name-claims build on (nonce
issuance, signing, spooling — same shape, different preimage), and the
`/assert` handoff model, see
[`identity-assertion.md`](../network-coordination/identity-assertion.md)
and [`user-identity.md`](../network-coordination/user-identity.md).
