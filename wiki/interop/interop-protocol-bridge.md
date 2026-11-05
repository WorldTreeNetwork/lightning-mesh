---
title: Full Mesh-to-Reticulum Protocol Bridge
created: 2025-11-05
updated: 2025-11-05
type: interop
tags: [interop, reticulum]
state: [FUTURE]
confidence: low
---

# Full Mesh-to-Reticulum Protocol Bridge

A gateway daemon that makes both networks transparent to applications. Highest effort, highest reward.

## Concept

An application talking to `foo.mesh` does not know if the peer is on the iroh transport or the Reticulum transport. The bridge handles all translation transparently.

## Architecture

```
Application
    |
    v
Mesh Application API (.mesh names, CRDT books)
    |
    v
+----------------------------------------------+
|              Bridge Daemon                    |
|                                               |
|  Unified Address Book:                        |
|  NodeId -> [iroh_addr, reticulum_dest]        |
|                                               |
|  Transparent Routing:                         |
|  mesh_packet -> reticulum_link -> peer        |
|  reticulum_packet -> iroh_datagram -> peer    |
|                                               |
|  MTU/Reliability Adaptation:                  |
|  iroh (best-effort datagram) <-> reticulum    |
|  (reliable channel with sequencing)           |
+----------------------------------------------+
    |                    |
    v                    v
Iroh Network        Reticulum Network
```

## Challenges

- **Different security models** -- mesh is open join, Reticulum requires explicit link establishment
- **Different addressing** -- mesh uses blake3-derived IDs, Reticulum uses hierarchical destination hashes
- **Different reliability** -- iroh datagrams are best-effort, Reticulum channels are reliable with sequencing
- **Different MTUs** -- iroh handles large MTUs, Reticulum is optimized for small packets (500-byte minimum)
- **Different latency profiles** -- QUIC vs multi-hop store-and-forward

## What It Would Enable

- A single application that works on both networks without changes
- Transparent mesh expansion: add Reticulum nodes without mesh changes
- Reticulum-only networks can access mesh services
- Mesh-only networks can access Reticulum services

## Why Not Phase 1

This is the highest-effort option. It requires building a full protocol translation layer, handling all edge cases (partition recovery, key rotation, address changes), and maintaining two protocol implementations. Start with the transport swap (Phase 1) and name translation (Phase 3) before attempting a full bridge.
