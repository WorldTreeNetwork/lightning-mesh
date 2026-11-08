# Tasks

- [ ] Radio-map JSON schema + parser (model, phys id/band/path, qualified flag)
- [ ] Fixture map for `openwrt,one` (two phys, qualified false)
- [ ] Radioset decider: one function per phy; omitted mapped phy → disabled; satellite-split default
- [ ] setup-wireless / apply projector reads radioset; does not last-wins 2g/5g; does not FATAL a station the set assigned
- [ ] Split radio roles out of node-connection-profiles; profile may name radioset id
- [ ] Tests: two functions on one phy refused; extra phy disabled; default set is a no-op on today's satellite fixture; reordered UCI names; two same-band phys; map/inventory mismatch; unowned STA; isolate without acknowledgement
- [x] Advise accept on this architecture change (READER line) before act — reviews/2026-11-08-advise-r2.md
- [x] Fresh advise after preparation — accepted by astra-arch-review in reviews/2026-11-08-advise-r2.md; contract d4b510719083f5ae2ab812b16cd90868702bf13e
- [ ] Amend ARCHITECTURE.md on fold: radioset is the radio assignment store

## Owed from advise — 2026-11-08

- [x] R1: Restore the explicit stranded-peer acknowledgement in the wired-core delta/composed validator contract; add the no-acknowledgement refusal scenario. See reviews/2026-11-08-advise.md.
- [x] R2: Specify the canonical named-radioset store, model resolution, persisted 2g/5g variant, unknown-id refusal, profile-reference precedence, revision/rollback behavior, and default-id adoption; add restart/re-apply and exact-selection restoration scenarios.
- [x] R3: Define phy identity and model/inventory matching, pre-mutation mismatch/ambiguity refusals, unqualified-default policy, and unmapped-device ownership; add reordered-name, duplicate-band, and mismatched-map fixtures to acceptance coverage.
- [x] R4: Reconcile shell projection with the live-adapter gate and the 9d3o preservation handoff; pin owned versus unowned STA and stale/disabled iface behavior; add assigned-station re-apply, unowned-STA refusal, unsupported-live-change refusal, and composed rollback coverage. Keep live upstream commit deferred to 21rw.

Findings / handoffs (not boxes):

- OpenWrt One WAN 2026-11-07: eth0 1000 M, DHCP 10.42.149.167 via 10.42.149.1 (tr3000-b client LAN), not 192.168.0.0/24.
- Station live commit: mjolnir-mesh-21rw.
- Preserve station across generic apply: mjolnir-mesh-9d3o (this projector unblocks it).
- Third radio: mjolnir-dongle `148f:5370` ap2g only.
