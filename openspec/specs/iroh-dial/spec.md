# iroh-dial

A client iroh endpoint publishes the UDP socket where it can be dialed on the household LAN. The hint is keyed by the endpoint id. It is not a `.mesh` name, not a service-directory row, and not a hello.mesh card. Folded from `add-iroh-dial-presence` (2026-11-26).

Code: `crates/mjolnir-mesh/src/crdt/iroh_dial.rs`, `crates/mjolnir-hello/src/routes.rs` (`POST /api/iroh-dial`, `GET /api/iroh/<endpoint-id>`), `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs` (spool sweep and gossip apply).

## Requirements

### Requirement: Dial hint keyed by endpoint id

The system SHALL keep dial hints in a book keyed by endpoint id. The endpoint id SHALL be the lowercase hex encoding of the 32-byte Ed25519 public key that signs the hint. The system SHALL NOT store the hint as a leased name or as a `ServiceEntryV2`.

#### Scenario: One socket per endpoint id

- GIVEN the book holds a hint for endpoint id A at `10.42.2.10:4000`
- WHEN a newer valid hint for A arrives at `10.42.2.10:4001`
- THEN lookup for A returns port 4001 and the book still has one row for A

### Requirement: Signed preimage

The signed bytes SHALL be exactly `mjolnir-iroh-dial:v1\n<challenge_hex>\n<port>\n<ip>\n<signed_at_ms>`, with `ip` in dotted decimal and `port` and `signed_at_ms` in unpadded decimal. The signature SHALL verify under the endpoint id with Ed25519 `verify_strict`. A record SHALL carry no relay URL. Unknown JSON fields on `POST /api/iroh-dial` SHALL be rejected with 400 and SHALL NOT be spooled.

#### Scenario: Relay field rejected

- GIVEN a body that is otherwise a valid dial claim and also contains `relay_url`
- WHEN the client POSTs it to `/api/iroh-dial`
- THEN the response is 400 and the spool has no file for that endpoint id

### Requirement: Client-LAN address and port

The signed address SHALL be an IPv4 address in `10.42.0.0/16` whose last octet is not 0, 1, or 255. The system SHALL reject `10.254.0.0/16`, any other network, IPv6, and port 0. hello SHALL accept the POST only when the signed address equals the TCP peer's IPv4 address. A differing body address SHALL be 400 with error `address does not match the client` and SHALL NOT be spooled. A peer that is not IPv4 SHALL be 400 with error `no client address`.

#### Scenario: Spoofed address

- GIVEN the TCP peer is `10.42.2.10` and the signed `ip` is `10.42.2.11`
- WHEN the client POSTs the claim
- THEN the response is 400, the error is `address does not match the client`, and nothing is spooled

#### Scenario: Gateway and overlay refused

- GIVEN a claim whose signed address is `10.42.2.1` or `10.254.1.1`, or whose port is 0
- WHEN it is validated
- THEN it is rejected and does not enter the book

### Requirement: Challenge, freshness, and source cap at hello

hello SHALL require a challenge from `GET /api/challenge` that is still inside the 5-minute challenge TTL. The challenge string SHALL be lowercase hex, even length, from 2 to 128 characters. hello SHALL consume the challenge only after the signature, address, freshness, and source cap succeed. hello SHALL apply the same freshness window as the book: `signed_at_ms` at most 120000 ms ahead of hello's clock and at most 90000 ms behind it. hello SHALL reject a claim when the projection already holds 8 other live hints for that signed address, including a known endpoint id moving onto that address from somewhere else. hello SHALL NOT count the claim's own endpoint id. A renewal is accepted while that address has at most 8 live hints. When the address holds 9 or more, which the book's rule allows, hello SHALL refuse every claim for that address until enough hints fade that fewer than 8 other live hints remain. An unreadable projection SHALL NOT by itself reject the claim. The book's cap SHALL run only when that endpoint id has no row yet, so a newer hint for an id already in the book SHALL replace the row even when the new address already has 8 other live hints.

#### Scenario: Ninth other endpoint

- GIVEN eight live hints from `10.42.2.10` for eight other endpoint ids
- WHEN a ninth endpoint id validly claims `10.42.2.10`
- THEN hello responds 400 with error `too many dial records from this address` and does not consume the challenge

#### Scenario: Known id moves onto a full address in the book

- GIVEN the book holds endpoint id A at `10.42.2.10` and eight other live hints at `10.42.2.20`
- WHEN a newer valid hint for A at `10.42.2.20` is applied
- THEN the book stores A at `10.42.2.20`

#### Scenario: Known id is refused when it moves onto a full address at hello

- GIVEN the projection holds endpoint id A at `10.42.2.10` and eight other live hints at `10.42.2.20`
- WHEN A POSTs a newer valid hint signed for `10.42.2.20` from that address
- THEN hello responds 400 with error `too many dial records from this address`, does not spool the claim, and does not consume the challenge

#### Scenario: Renewal refused when the address holds nine

- GIVEN the projection holds nine live hints at `10.42.2.20`, one of them endpoint id A
- WHEN A POSTs a newer valid hint for `10.42.2.20`
- THEN hello responds 400 with error `too many dial records from this address` and does not consume the challenge

#### Scenario: Renewal at the cap

- GIVEN eight live hints from `10.42.2.10`, one of them for endpoint id A
- WHEN A submits a newer valid hint from `10.42.2.10`
- THEN the claim is spooled

### Requirement: Spool and single-record lookup

An accepted POST SHALL write `iroh-dial/<endpoint-id>.json` under the hello spool and SHALL return 200 with that record's JSON (`endpoint_id`, `ip`, `port`, `signed_at_ms`, `sig`, `challenge`). meshd SHALL ignore names that are not `.json`. A file whose stem differs from `endpoint_id` SHALL be deleted and not applied. A file that does not parse SHALL be renamed to a `.json.bad` sibling and not applied. A file that cannot be read SHALL be left in place. A parsed file whose stem matches SHALL be deleted after apply, whether or not the book changed. `GET /api/iroh/<endpoint-id>` SHALL return that one live record from the projection beside `directory.json`, or 404. The id SHALL be 64 lowercase hex characters. `GET /api/iroh` and `GET /api/iroh/` SHALL return 404. GET `/api/iroh/*` SHALL send `Access-Control-Allow-Origin: *`, the same as other `GET /api/*` reads. The GET handler does not re-verify the signature. The 200 body includes `sig` and `challenge`, so the caller can.

#### Scenario: Lookup of one id

- GIVEN the projection holds a live hint for endpoint id A and none for B
- WHEN a client GETs `/api/iroh/A` and `/api/iroh/B`
- THEN A is 200 with A's record and `Access-Control-Allow-Origin: *`, and B is 404

#### Scenario: No list

- WHEN a client GETs `/api/iroh` or `/api/iroh/`
- THEN the response is 404

### Requirement: Memory book, fade, and gossip

meshd SHALL keep the book only in memory and SHALL NOT restore it from disk at startup. A record SHALL remain resolvable while `signed_at_ms` is no more than 90000 ms in the past and no more than 120000 ms in the future, measured by the checker that is looking, and SHALL leave the book when it falls outside that window. Last-writer-wins SHALL use `signed_at_ms`: a greater timestamp replaces the row, an equal timestamp keeps the row already stored, and a smaller timestamp or a failed validation SHALL NOT remove the current row. Gossip SHALL carry the hint as an appended `IrohDialPublish` variant whose postcard discriminant is 15, after `NeighbourReportUpdate`. On receipt, apply SHALL check port, address class, encodings, the signature, the receiver's own clock, last-writer-wins, and the book cap when the endpoint id has no row yet. Receipt SHALL NOT treat the signed address as this node's TCP peer and SHALL NOT require that this node issued the challenge. Only an insert or an update from that receipt SHALL rewrite the projection. The spool sweep SHALL rewrite the projection from the live book on every tick, including a tick that changed nothing. Every fourth tick a node SHALL re-announce its live book, including rows it learned from other nodes. A node that does not know the variant SHALL log-and-skip it and SHALL keep decoding earlier variants. The projection SHALL contain version 1 and only records that resolve at the time it is written.

#### Scenario: Stale hint does not replace a live one

- GIVEN the book holds endpoint id A signed at time T
- WHEN a hint for A arrives signed at an earlier time, or with a bad signature
- THEN A's stored socket is unchanged

#### Scenario: Restart does not revive a socket

- GIVEN meshd had a live hint and then the process starts again
- WHEN the book is constructed
- THEN the in-memory book is empty until a valid spool file is applied or a peer's re-announced hint is accepted, and the next sweep rewrites the projection from the book
