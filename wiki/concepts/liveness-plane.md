---
title: Liveness Plane
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-core, crdt]
sources: [crates/mjolnir-mesh/src/crdt/liveness.rs]
state: [CURRENT]
confidence: high
---

# Liveness Plane

An ephemeral per-node liveness tracking system that avoids flash writes by decoupling staleness from durable CRDT state.

## Design

- Each node emits a `LivenessBeacon` once per anti-entropy tick
- Beacons are never merged into a book, never persisted, never relayed
- Every other node keeps an in-memory `LivenessTracker` of when it last heard a newer beacon
- A record is stale iff its owner's beacon has not advanced within `stale_threshold_ms`

## Beacon Format

- `incarnation` -- boot wall-clock time in ms. A reboot yields a later incarnation.
- `counter` -- per-boot monotonic sequence (+1 each beacon)
- `egress` -- optional internet-egress advertisement (gateway status)

## Staleness Judgment

The beacon carries no timestamp that orders events. Freshness is `monotonic_now_ms()` minus the receiver-local time the beacon was accepted. This makes the plane immune to clock skew.

## Gateway Tracking

`live_gateways(now_ms)` returns nodes that are not stale and advertised a healthy egress. A gateway that stops beaconing drops out; one that withdraws uplink drops out on the next beacon.

## Key Files

- `crates/mjolnir-mesh/src/crdt/liveness.rs` -- implementation
- `docs/network-coordination/lane-staleness.md` -- design rationale
