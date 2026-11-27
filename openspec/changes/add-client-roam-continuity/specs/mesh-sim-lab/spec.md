# mesh-sim-lab

## MODIFIED Requirements

### Requirement: DHCP CRDT e2e is a gated command

`deploy/sim/e2e/run.sh dhcp-crdt` SHALL hop or place a STA on node-b
after node-a vended its IPv4, then DHCPREQUEST that address. The run
SHALL fail if node-b offers a different IPv4 or if `.mjolnir-roam.hosts`
in node-b's dnsmasq `conf-dir` does not contain the STA MAC→IP. The run
SHALL NOT close `mjolnir-mesh-wvg.4`.

#### Scenario: B ACKs A's lease

- GIVEN a STA whose IPv4 was vended on node-a
- WHEN it DHCPREQUESTs on node-b
- THEN the IPv4 is unchanged
- AND B's roam hosts file lists that MAC and IP
