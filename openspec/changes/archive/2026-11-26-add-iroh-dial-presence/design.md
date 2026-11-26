# iroh dial presence

Records the contract shipped in commit `1a55cc7`. The code is the source; this note pins what it decided and where.

## Shape

```
client ── GET /api/challenge ─────────────▶ hello (LAN gateway)
client ── POST /api/iroh-dial ────────────▶ hello: check, then write <spool>/iroh-dial/<endpoint-id>.json
                                            meshd sweep (5 s): ingest → book → iroh-dial.json → gossip
other nodes ◀── IrohDialPublish ─────────── apply (re-verify) → book → iroh-dial.json
client ── GET /api/iroh/<endpoint-id> ────▶ hello reads iroh-dial.json, returns one record or 404
```

hello and meshd share one module, `crates/mjolnir-mesh/src/crdt/iroh_dial.rs`. hello never touches the book. It writes the spool and reads the projection. meshd owns the book.

## Decisions

### 1. A separate book keyed by the verifying key

`IrohDialBook` is a `BTreeMap<String, IrohDialEntry>` from endpoint id to the newest accepted claim. The endpoint id is the lowercase hex of the 32-byte Ed25519 key that signed the claim. Hex decoding accepts `0-9a-f` only, so an uppercase id is an encoding error.

It is not a `LeasedName` and not a `ServiceEntryV2`. The subject is a key, not a `.mesh` name, and the record is a 90-second hint with no owner, tombstone, or directory row. One id holds one socket: a newer claim replaces the row.

### 2. Signed preimage, with the IP inside it

`signing_message` builds exactly:

```
mjolnir-iroh-dial:v1\n<challenge_hex>\n<port>\n<ip>\n<signed_at_ms>
```

`ip` is dotted decimal. `port` and `signed_at_ms` are unpadded decimal. Verification is Ed25519 `verify_strict` under the endpoint id.

The IP is signed, and `submit_iroh_dial` accepts the POST only when the signed IP equals the TCP peer's IPv4 address. A differing address is `address does not match the client`. A peer that is not IPv4 is `no client address`. A client on the open SSID can therefore publish only the address it is speaking from.

The challenge rides in the record (`IrohDialEntry.challenge`) because every later checker has to rebuild the preimage. Only hello holds the challenge store, so only hello checks that the challenge was issued. Past hello the challenge is checked for form: lowercase hex, even length, 2 to 128 characters.

### 3. No relay URL, and routers are unchanged

`IrohDialEntry` has six fields: `endpoint_id`, `ip`, `port`, `signed_at_ms`, `sig`, `challenge`. `IrohDialRequest` carries the same six with the id named `pubkey`, and is `deny_unknown_fields`, so a body with `relay_url` or any other extra field is 400 `invalid request body` and nothing is spooled.

The commit does not change meshd's own iroh endpoint or its relay mode. The record is a dial hint on the existing gossip plane. iroh's handshake still authenticates the peer.

### 4. Address class, port, freshness, cap

| Rule | Value | Code |
| --- | --- | --- |
| Address | `10.42.0.0/16`, last octet not 0, 1, or 255 | `acceptable_dial_ipv4` |
| Port | 0 refused | `verify_signature` |
| Behind the checker | at most 90 000 ms | `RESOLVE_STALE_MS` |
| Ahead of the checker | at most 120 000 ms | `MAX_SKEW_MS` |
| Per source address | 8 other live endpoint ids | `MAX_PER_SOURCE` |

`10.254.0.0/16`, every other network, and IPv6 fall outside the address rule. The type is `Ipv4Addr`.

One predicate, `resolves`, is both the submission freshness check and the lookup liveness check. Each checker uses its own clock: hello at POST, meshd at ingest, every gossip receiver at apply, hello again at GET.

`verify_signature` checks port, then address, then encodings, then the signature. `validate_submission` adds the clock.

The cap counts live rows with the same `ip` and a different `endpoint_id`. It runs in two places with different inputs:

- hello reads `iroh-dial.json` and refuses with `too many dial records from this address`. A projection that is absent or unreadable does not refuse the claim.
- `apply` checks the book, and only when the id has no row yet. An id already in the book is never capped, including when its new address already has 8 other live hints. hello would refuse that move, because it counts the projection on every POST. This change keeps both checks as they shipped. It does not unify them.

### 5. Order of checks at hello

1. Parse the body. Unknown field or bad shape is 400.
2. Peer is IPv4 and equals the signed `ip`.
3. `validate_submission`: port, address, encodings, signature, clock.
4. Challenge is present and inside the 5-minute `CHALLENGE_TTL`. It is only looked at here.
5. Source cap from the projection.
6. Challenge is removed from the store.
7. Record is written to `<spool>/iroh-dial/<endpoint-id>.json` and returned with 200.

The challenge is consumed last, so a claim refused at any earlier step leaves the challenge usable.

### 6. Ingest and last-writer-wins

`iroh_dial_sweep_loop` runs on `NAME_CLAIM_SWEEP_INTERVAL` (5 s). Each tick it ingests the spool, reaps faded rows, rewrites the projection, and publishes each claim that changed the book. Every fourth tick it re-announces the whole live book for a peer that missed the first broadcast.

`ingest_spool` reads `*.json` only. A file whose stem differs from its `endpoint_id` is deleted without being applied. A file that does not parse is renamed to `.json.bad`. Every parsed file is deleted after `apply`, whatever the outcome.

`apply` validates first, then compares `signed_at_ms`. Greater replaces the row. Equal or smaller leaves it. A rejected claim never evicts the current row.

### 7. Gossip variant

`GossipMessage::IrohDialPublish { entry }` is appended after `NeighbourReportUpdate`. Its postcard discriminant is 15, pinned by `postcard_roundtrip_iroh_dial_publish`.

The receive arm in `mjolnir-meshd.rs` calls the same `apply`, so the signature, address, port, clock, and cap are checked again on every node. Only `Inserted` or `Updated` reaps and rewrites the projection. Any other outcome returns without a write.

A node that predates the variant fails to decode the payload and the gossip receive loop logs and skips it. Earlier variants keep decoding because nothing before 15 moved.

### 8. Lookup is one id, read from the projection

`write_projection` writes `iroh-dial.json` beside `directory.json` through a temp file and a rename. It holds `version: 1` and live records only.

`GET /api/iroh/<endpoint-id>` goes to `lookup_iroh_dial`. The id is 64 lowercase hex characters. The handler returns the record when the map key and the row's `endpoint_id` both equal the path id and the row still resolves. Everything else is 404, including an unreadable projection.

`GET /api/iroh` and `GET /api/iroh/` have their own route arm and are 404. There is no list.

GET does not re-verify the signature. Signatures are checked on the way into the book, and the projection is meshd's output on the same host. The record still carries `sig` and `challenge`, so a caller can verify it.

The response carries `Access-Control-Allow-Origin: *` like the other `GET /api/*` reads.

### 9. The book is not restored from disk

meshd constructs `IrohDialBook::new()` at startup. No file is loaded. A socket older than the resolve window is not a dial target, and a persisted book would grow from heartbeats that nobody deletes. The first sweep after a restart rewrites the projection from the empty book, so a stale `iroh-dial.json` lasts at most one tick, and lookup filters it by `resolves` in the meantime.

## What follows from these choices

- A client renews inside 90 seconds, and each renewal needs a new challenge.
- A node whose clock is outside the window drops the record at that node. If the off clock is the client's own gateway, hello returns 400 and nothing is spooled or gossiped, so no peer receives it either. The client sees that error.
- A receiver that missed a publish converges on the next re-announce, about 20 seconds.
- Past hello, replay is bounded by the freshness window and by last-writer-wins, not by the challenge store.
- The address-equals-peer check exists only at hello. Gossip receivers trust that the ingesting node made it and re-check everything the signature covers.

## Not in this change

Fleet rollout, a relay URL or house relay, a list endpoint, a persisted book, any client library, and the client-side address-lookup behaviour. The last one is tracked on `mjolnir-mesh-23ks.2` and nothing here depends on it.

## Driver edit

- Spec ingest now matches `ingest_spool`: stem mismatch deleted, malformed renamed to `.json.bad`, unreadable left in place, parsed file deleted after apply.
- Spec cap now states the book side: an id that already has a row is not capped, even when it moves onto an address hello would refuse.
- Left the spool-write race out of the contract. It is filed as follow-up, not a premise of this delta.
- After the 2026-11-26 send-back: the delta now scopes projection rewrite to gossip receipt, states the sweep rewrite and the fourth-tick re-announce, lists the full receipt check and what receipt cannot check, words hello's cap by address, and corrects the restart scenario and the off-clock bullet.

