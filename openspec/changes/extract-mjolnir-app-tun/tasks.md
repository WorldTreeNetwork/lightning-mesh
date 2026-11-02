# Tasks

- [x] Add `crates/mjolnir-app-tun` with the address functions, `DatagramConn`, `spawn_encap_pair`, and the Linux session
- [x] Make `mjolnir-mesh` and Admin depend on that crate, and remove the session from Admin
- [x] Sync foreign `/48` routes from prefix bytes the caller passes
- [x] Keep the Admin guild window, including the nested-access blank, in Admin
- [x] Move the session tests and keep `cargo test -p mjolnir-mesh` and the Admin lib tests passing
- [x] Lifecycle tests: each encap direction returns while its sibling stays pending, and Drop of an active session runs before the supervisor polls; assert both tasks are aborted and the owned address and routes are gone, not only the down flag
- [x] Fresh advise after preparation

## Owed from fresh advise (2026-11-01, astra-arch-review)

- [x] Reconcile `design.md`'s landed-runner claim with the retained cancellation contract: specify ownership that aborts the sibling on either encap task returning and aborts both tasks on session Drop, including before the supervisor first polls. See `reviews/2026-11-01-advise.md` (P1).
- [x] Add focused lifecycle verification to the extraction gates: each encap direction returns while its sibling remains pending, and an active session is dropped; assert both task/resource release and owned address/route cleanup, not only the down flag or address disappearance.
