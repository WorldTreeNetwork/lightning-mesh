# Publish one EdgeBox service

> **PENDING**

## Why

A Raspberry Pi on an AP’s LAN already serves an application. Give its operator a way to advertise that application under a stable name and recover after a reboot. A visitor on the other AP should be able to find it in hello.mesh and open it. Today an operator can SSH into a router and publish on the device’s behalf. This change gives the service its own persistent app key and uses the existing router HTTP API.

## What belongs in #8

Ship one vertical slice: an opt-in publisher command, a single-service configuration, a boot recipe for the Pi, and evidence from a second AP. The client runs on an ordinary laptop for development. The only new capability is `edgebox-service-publishing`. The protocol, router software, observer stream and visitor UI already exist.

| Contribution | Disposition |
| --- | --- |
| Portable CLI, explicit role, persistent identity | Keep the interaction model. Add an explicit publisher subcommand to the merged CLI. Use a separate app signing key per service. |
| Periodic participation and restart persistence | Rework as renewal of the one configured lease. Persist the service configuration and key. Reconstruct requests after restart. |
| Mesh topic listener, address tickets and event decoding | Already landed in #12. Use it independently to observe the router’s `LeasedNamePublish` events. |
| Replicated books, disk snapshots, anti-entropy, node beacons, local DNS and control API | Leave with the prototype. The existing router owns these responsibilities. The old control API could list services but lacked a publish command, its address broadcast could be empty, and its spool did not restore service state. |
| Direct laptop/Pi gossip publishing | Defer until a use case requires operation without an ingesting router. That change would need an ownership, verification, refresh and conflict model for direct writes. |

[Original prototype](https://github.com/WorldTreeNetwork/lightning-mesh/tree/461826fc6b8052553596fc8cfc889fae62f3b276/crates/mjolnir-node) · [Merged observer](https://github.com/WorldTreeNetwork/lightning-mesh/pull/12) · [Existing publishing contract](../../../docs/deploy/mesh-app-publishing.md)

## User journey and surfaces

An operator starts the app on the EdgeBox, provisions a key once, configures the gateway and service, and enables the publisher. For example, the proposed command is `mjolnir-node publish --config /etc/lightning-mesh/publisher.toml`. It runs in the foreground under systemd on the Pi or directly in a laptop terminal. Its output is available to operators and agents.

A visitor on the m3000 opens hello.mesh. A plain web service appears under **Services**. An explicitly marked mini-app appears under **Apps** with its configured entry path. Opening it must reach the Pi attached to the AP3000. The publisher only advertises an address; ordinary mesh routing must already reach it. When the app stops listening, renewals stop and the listing fades. Restarting the app and publisher restores it under the same key.

## Proposed first configuration

```toml
gateway = "http://10.42.7.1"
key_file = "/var/lib/lightning-mesh/publisher/wiki.key"
name = "wiki"
ip = "10.42.7.42"
port = 8080
scheme = "http"
# Optional. Omit the table for a plain service.
[app]
path = "/wiki"
```

Addresses are examples. Use a stable, mesh-routable address assigned to the Pi for this first deployment. On startup validate the gateway URL, name, scheme, IP, port and optional app marker before issuing a claim. Use the existing name and marker validation rules. Provision the key with the proposed command `mjolnir-node publish init-key --config /etc/lightning-mesh/publisher.toml`. It creates the key exclusively and refuses to overwrite an existing file. Normal startup requires that key.

## Impact and boundaries

Implementation is confined to the publisher, its packaging and operator documentation. No change to the wire format, DNS, hello.mesh or guild membership is proposed. One configured name and key per instance fits the router’s one-live-name-per-key rule. Multi-service inventory, automatic DHCP roaming, Lightning Admin controls and a general-purpose gossip writer are separate product decisions. The storage-node roadmap’s signed management UI remains a later integration. Browser identity work in bead `mjolnir-mesh-8tk` remains independent.

Preparation: revised after #12 merged. The source of truth for ingress is `submit_name_claim` in [hello routes](../../../crates/mjolnir-hello/src/routes.rs), followed by `ingest_name_claim_spool` and `leased_directory_services` in [meshd](../../../crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs). The current code forwards an optional request IP unchanged and drops entries lacking it at ingest, despite the operator guide describing source-address fallback. This client supplies the IP on every renewal.
