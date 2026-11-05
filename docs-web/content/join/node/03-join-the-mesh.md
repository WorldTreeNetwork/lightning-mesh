---
id: node.join
title: Join the mesh
description: Record the new router in the fleet inventory and verify it is routing with the rest of the mesh.
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

Install already started `mjolnir-meshd` and wrote `list peer` from
`fleet-nodes.conf`: every other member, not a list typed on the router.
A new router joins once that file has its id and the routers already in the
file have been installed again so they learn it. After that, the rest is
automatic:
- a `10.254.x.y` overlay address derived from its identity
- a `/24` of client addresses claimed without a coordinator
- routes learned by babel

> **Status: built.** Peer lists come from the fleet inventory at install.
> A box that is only powered on, with no inventory line, does not join.
> Identity-peering / zero-touch join is designed, not shipped.

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
ssh root@192.168.1.1 'mjolnir-meshd status --secret-file /etc/mjolnir/secret'
```

**Expect:** a line `node id:` followed by 64 hex characters.

`mjolnir-meshd status` reads the secret and does not open an endpoint.
`mjolnir-meshd id` binds the identity ULA on `br-mesh`. A missing house guild
does not make it exit. `service mjolnir-meshd diag` prints the same status.

## Step 2: Put this router in the inventory

**Do:** append a member line to
[`deploy/openwrt/fleet-nodes.conf`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/deploy/openwrt/fleet-nodes.conf):

```text
name|10.254.x.y|<node-id>|model|notes
```

The id is the one from step 1. The address is the `10.254.x.y` column
`mjolnir-meshd status` prints. Place the line in leaf-first order, above
the jump node.

**Expect:** the line has a name, a `10.254` address, and 64 hex characters.
A line with `-` in the address or the id is recorded hardware, not a peer.

## Step 3: Let the existing routers learn it

**Do:** from a checkout that has the new line:
```sh
deploy/openwrt/update-fleet.sh
```

That re-runs install on each reachable member, rewrites `list peer` to the
full inventory (every other member, not a chain of one), and restarts
`mjolnir-meshd`. This router's own install already peered whoever was in the
file at that time. The rollout is what teaches the existing routers the new
id. Rollout SSH uses the `10.254` column, so this router is included once
that address answers.

**Expect:** `uci show mjolnir.meshd.peer` on each router lists the other ids.
`logread -e "mjolnir-meshd starting"` shows `version=` and `build=`. The
`build=` short SHA must be the same on every router in the mesh.

**If not:** routers on different builds are the first suspect for any
strange behaviour. Update them all to one build. A chain of single peers
once split the fleet into two gossip partitions; the inventory write replaces
the whole list, so do not add peers one id at a time over SSH.

## Step 4: Verify it joined

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
- Routes but no gossip lines: this id is missing from `fleet-nodes.conf`, or `update-fleet.sh` has not run since it was added (steps 2–3).

Next: [Operate your node](04-operate.md). Or, to use the mesh from a phone:
[Connect to the Wi-Fi](../person/01-connect.md).
