# Tasks

`mjolnir-mesh-ai0.7` owns this architecture change.

- [ ] Fresh advise after preparation
- [x] Answered N1–N6 (2026-10-22). `psk2` plus `wpa_psk_file` lines `00:00:00:00:00:00 <passphrase>`; snapshot/restore and `secret_generation`; aggregate destination deny with guest egress kept; IPv6 read from committed config; guests re-DHCP and never touch `br-lan`; commit scenario is the planned `wireless`, not association; default guest plan has no hello.mesh exception.
- [x] Answered r2 R1–R3 (2026-10-22). Guest-sourced INPUT is rejected on every house node except DHCP and DNS on that node's guest interface. The upstream LAN past the node's own addresses is internet. The guest CIDR is a CRDT `/subnets/{cidr}` input. Babel return routes are a meshd follow-up. Guest profiles are refused while IPv6 is enabled.
- [x] Answered r3 R3-1 and R3-2 (2026-10-22). The guest-source class lives in each node's committed `mjolnir` config. A guest SSID on node N waits until every house node's committed config already rejects INPUT from N's guest CIDR. A passphrase-only revision names the iface whose `wpa_psk_file` hostapd re-reads on apply and on restore.
- [ ] Resident and optional guest plan builder. Passphrase stays out of the four UCI files, receipts, and hello.mesh. Uncommitted plans leave the open client SSID unchanged.
- [ ] Isolation gate: a plan is labeled isolated only when guest-to-resident and cross-node denies hold for IPv4 and for IPv6 if IPv6 is enabled, with no `br-lan` bridge. Otherwise the plan is refused and the previous revision stays.
- [ ] Tests for open-SSID non-mutation, a different SSID as a different profile, guest secret not equal to the resident secret, missing IPv6 deny when IPv6 is enabled, a named service exception, and refusal to call `OpenWrtAdapter`.

Not owed here:

- Live radio writes (`mjolnir-mesh-lpv`).
- Health probes (`mjolnir-mesh-2mgv`).
- Biscuit issuance (`mjolnir-mesh-ai0.1`).
- CAPPORT and the welcome page.
