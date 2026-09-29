## ADDED Requirements

### Requirement: NAT e2e is a gated command

`deploy/sim/e2e/run.sh nat` SHALL check that node-a WAN is on the
ISP LAN (`192.168.1.0/24`) with `gateway=auto`, node-b WAN is on
`192.168.50.0/24` without default export, overlay `10.254` traceroute
is one hop, and node-a cannot ping node-b's WAN address.

#### Scenario: Overlay not WAN

- GIVEN the household NAT fixture
- WHEN `deploy/sim/e2e/run.sh nat` runs
- THEN overlay traceroute a↔b is 1 hop
- AND ping from A to B's WAN address fails
