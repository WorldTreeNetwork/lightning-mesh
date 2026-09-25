## ADDED Requirements

### Requirement: install-node runs on sim guests

`install-node.sh` SHALL be able to stage and apply meshd/babeld on a
sim guest over mgmt SSH. The sim wireless profile SHALL require the
sim-guest marker before any UCI/`wireless` write.

#### Scenario: Apply on a marked guest

- GIVEN a sim guest with `/etc/mjolnir/sim-guest`
- WHEN install-node runs against its mgmt address
- THEN meshd and babeld are present
- AND wireless devices include band=2g and band=5g hwsim radios
