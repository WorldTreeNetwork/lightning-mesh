---
name: Layer separation vision
description: mjolnir-mesh = mesh networking lib+binary for router hardware; mjolnir-node = example client app (audio MVP)
type: project
---

mjolnir-mesh is the networking layer — a lib crate and binary meant to run on specialized router hardware (GL.iNet routers). It handles mesh routing, repeater functionality, and will have caching/store-and-forward for offline nodes. Generic byte streams, no application knowledge.

mjolnir-node is the example client application — for MVP it's the audio chat app that uses mjolnir-mesh as a library.

**Why:** The mesh layer must be application-agnostic to serve as infrastructure on embedded routers. Audio is just the first use case.

**How to apply:** Keep all audio/codec/cpal code out of the mesh crate. The mesh exposes broadcast producers and track consumers of raw bytes. The node binary wires those to audio pipelines.

Capability-based access control is in the project's DNA but not being designed yet. DHT, distributed DHCP/DNS work happening in a separate session.
