# Tasks

`mjolnir-mesh-bf7.1` owns this instrument change.

- [x] Independent architecture advise r1 send-back (`reviews/2026-10-22-advise.md`)
- [x] Answer R1–R4 in design.md and the spec delta (2026-10-22)
- [x] Fresh advise r2 send-back B1–B3 (`reviews/2026-10-22-advise-r2.md`)
- [ ] Fresh advise after preparation
- [ ] Claim persistence: card HMAC, client-checked device signature, 120 second button window, single winner, reset clears the verifier
- [ ] Recovery: both factors, ownership and authority epoch advance, retire prior recovery credentials
- [ ] Button dispatch: unclaimed or pending recovery arms the ceremony only; otherwise the existing WAN-SSH toggle
- [ ] Tests for racing claimants, secret-only, button-only, sniffed HMAC with a substituted owner key, ceremony press does not open WAN SSH, reboot closes the window, five-attempt cap, and hub handlers not receiving the raw secret

Not owed here:

- Live fleet or radio writes.
- WPS protocol.
- Biscuit engine implementation (`mjolnir-mesh-ai0.1`).
- Chain-merge verification (`add-control-record-integrity`).
