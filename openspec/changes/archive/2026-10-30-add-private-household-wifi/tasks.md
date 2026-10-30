# Tasks

`mjolnir-mesh-ai0.7` owns this architecture change.

Decision 2026-10-22: one SSID, three classes, priority only. The
guest-SSID rounds through r7 are superseded. Their reviews stay on
disk and are not authority.

- [x] Fresh advise after preparation (r9 accept, 2026-10-30; contract `openspec/changes/add-private-household-wifi/specs/private-household-wifi/spec.md`)
- [x] R8-1 (advise 2026-10-30): option (b). `10.42.0.0/16` is `no_blanket_route`, not `closed`. Published-name exceptions stay a later apply slice. Committed-policy test asserts the prefix is not closed.
- [x] R8-2 (advise 2026-10-30): client flows are member or unidentified. A published service is a destination.
- [x] R8-3 (advise 2026-10-30): member-ahead SHALL NOT starve unidentified flows. The scheduler stays not owed.
- [x] Superseded the guest-SSID contract (2026-10-22). No second SSID, no guest prefix, no guest isolation gate, no rate cap. Classes are published service, member, and unidentified.
- [x] Class plan builder. One SSID, radio unchanged until a later owner-committed plan. Member flows scheduled ahead of unidentified flows. No rate cap. Published `.mesh` names reachable. Management plane closed. No `OpenWrtAdapter`. `crates/mjolnir-apply/src/client_class.rs`.
- [x] Tests for radio non-mutation, association is not membership, a member scheduled ahead without a cap, an unidentified client reaching a published name and not the management plane, and refusal to call `OpenWrtAdapter`.

Not owed here:

- A second SSID, a guest prefix, roam exclusion, or nftables for a guest class (`mjolnir-mesh-mzgw`, `mjolnir-mesh-fyby.4`, `mjolnir-mesh-nx1k` closed as superseded).
- A hard rate cap.
- Encrypting the one SSID.
- Live radio writes (`mjolnir-mesh-lpv`).
- Health probes (`mjolnir-mesh-2mgv`).
- Biscuit issuance (`mjolnir-mesh-ai0.1`). The plan states the mark and does not invent one.
- CAPPORT and the welcome page.
- Installing a guild admin's SSH key.
