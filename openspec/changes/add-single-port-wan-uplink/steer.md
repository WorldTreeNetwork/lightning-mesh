# steer add-single-port-wan-uplink

**When.** 2026-11-26
**Depth.** standard

## Decided

- Single Ethernet port: one port is WAN (user)
  Why: a node with one port, cabled into a house LAN, was merging that LAN with its client bridge. Outdoor-b is already converted this way.
- Foreign upstream on a client port: detach that port only (user)
  Why: the other ports, Wi-Fi, and the mesh stay up. The port rejoins when the link drops or the foreign server has been quiet for 60 seconds. An owner can exempt a port.
- Two gateways on one house LAN: measure first (user)
  Why: a roam changes egress. The walk records what a call and a TCP flow do before any policy is chosen. Accept-resets, pin-egress, and drop-NAT stay later options.

## Skipped

- (none)

## Feeds change

The draft direction stands. A node with exactly one Ethernet port uses it as WAN, and its client LAN is Wi-Fi only. A foreign DHCP server or router behind one client port detaches that port alone. Do not choose an egress policy for two NAT gateways until the roam walk has been recorded. The banner stays PENDING until the change is activated.
