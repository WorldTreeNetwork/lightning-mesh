# Tasks

- [x] Sign `keyspace.admit-prefix` with `keyspace`, `peer`, and a 6-byte `prefix` on the existing guild log
- [x] Accept that kind in the log reader without changing roster, profile, epoch, or assigned nodes
- [x] Admit a pair only when both directions verify, each target matches `keyspace`, each actor is an `admin` in the caller-supplied snapshot, and each prefix is `house_ula_prefix` of the other guild
- [x] Test both directions with admins `a1` and `b2`, one direction, a self-pair, a stock prefix, an unrelated actor `ee`, a `member`, a missing snapshot, a mismatched target, and the fixture guilds `0x11` and `0x22`
- [x] Keep signing from staging the client SSID or writing Babel, the TUN, or a `10.42` claim
- [x] Fresh advise after preparation

## Owed from advise (2026-11-01, astra-arch-review)

- [x] Pin authenticated signer-to-house authority in the design and foreign-prefix-permit delta: authority source, eligible role, evaluation time, and fail-closed behavior for absent or unverifiable authority; the display roster alone is not authenticated authority
- [x] Add acceptance scenarios for an unrelated actor signing both reciprocal directions, an ineligible member, missing authority, and a valid authorized pair; record the authority fixture needed for the `0x11` / `0x22` vectors
- [x] Pin verifier rejection of a missing or mismatched `target_fp` versus body `keyspace`, retain exact raw-byte field lengths, and add negative scenarios using an otherwise authorized signer
