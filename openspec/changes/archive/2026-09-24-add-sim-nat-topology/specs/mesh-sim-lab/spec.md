## ADDED Requirements

### Requirement: Double-NAT nodes still mesh on the air

node-a SHALL be able to take WAN on the ISP-CPE LAN with `gateway=auto`.
node-b SHALL be able to take WAN behind a second NAT without exporting
default. Overlay `10.254` reachability SHALL NOT require a WAN
hole-punch from B to A.

#### Scenario: Overlay without WAN path

- GIVEN 802.11s ESTAB a↔b
- AND B's WAN is double-NAT
- WHEN an operator pings A's `10.254` from B
- THEN it succeeds
- AND the path is not B-WAN → A-WAN
