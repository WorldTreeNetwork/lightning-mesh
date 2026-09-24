---
id: publish.service
title: Publish a service on the mesh
description: Give an app or device a .mesh name that resolves on every router and appears in hello.mesh.
path: publish
order: 1
audience: [operator, developer, agent]
status: partial
time: 10 min
requires: [node.join]
next_step: publish.miniapps
verified_against: 2f6dc6b (2026-09-24)
---

# Publish a service on the mesh

Publishing gives your app a `<name>.mesh` address that resolves from every
router, and lists it in the Services panel on every hello.mesh. There are
two ways in.

| Lane | Who | Status |
|---|---|---|
| **Operator publish** | Someone with SSH to a router | Built |
| **Key-owned name** | The app itself, signing with its own key | Built on the router side. No reference client library yet (bead `8tk`), and signatures aren't re-checked mesh-wide yet |

## Lane 1: Operator publish

The publish command talks to the router's local control API, which only
listens on `127.0.0.1`. So you run it **on the router**, over SSH.

The on-router command is **`mjolnir-meshd`**. Older docs say `meshd publish`,
and no `meshd` alias exists.

### Step 1: Pick the right router

**Do:** SSH to the router whose LAN your app's host is plugged into:
`ssh root@10.254.x.y`.

A plain publish always uses **that router's own gateway address**
(`10.42.x.1`). A service on another box on that LAN needs `--ip`
(step 2b).

### Step 2a: Publish a service the router itself serves

**Do:**
```sh
mjolnir-meshd publish keyed --port 3000
```

**Expect:** `published keyed.mesh  ip=10.42.x.1 port=3000`.

### Step 2b: Publish a device on the router's LAN

**Do:**
```sh
mjolnir-meshd publish keyed --ip 10.42.7.42 --port 3000 --mac aa:bb:cc:dd:ee:ff
```

**Expect:** a **router-scoped** name,
`published keyed.<4-char>.mesh  ip=10.42.7.42 port=3000`. Share that full
name. The router-specific part stops two routers' devices from colliding.

### Step 3: Add details (optional)

**Do:** add repeatable `--txt` key/value pairs, for example
`--txt path=/app --txt proto=http`.

For a web app, include `--txt proto=http` (or `proto=https`). That's what
makes hello.mesh show it as a clickable link; without it the entry shows as a
bare address. Add `--txt app=v1` (and optionally `--txt path=/entry`) to
put it on the **Apps** shelf — [Mini-apps](02-mini-apps.md).

### Step 4: Check it from anywhere on the mesh

**Do:** from any device on any router's Wi-Fi, open `http://hello.mesh` and
look under **Services**. Or run `nslookup keyed.mesh`.

**Expect:** the name resolves to the published address, and a web service
(`http`/`https`) shows as a clickable link.

**If not:**
- `409` conflict: another router already owns that name. The first
  publisher wins, and only the owner can refresh it. Pick another name.
- Reserved names are rejected: `hello`, `id`.
- Not listed yet: the directory refreshes about every 5 seconds. Gossip can
  take a few more.

### Unpublish

```sh
mjolnir-meshd unpublish keyed            # plain publish
mjolnir-meshd unpublish keyed --device   # a --ip device publish
```

## Lane 2: Key-owned names (for app developers)

An app can hold a name with its **own Ed25519 key**, with no operator and no
SSH. The name is a lease: it keeps resolving while the app renews it, and
frees up after the app goes quiet.

| Rule | Value |
|---|---|
| Names per key | One |
| Stops resolving after | 90 s without renewal |
| Ownership lease | 1 hour. Another key can only take an **expired** lease |
| Signed message | `mjolnir-name-claim:v1\n<challenge>\n<name>\n<port>` |

### Step 1: Get a challenge

**Do:** `GET http://<any-router-LAN-gateway>/api/challenge`

**Expect:** `{"challenge":"<hex>"}`, single-use and valid for 5 minutes.

### Step 2: Sign

**Do:** sign the exact bytes
`"mjolnir-name-claim:v1\n" + challenge + "\n" + name + "\n" + port` with your
Ed25519 key. `name` must already be one lowercase DNS label that isn't
reserved. The router checks the bytes you signed, so don't let it
normalize anything. Use port `0` for a plain address record.

### Step 3: Claim

**Do:**
```http
POST /api/name-claim
Content-Type: application/json

{"pubkey":"<64 hex>","sig":"<128 hex>","challenge":"<hex>","name":"keyed","port":3000,"ip":"10.42.7.42","scheme":"https"}
```
`ip` and `scheme` are optional and aren't covered by the signature. Without
`ip`, the router uses your request's source address. Set `scheme` to `http`
or `https` so hello.mesh lists your name as a clickable link; without it the
name shows as a bare address.

**Expect:** accepted. Within about 5 seconds the router publishes the name,
and it resolves mesh-wide.

### Step 4: Renew

**Do:** repeat steps 1–3 with a fresh challenge well inside 90 seconds.

A working example of this lane is the `walkie-talkie` app's republish loop
(`src/lib/server/mesh.ts` in that app).

**Current limits:** only the router that ingests a claim checks its
signature. Mesh-wide re-verification isn't built yet. Treat key-owned names
as convenient, not as proof of identity.

## Read the directory from your app

`GET http://<any-router-LAN-gateway>/api/directory` returns the mesh-wide
snapshot: `node`, `neighbors`, `identities[]`, `services[]`. Every `GET /api/*`
sends `Access-Control-Allow-Origin: *`. There's no push yet, so poll about
every 5 seconds.

Full protocol reference:
[`docs/deploy/mesh-app-publishing.md`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/docs/deploy/mesh-app-publishing.md).

Next: [Mini-apps](02-mini-apps.md).
