# Tasks

`mjolnir-mesh-ai0.7` owns this architecture change.

- [ ] Fresh advise after preparation
- [x] Answered N1–N6 (2026-10-22). `psk2` plus `wpa_psk_file` lines `00:00:00:00:00:00 <passphrase>`; snapshot/restore and `secret_generation`; aggregate destination deny with guest egress kept; IPv6 read from committed config; guests re-DHCP and never touch `br-lan`; commit scenario is the planned `wireless`, not association; default guest plan has no hello.mesh exception.
- [x] Answered r2 R1–R3 (2026-10-22). Guest-sourced INPUT is rejected on every house node except DHCP and DNS on that node's guest interface. The upstream LAN past the node's own addresses is internet. The guest CIDR is a CRDT `/subnets/{cidr}` input. Babel return routes are a meshd follow-up. Guest profiles are refused while IPv6 is enabled.
- [x] Answered r3 R3-1 and R3-2 (2026-10-22). The guest-source class lives in each node's committed `mjolnir` config. A guest SSID on node N waits until every house node's committed config already rejects INPUT from N's guest CIDR. A passphrase-only revision names the iface whose `wpa_psk_file` hostapd re-reads on apply and on restore.
- [x] Aligned the open task lines and the IPv6 scenarios with that refusal (2026-10-22). A guest profile is refused when IPv6 is enabled or unknown. The cross-node deny scenario is IPv4. Fresh advise stays open.
- [x] Answered r4 R4-1 through R4-4 (2026-10-22). Any revision that drops a guest CIDR while a guest SSID for it is committed is refused. The class is each node's committed `mjolnir` config, not a fleet plan object. `guest_routes` excludes that class. IPv6 is enabled on `ula_prefix`, `ip6assign`, `wan6`/`dhcpv6`, or odhcpd `ra`/`dhcpv6`, and a guest iface sets no `ip6assign`. A joiner (`mjolnir-mesh-fyby`) carries the class before it exports routes.
- [ ] Answer r5 R5-1 (`reviews/2026-10-22-advise-r5.md`): guest IPv6 link-local. The guest device carries `ipv6 '0'`, or the guest node's INPUT reject is keyed on the guest interface for both families. Pin it with a test.
- [ ] Answer r5 R5-2: one sentence in design.md that names what orders the cross-node steps between N's build and N's commit (re-read peer revisions at submit, or one house apply session at a time), plus the residual window.
- [ ] Answer r5 R5-3: the deny aggregate is the committed mesh client space plus `10.254.0.0/16`. A guest CIDR outside the client space is refused, with a test.
- [ ] Resident and optional guest plan builder. Passphrase stays out of the four UCI files, receipts, and hello.mesh. Uncommitted plans leave the open client SSID unchanged. The passphrase type is uplink `Passphrase`.
- [ ] Roam exclusion: `guest_routes` does not install a proto-158 `/32` for an address in the committed guest-source class. Test a `10.42` address in that class.
- [ ] Isolation gate: a plan is labeled isolated only when guest-to-resident and cross-node denies hold for IPv4, with no `br-lan` bridge. A guest profile is refused when IPv6 is enabled or unknown. Otherwise the plan is refused and the previous revision stays.
- [ ] Tests for open-SSID non-mutation, a different SSID as a different profile, guest secret not equal to the resident secret, a guest profile refused when `ula_prefix` or `ip6assign` is set or the configs are missing, a revision that drops a committed guest CIDR while that SSID is committed, a named service exception, and refusal to call `OpenWrtAdapter`.

Not owed here:

- Live radio writes (`mjolnir-mesh-lpv`).
- Health probes (`mjolnir-mesh-2mgv`).
- Biscuit issuance (`mjolnir-mesh-ai0.1`).
- CAPPORT and the welcome page.
