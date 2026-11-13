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

### Requirement: A foreign upstream on the client bridge is isolated

A node that observes a DHCP server or default router that is not a mesh
node on its client bridge SHALL detach the wired ports from that bridge and
SHALL report it.

#### Scenario: Client port cabled into a house LAN

- GIVEN a multi-port node with a client port cabled into a house router
- WHEN the node sees that router answering DHCP on its client bridge
- THEN the wired ports leave the client bridge
- AND the node's status names the port and the foreign server
