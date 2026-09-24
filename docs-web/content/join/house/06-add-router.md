---
id: house.add-router
title: Add another router
description: Expand coverage with a second pre-flashed box, or flash your own hardware onto the same mesh.
path: house
order: 6
audience: [operator, agent]
status: partial
time: 30 min–2 h
requires: [house.services]
next_step: node.hardware
verified_against: 2f6dc6b (2026-09-24)
---

# Add another router

One box is a house Wi-Fi with a local front desk. A second box is a mesh:
walk between rooms, share one WAN, see both under Routers.

> **Status: partial.** Radios and routing work. **Peering is still
> manual.** Powering a second pre-flashed box does **not** join it to the
> first. You paste 64-character node ids over SSH on both boxes.
> Automatic discovery (without treating association as trust) is in
> progress, not on the box.

## Path A — another pre-flashed box (same image)

Both boxes must share the **same** client SSID, backhaul mesh id, channel
plan, and mesh key (including "empty / open"). Factory images already
match (`CLIENT_SSID` `⚡` or `Lightning Mesh`, `MESH_ID` `mjolnir-mesh`,
open backhaul).

### Step 1: Power the new box in radio range

**Do:** place it where it can hear the first router on 5 GHz, power it,
do not give it a second WAN unless you want a second uplink.

**Expect:** after a couple of minutes, `mjolnir-mesh` is on the air from
both. Client Wi-Fi with the same name appears from both. Phones can land
on either radio. They are **not** yet exchanging routes or directory.

**If not:** they were flashed with different `wireless.env`. Re-flash or
re-apply wireless from [Install](../node/02-install.md).

### Step 2: Exchange node ids (required)

There is no membership gate and no zero-touch join yet. Each daemon must
list the other as a peer.

**Do:** on each box (recovery ethernet `root@192.168.1.1` or overlay SSH):

```sh
mjolnir-meshd id --secret-file /etc/mjolnir/secret
```

(`id` briefly starts a network endpoint. On a **live** node prefer
`service mjolnir-meshd diag` and read the id from there.)

Then on box A, add B's id; on box B, add A's id:

```sh
uci add_list mjolnir.meshd.peer='<64-hex-id>'
uci commit mjolnir
service mjolnir-meshd restart
```

Add **every** other router, not just one. A chain of single peers has
split a fleet into two gossip islands.

Exact verify steps (station dump, babel routes, gossip, `/api/health`):
[Join the mesh](../node/03-join-the-mesh.md).

**Expect:** hello.mesh → **Routers** shows both, with a link between them.

**If not:** they are on different channels or mesh ids. `service
mjolnir-meshd diag` on each box.

### Step 3: Only one house gateway unless you mean it

Leave `gateway=auto` on the box that has the real WAN. Set
`gateway=never` on extras that happen to have ethernet to an untrusted
network. See [Administer the house](04-administer.md).

## Path B — flash your own hardware

Use this when the box is still stock Cudy (or other supported mt76
OpenWrt), not when it already boots Lightning Mesh.

1. [Choose a router and flash OpenWrt](../node/01-hardware-and-flash.md)
2. [Install Lightning Mesh](../node/02-install.md) — same `wireless.env`
   as the house
3. [Join the mesh](../node/03-join-the-mesh.md)
4. [Operate](../node/04-operate.md)

Supported fleet hardware today: Cudy WR3000S, M3000, TR3000, AP3000
Outdoor (all MT7981). Wrong Cudy intermediate image bricks the unit —
match the **label on the router**, not the box art.

A single-radio board cannot run 802.11s and a client AP on the same radio.
The installer turns that co-located AP off. The supported USB client AP
is Ralink RT5370 (`148f:5370`).

## After it joins

Phones keep using one client SSID. They land on whichever radio is
stronger. Household DHCP tries to keep one IP per MAC; a live call can
still drop. See [How it works — walking](03-how-it-works.md#walking-between-rooms).

When you are done adding boxes, residents can stay on
[Connect to the Wi-Fi](../person/01-connect.md). Operators come back to
[Administer the house](04-administer.md).
