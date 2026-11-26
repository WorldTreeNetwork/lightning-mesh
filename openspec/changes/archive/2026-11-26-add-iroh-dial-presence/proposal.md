# add-iroh-dial-presence

> **ACTIVE BUILD**

**Rigor:** architecture. Campaign authority on `mjolnir-mesh-23ks` from "activate this set" (2026-11-26). This node was dependency-ready. `mjolnir-mesh-23ks.3` stays blocked. This pause does not act or fold.

## Why

A client iroh endpoint on the household LAN can already be reached by babel once its `10.42` UDP socket is known. Commit `1a55cc7` publishes that socket as a signed dial hint. No living spec states the contract, so a later edit can change the preimage, the address rules, or the gossip variant without a spec to fold against.

## What

Record the shipped dial book as capability `iroh-dial`.

- The record is keyed by the endpoint id, the Ed25519 verifying key, not by a `.mesh` name and not by a `ServiceEntryV2`.
- hello.mesh accepts `POST /api/iroh-dial` when the signed address is the TCP peer, and answers `GET /api/iroh/<endpoint-id>` with one live record.
- meshd keeps an in-memory book, re-checks the signature on ingest and on gossip, and projects live records beside `directory.json`.

## Impact

- Capabilities: ADDED `iroh-dial`
- ADRs: none. `ARCHITECTURE.md` does not change shape; the dial book is a hint on the existing gossip plane.

## User journey & surfaces

No new UI because the outcome is HTTP on the LAN gateway the client already uses: `GET /api/challenge`, `POST /api/iroh-dial`, and `GET /api/iroh/<64-hex-endpoint-id>`. There is no directory page, no hello.mesh card, and no list.

Working: a signed `10.42` host socket with a fresh challenge is spooled, then lookup returns that record. Empty: an unknown id, a faded record, and the bare paths `/api/iroh` and `/api/iroh/` are 404. Failed: a spoofed address, an unknown JSON field, a bad signature, port 0, a clock outside the window, or a ninth other endpoint on the same source address is 400 and is not spooled.

## Out of scope

- Fleet rollout. Tracked on `mjolnir-mesh-23ks.3`, blocked by this node.
- Whether a client iroh stack calls address lookup after a missed direct socket. Tracked on `mjolnir-mesh-23ks.2`. This change does not use that mechanism as a premise.
- Router `RelayMode`. LAN meshd stays as it is.
- A house relay, tunnelling app QUIC through the router endpoint, and a relay URL on this record.
- Listing the book over HTTP.
- The name-claim source-IP fallback. Lane 2 names stay a different book.
- Bead `8tk` and any client library.

## Preparation

- Upstream: commit `1a55cc7`. Anchors: `crates/mjolnir-mesh/src/crdt/iroh_dial.rs`, `submit_iroh_dial` and `lookup_iroh_dial` in `crates/mjolnir-hello/src/routes.rs`, `iroh_dial_sweep_loop` and the `IrohDialPublish` arm in `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`.
- Living spec: none for this capability. In-flight changes do not own the dial book.
- Outcome: still valid. The delta records the shipped contract.
- Authority: the campaign note on `mjolnir-mesh-23ks`. Bounds are that note's goals, non-goals, and architectural commitments.
