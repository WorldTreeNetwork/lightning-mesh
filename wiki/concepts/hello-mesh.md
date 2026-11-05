---
title: hello.mesh
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-components, hello]
sources: [crates/mjolnir-hello/]
state: [CURRENT]
confidence: high
---

# hello.mesh

hello.mesh is the visitor front desk -- a live directory of people, apps, services, and routers accessible from any node on the client network.

## Features

- **Directory** -- Gossip-converged list of nodes, services, users, and devices
- **IdentiKey** -- Browser-held cryptographic identity with recovery phrase and cross-origin sign-in (`/assert`)
- **Service discovery** -- `.mesh` names resolve from any node via the DNS responder
- **Mini-apps** -- Sandboxed iframes with TXT marker `app=v1`, loaded from node-side manifests

## Architecture

- `crates/mjolnir-hello/` -- The hello.mesh web application (Rust HTTP server + embedded assets)
- Serves on the LAN gateway (e.g., `http://10.42.1.1`)
- The visitor's rung-1 key lives in IndexedDB of hello origins
- `canEmbed` refuses every key-bearing origin structurally (IP-literal hosts, reserved names)

## Security Model

- Identity over the bridge uses a one-shot MessagePort pattern
- One pending request, 60s host deadline, cooldown keyed by service name
- Framed hello.mesh fails closed before identity load
