# Tasks

`mjolnir-mesh-ai0.6` owns this architecture change.

- [x] Independent architecture advise r1 send-back (`reviews/opus-arch-review.md`)
- [x] Answer B1–B5 in design.md and the spec delta (2026-10-22)
- [ ] Fresh advise after preparation
- [ ] Profile decider: one role per port and radio, node-link not in `br-mesh`, omitted mapped ports disabled, default `ethernet-gateway-satellites`, refuse unqualified `wired-core`
- [ ] Plan builder on a fake adapter: `mjolnir.node_link`, required health, previous revision on failure. Do not submit to `OpenWrtAdapter` until live probes exist
- [ ] Babel renderer reads `mjolnir.node_link` as wired interfaces
- [ ] Tests for loop refusals: client babel, node-link in `br-mesh`, node-link DHCP, WAN bridged into `br-lan`, unassigned mapped port, cross-node client bridge, AP3000 `192.168.1.1/24` add, silent deletion of a house-gateway default export, and default-profile no-op on a current fixture

Not owed here:

- Live radio writes or fleet SSH.
- Per-model qualification (`mjolnir-mesh-lpv`).
- Resident Wi-Fi credentials (`mjolnir-mesh-ai0.7`).
- Station uplink tests (`mjolnir-mesh-bf7.2`).
