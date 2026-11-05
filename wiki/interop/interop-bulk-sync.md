---
title: Bulk CRDT Sync via Reticulum Resources
created: 2025-11-05
updated: 2025-11-05
type: interop
tags: [interop, reticulum, crdt]
state: [FUTURE]
confidence: medium
---

# Bulk CRDT Sync via Reticulum Resources

Use Reticulum's reliable `Resource` transfer for CRDT book reconciliation after network partitions.

## Concept

Gossip handles incremental CRDT updates in real-time. When a node reconnects after a partition, it needs the full state. Reticulum's `Resource` system provides reliable, sequenced, compressed file transfer exactly what is needed for bulk state sync.

## Flow

1. Node A reconnects after being offline
2. Node A requests a full CRDT snapshot from Node B
3. Node B serializes all CRDT books to a single snapshot file
4. Transfer via Reticulum `Resource` with sequencing and compression
5. Node A applies the snapshot, then resumes gossip for incremental updates

## Architecture

```
meshd                          Reticulum
  |                               |
  |-- CRDT Books --serialize-->   |-- Resource (snapshot.bin)
  |                               |-- Channel (reliable transfer)
  |                               +-- Link (encrypted)
  |                               |
  |-- apply_snapshot() <--deserialize--|
  |                               |
  +-- resume_gossip() <--|
```

## Snapshot Format

- Postcard-serialized collection of all CRDT book states
- Includes HLC timestamps for conflict resolution
- Signed by the source node for authenticity

## Benefits

- Fast full-state recovery after long partitions
- Reliable transfer with compression (important for low-bandwidth carriers)
- No custom sync protocol needed reuse Reticulum's battle-tested Resource system
- Works over any Reticulum carrier (LoRa, serial, WiFi, IP)

## Risks

- Snapshot size grows with mesh size (all books, all nodes)
- Requires both transports to be active simultaneously
- Reticulum's transfer is synchronous may block the gossip loop

## Implementation

- Add `serialize_snapshot()` and `apply_snapshot()` to the CRDT module
- A Reticulum `Resource` handler that serves/applies snapshots
- Trigger bulk sync when a node's HLC is more than N anti-entropy ticks behind
