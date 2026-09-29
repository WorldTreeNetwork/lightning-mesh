## ADDED Requirements

### Requirement: DHCP CRDT e2e is a gated command

`deploy/sim/e2e/run.sh dhcp-crdt` SHALL hop or place a STA on node-b
after node-a vended its IPv4, then DHCPREQUEST that address. The run
SHALL fail if node-b offers a different IPv4 or if `mjolnir-roam.conf`
on B does not contain the STA MAC→IP. The run SHALL NOT close
`mjolnir-mesh-wvg.4`.

#### Scenario: B ACKs A's lease

- GIVEN a STA whose IPv4 was vended on node-a
- WHEN it DHCPREQUESTs on node-b
- THEN the IPv4 is unchanged
- AND B's roam dhcp-host file lists that MAC and IP
