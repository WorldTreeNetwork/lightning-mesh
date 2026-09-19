---
id: house.how-it-works
title: How the house mesh works
description: Client Wi-Fi, router backhaul, addresses, internet sharing, and the service directory — in household language.
path: house
order: 3
audience: [person, operator, agent]
status: built
time: 8 min
requires: [house.first-hour]
next_step: house.administer
verified_against: 723b308 (2026-09-19)
---

# How the house mesh works

Lightning Mesh is several layers that share a box. Mixing them up is the
usual source of confusion.

> **Status: built.** This is how a live house fleet behaves today.

## Two Wi-Fi names

| Name you see | Who joins it | Purpose |
|---|---|---|
| `⚡` or `Lightning Mesh` (or your rename) | Phones, laptops, IoT | **Client** access point. This is the house Wi-Fi. |
| `mjolnir-mesh` | Routers only | **Backhaul.** 802.11s. Phones cannot usefully join it. |

Association to the client name is not membership of a guild and does not
write identity logs. The backhaul is how routers talk to each other when
they are not on the same ethernet.

## Addresses you will see

| Kind | Looks like | Meaning |
|---|---|---|
| Your device | `10.42.61.23` | DHCP from the router you associated to. Each router owns its own `/24`. |
| That router's LAN gateway | `10.42.61.1` | hello.mesh and LAN services on **this** box |
| Router management (overlay) | `10.254.x.y` | Derived from the router's node id. SSH here once the mesh is up |
| Recovery ethernet | `192.168.1.1` | Only when you plug a computer into a LAN port and the overlay is dead |

Two people in the same room can have different `10.42.` prefixes if they
landed on different routers. That is expected. babel routes between them.

Do not add `192.168.1.1/24` onto a live gateway's `br-lan` as a second
address while it is already serving `10.42.x.1`. That combination has
produced a "lease but no ping" black hole for phones.

## What talks to what

```
phone  --client Wi-Fi-->  router A (10.42.A.1)
                              |
                         802.11s backhaul  (mjolnir-mesh)
                              |
                         router B (10.42.B.1)  --WAN-->  existing house internet
                              |
                         overlay 10.254/16  (SSH, gossip, .mesh names)
```

- **Radio** is plumbing. The product is the routed overlay plus the
  directory of people and services.
- Client LAN segments are **not** bridged across routers. Broadcast stays
  on one box. That is why printers and Chromecast discovery that depend on
  LAN broadcast may need a published `.mesh` name instead.
- If any one router has a working WAN and `gateway=auto`, the whole mesh
  can use that internet. If the uplink dies, sharing stops by itself.

## The service mesh (directory)

Every router runs a small front desk (`mjolnir-hello`) at
`http://hello.mesh`. Behind it, `mjolnir-meshd` gossips a CRDT: node
names, who is around, and published services.

| Piece | What you see |
|---|---|
| **People** | Optional IdentiKeys that introduced themselves |
| **Services** | `.mesh` names (a chat app, a file drop, a printer) |
| **Routers** | Boxes in this mesh and which ones hear each other |

A service is not "installed on the cloud." Someone published it from a
router (or an app claimed a name). It resolves on every node. If the
publisher goes away, the listing ages out (about 90 seconds after it
stops renewing).

Apps at `https://something.mesh` use certificates the mesh made. Browsers
warn because `.mesh` is not a public internet name. Continue only for
names you opened from hello.mesh. Details:
[Find services](../person/05-services.md) and
[Publish a service](../publish/01-publish-a-service.md).

## What is not built yet

| Planned | Today |
|---|---|
| House owner / admin roles in the UI | SSH keys and whoever can reach the box |
| Private resident Wi-Fi by default | Open client SSID unless you set a password at flash time |
| Membership gate for adding routers | Matching radio settings is enough to join the backhaul |
| Seamless roam while on a call | You may get a new `10.42.` address when you walk |

Treat those as design, not as knobs on the box you just plugged in.

Next: [Administer the house](04-administer.md).
