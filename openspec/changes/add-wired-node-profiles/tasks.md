# Tasks

`mjolnir-mesh-ai0.6` owns this architecture change.

- [ ] Independent architecture advise
- [ ] Profile decider: one role per port and radio, default `ethernet-gateway-satellites`, refuse unqualified `wired-core`
- [ ] Plan builder that emits a network-change transaction and restores the previous profile on failure
- [ ] Tests for loop refusals: client babel, node-link DHCP, WAN bridged into `br-lan`, cross-node client bridge, AP3000 `192.168.1.1/24` add, and silent deletion of a house-gateway default export

Not owed here:

- Live radio writes or fleet SSH.
- Per-model qualification (`mjolnir-mesh-lpv`).
- Resident Wi-Fi credentials (`mjolnir-mesh-ai0.7`).
- Station uplink tests (`mjolnir-mesh-bf7.2`).
