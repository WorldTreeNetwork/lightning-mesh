# node-connection-profiles

## ADDED Requirements

### Requirement: A single Ethernet port is WAN

A node whose port map has exactly one Ethernet port SHALL assign that port
the WAN role under the default profile. Its client bridge SHALL have no
wired port. A single-port node found with its port on the client bridge
SHALL be converted in one network-change transaction.

#### Scenario: Outdoor AP cabled to a house router

- GIVEN a `cudy,ap3000outdoor-v1` whose `eth0` is cabled into a house LAN
- WHEN the default profile is applied
- THEN `eth0` takes a DHCP lease from the house router
- AND no house device receives a `10.42` lease from the node
- AND a `Lightning Mesh` client on its AP is leased from the node's own `/24`

#### Scenario: Conversion fails its health gate

- GIVEN the conversion is applied
- WHEN the node cannot reach a mesh peer or its WAN gateway afterwards
- THEN the previous configuration is restored

#### Scenario: Recovery on the WAN port

- GIVEN a converted node
- WHEN the operator presses WPS
- THEN SSH from the house LAN reaches the node on its WAN address

### Requirement: A foreign upstream on a client port is isolated

A node that observes a DHCP server or default router that is not a mesh
node behind one of its wired client ports SHALL detach that port, and only
that port, from the client bridge. Wi-Fi clients, the mesh backhaul, WAN and
the other wired ports SHALL keep working. The node SHALL show the isolated
port and the foreign server's address on hello.mesh and in its status, and
SHALL log it once. The port SHALL rejoin the bridge without operator action
when its link drops, or when the foreign server has not been seen for 60
seconds. An owner SHALL be able to mark a port as never isolated.

#### Scenario: Client port cabled into a house LAN

- GIVEN a multi-port node with `lan2` cabled into a house router
- WHEN the node sees that router answering DHCP behind `lan2`
- THEN `lan2` leaves the client bridge
- AND a laptop on `lan1` and a phone on the node's Wi-Fi keep working
- AND hello.mesh names `lan2` and the router's address

#### Scenario: The cable is pulled

- GIVEN `lan2` is isolated
- WHEN its link drops
- THEN `lan2` rejoins the client bridge at once

#### Scenario: A device that happens to run DHCP

- GIVEN an owner plugged in a device that serves DHCP on purpose
- WHEN the owner marks that port as never isolated
- THEN the port stays on the client bridge

#### Scenario: Nothing else stops

- GIVEN a port is isolated
- WHEN a mesh peer routes traffic through this node
- THEN that traffic is unaffected
