## ADDED Requirements

### Requirement: Named libvirt domains boot on mgmt

The sim lab SHALL define named libvirt networks and q35 domains so an
operator can start guests and SSH on the management net (`10.99.0.0/24`)
with BatchMode, without joining the client SSID. The management prefix
SHALL NOT overlap Docker or other host bridges.

#### Scenario: Cold start two nodes

- GIVEN the sim x86 image exists
- WHEN the documented start command runs
- THEN node-a and node-b obtain mgmt leases
- AND `ssh -o BatchMode=yes root@<mgmt>` succeeds
