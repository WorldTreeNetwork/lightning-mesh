# Add single-port WAN uplink

> **PENDING**

**Rigor:** architecture. Drafted 2026-11-13 from a field finding
(`mjolnir-mesh-9nl6`).

## Why

A house router keeps its own network. Mesh nodes cabled into it are its
clients, the way wr3000s-bench already is: DHCP on `wan`, NAT out, WAN input
refused, WPS press opens SSH from the house LAN (`wan-lan-admin`).

ap3000-outdoor-b does not follow that. OpenWrt's board default for
`cudy,ap3000outdoor-v1` puts its only port, `eth0`, on `br-lan`, and nothing
in install or the default profile moves it. The default profile reads port
roles from the config it finds, so it reports "no-op" on a node whose only
port is a client port. Cabled into the house LAN, that node:

- merged its client `/24` with the house LAN (23 house MACs on `eth0`),
- leased `10.42.181.x` to house devices,
- let the house router answer `Lightning Mesh` clients on its AP.

The ceiling AP3000 has `eth0` as WAN only because its board default differs.

## What

1. **One port means WAN.** A node whose port map has exactly one Ethernet
   port uses it as WAN (DHCP client, `wan` zone, masquerade). Its client LAN
   is Wi-Fi only (`br-lan` with no wired port, `bridge_empty`).
2. **Install and apply enforce it.** A single-port node found with that port
   on the client bridge is converted in one network-change transaction with
   the usual health gate and rollback.
3. **Foreign upstream on a client bridge is detected.** A node that sees a
   DHCP server or router that is not a mesh node on its client bridge detaches
   the wired ports from that bridge and says so. Today
   `heal::foreign_gateway_on_bridge` only knows mesh gateways.
4. **Every wired node is a gateway.** Each node with a live WAN exports the
   default route (`gateway=auto`, already built). Nodes without one use the
   nearest.

## The trade this makes

Two gateways on one house LAN each NAT behind their own house address. A
phone that roams between two gateway nodes changes its egress, so the house
router sees a new flow. A Signal call re-establishes its media path; a TCP
connection resets. Keeping the phone's address does not prevent that.

Not solved here. Options, to decide after the walk test measures it:
- accept it (calls recover, long TCP flows do not);
- pin each client's egress to its home node while a /32 is live, at the cost
  of a mesh hop;
- ask the house router for a route to `10.42.0.0/16` and drop NAT, which
  touches a network this design says to leave alone.

## Impact

- Capability: MODIFIED `node-connection-profiles`.
- Code: `crates/mjolnir-apply/src/node_profile.rs` (single-port rule),
  `crates/mjolnir-mesh/src/heal.rs` (foreign upstream), install path.
- Fleet: ap3000-outdoor-b converts; wr3000s-bench is already correct.

## User journey & surfaces

No new UI because the surface is the cable: plug a single-port node into a
house router and it becomes an uplink, not a second DHCP server. Recovery on
that port is the existing WPS press (`wan-lan-admin`).
