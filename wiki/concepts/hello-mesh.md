---
title: hello.mesh
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-components, hello]
sources: [crates/mjolnir-hello/, hello-mesh-web/src/lib/miniapp/contract.ts, hello-mesh-web/src/lib/miniapp/identity-bridge.ts]
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
- The public name is `http://hello.mesh` on that node's LAN gateway (`10.42.<x>.1`)
- The visitor's rung-1 key lives in IndexedDB of hello origins
- `canEmbed` accepts an embed only when the mode is `card`, the host ends in `.mesh`, and the origin is not key-bearing. Same-origin, IP-literal hosts, and reserved names are refused

## Security Model

- The identity bridge accepts exactly one `MessagePort`
- One pending request, 60s host deadline, cooldown keyed by service name
- Framed hello.mesh fails closed before identity load
