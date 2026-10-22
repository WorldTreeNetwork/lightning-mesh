# add-wired-node-profiles

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

A cable can be WAN, node backhaul, or a client port. Treating every Ethernet
link as one LAN bridges client segments and creates loops. The house needs
named profiles that refuse those mixes before `mjolnir-apply` commits them.

## What

- Add node-connection profiles that give each used port and radio one role.
- Default to the current satellite split until a per-model map is qualified.
- Make wired-core require distinct WAN, node-link, and client ports, and make
  cable-loss behavior part of the profile instead of a silent radio grab.
- Apply a profile only as a network-change transaction.

## Impact

- Capabilities: ADDED node-connection-profiles.
- ADRs: none.
- Work node: mjolnir-mesh-ai0.6. Depends on the folded
  network-change-transactions spec. Activation inherited from Duke's
  `activate ai0`. No live radio writes. Hardware qualification stays
  `mjolnir-mesh-lpv`.
- Does not change AP3000's house-gateway default-route export and does not
  put `192.168.1.1/24` on AP3000 `br-lan`.

## User journey & surfaces

An owner, from the future Manage network surface, applies the default
satellite profile and still has the 2.4 GHz client network and the 5 GHz
backhaul. Choosing wired-core on a model with no qualified port map is
refused with the missing map named. After a qualified wired-core apply,
unplugging the node-link cable isolates that node when the profile released
the mesh radio, or keeps the reserved mesh radio when the profile said so.
No profile joins client bridges across nodes.

## Out of scope

- Private resident Wi-Fi (`mjolnir-mesh-ai0.7`).
- Wi-Fi station uplink (`mjolnir-mesh-bf7.2`), except that a profile may name
  the station role as assigned elsewhere.
- Live fleet applies and per-model port qualification (`mjolnir-mesh-lpv`).
- Paid transit.
