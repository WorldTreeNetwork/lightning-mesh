---
id: node.join
title: Join the mesh
description: Give the new router its peers, start the daemon, and verify it is routing with the rest of the mesh.
path: node
order: 3
audience: [operator, agent]
status: built
time: 15 min
requires: [node.install]
next_step: node.operate
verified_against: 2f6dc6b (2026-09-24)
---

# Join the mesh

A new router joins by knowing at least one other router's **node id**, and
by being known in return. After that, everything else is automatic:
- a `10.254.x.y` overlay address derived from its identity
- a `/24` of client addresses claimed without a coordinator
- routes learned by babel

> **Status: built**, with one manual chore: peer lists are edited by hand on
> each node. Identity-peering / zero-touch join is designed, not shipped.

## How joining works

| Piece | What happens |
|---|---|
| Radio link | Routers with the same `MESH_ID`, band, channel and key form an 802.11s mesh (`br-mesh`) |
| Identity | Each router generates `/etc/mjolnir/secret` on first start. Its public key is the **node id** (64 hex chars) |
| Overlay address | `10.254.<first 16 bits of blake3(node id)>`. Collisions are detected and re-derived |
| Client subnet | Each router claims a `/24` in `10.42.0.0/16`. The earliest claim wins, and losers move to a free slot |
| Coordination | Routers gossip claims, names and services over iroh on one shared topic |
| Routing | babel routes everything between routers; any router with a working WAN offers internet to all |

> **Trust today:** there is **no membership gate yet.** The live backhaul is
> open, and any router running this software with matching radio settings
> can join the radio mesh and claim a subnet. Identity-gated peering is
> designed but not built (beads `dgi`, `661`, `met`). If that matters for
> your site, set `MESH_KEY` so only routers with the key can link.

## Step 1: Get the new router's node id

**Do:**
```sh
ssh root@192.168.1.1 'mjolnir-meshd id --secret-file /etc/mjolnir/secret'
```

**Expect:** a 64-character hex node id.

`mjolnir-meshd id` briefly starts a network endpoint. Run it on a router
whose daemon isn't running yet (true at this point), not on a busy live
node. For live nodes use `service mjolnir-meshd diag`, which is read-only.

## Step 2: Tell the new router about its peers

**Do:** on the new router, add one `list peer` line per other router, and
give it a name:
```sh
ssh root@192.168.1.1 "uci add_list mjolnir.meshd.peer='<node-id-of-router-A>'; \
  uci add_list mjolnir.meshd.peer='<node-id-of-router-B>'; \
  uci set mjolnir.meshd.name='kitchen'; uci commit mjolnir"
```

Maintainers of the live fleet keep ids in
[`deploy/openwrt/fleet-nodes.conf`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/deploy/openwrt/fleet-nodes.conf).
Add **every** other router, not just one. A chain of single peers once split
the fleet into two gossip islands.

**Expect:** `uci show mjolnir.meshd.peer` lists the ids.

## Step 3: Tell the existing routers about the new one

**Do:** on each existing router (reach them at their `10.254.x.y` address):
```sh
ssh root@10.254.x.y "uci add_list mjolnir.meshd.peer='<new-router-node-id>'; uci commit mjolnir; service mjolnir-meshd restart"
```

If you maintain an inventory, append a line to `fleet-nodes.conf` in the
form `name|10.254.x.y|node_id|model|notes`.

## Step 4: Start the daemon

**Do:**
```sh
ssh root@192.168.1.1 'service mjolnir-meshd start && logread -e "mjolnir-meshd starting"'
```

**Expect:** a startup line with `version=` and `build=`. The `build=` short
SHA must be the same on every router in the mesh.

**If not:** routers on different builds are the first suspect for any
strange behaviour. Update them all to one build.

## Step 5: Verify it joined

Run these on the new router (`ssh root@192.168.1.1 '…'`):

| Check | Command | Healthy result |
|---|---|---|
| Ground truth | `service mjolnir-meshd diag` | Node id, a `10.254.x.y` backhaul address on `br-mesh`, mesh routes |
| Radio peers | `iw dev` then `iw dev <mesh-if> station dump` | One entry per nearby router |
| Routes | `ip -4 route \| grep 'via 10.254'` | Routes to other routers' `10.42.x.0/24` subnets |
| Gossip | `logread \| grep -E 'received peer subnet claim\|addrbook: learned peer address'` | Lines naming other routers |
| Front desk | `wget -qO- http://<this-router-LAN-ip>/api/health` | A JSON health response |

hello.mesh listens on the router's **LAN gateway address** (`10.42.x.1`),
not on the `10.254` overlay. Check it from a device on that router's Wi-Fi
or LAN.

**Expect:** within about a minute the router has an overlay address, radio
peers, routes to the other subnets, and a working front desk.

**If not:**
- No radio peers: `MESH_ID`, `MESH_KEY`, band, channel or country don't match.
- Radio peers but no routes: check `backhaul_iface` is `br-mesh`, then
  `service mjolnir-meshd diag`.
- Routes but no gossip lines: peer ids are missing on one side (steps 2–3).

Next: [Operate your node](04-operate.md). Or, to use the mesh from a phone:
[Connect to the Wi-Fi](../person/01-connect.md).
