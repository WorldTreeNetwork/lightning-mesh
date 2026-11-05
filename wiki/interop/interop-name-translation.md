---
title: Name Translation Bridge
created: 2025-11-05
updated: 2025-11-05
type: interop
tags: [interop, reticulum, name-translation]
state: [FUTURE]
confidence: medium
---

# Name Translation Bridge

Bidirectional name resolution between `.mesh` labels and Reticulum destination hashes.

## Concept

A service published as `myapp.mesh` on Lightning Mesh is also discoverable as `<name>.rns` on Reticulum, and vice versa.

## Architecture

```
meshd                          bridge daemon                    Reticulum
  |                               |                                |
  |-- ServiceBook --------------> |                                |
  |         |                     |-- RNS.Resolver ---------------|
  |         v                     |                                |
  |-- DNS Responder ------------- |                                |
  |         |                     |                                |
  |         v                     |                                |
  |   .mesh name --> RNS name ----|                                |
  |         ^                     |                                |
  |         |                     |                                |
  +-- Gossip  ---------------------+--------------------------------+
```

## Translation Table

| .mesh Label | Reticulum Name | Direction |
|---|---|---|
| `myapp.mesh` | `owner.myapp.mesh` | mesh to reticulum |
| `owner.myapp.mesh` | `myapp.mesh` | reticulum to mesh |

## Implementation

- A small daemon (`mesh-rns-bridge`) runs on nodes with both transports
- Listens to the mesh `ServiceBook` and Reticulum `ResourceAdvertisement` layer
- Maps between `.mesh` labels and Reticulum destination hashes
- Uses Reticulum's `Resolver` for name-to-hash lookup
- Uses the mesh's `.mesh` DNS responder for the reverse direction

## Benefits

- Apps on one network discover services on the other transparently
- No application changes needed the bridge is transparent
- Supports gradual migration: a mesh-only app can discover reticulum services

## Risks

- Name collisions if the same label exists on both networks with different meanings
- Different TTL policies (mesh uses HLC, Reticulum uses announce intervals)
- The bridge daemon adds a dependency on Python on OpenWrt
