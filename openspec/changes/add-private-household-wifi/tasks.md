# Tasks

`mjolnir-mesh-ai0.7` owns this architecture change.

- [ ] Fresh advise after preparation
- [ ] Resident and optional guest plan builder. Passphrase stays out of the four UCI files, receipts, and hello.mesh. Uncommitted plans leave the open client SSID unchanged.
- [ ] Isolation gate: a plan is labeled isolated only when guest-to-resident and cross-node denies hold for IPv4 and for IPv6 if IPv6 is enabled, with no `br-lan` bridge. Otherwise the plan is refused and the previous revision stays.
- [ ] Tests for open-SSID non-mutation, a different SSID as a different profile, guest secret not equal to the resident secret, missing IPv6 deny when IPv6 is enabled, a named service exception, and refusal to call `OpenWrtAdapter`.

Not owed here:

- Live radio writes (`mjolnir-mesh-lpv`).
- Health probes (`mjolnir-mesh-2mgv`).
- Biscuit issuance (`mjolnir-mesh-ai0.1`).
- CAPPORT and the welcome page.
