# Tasks

`mjolnir-mesh-bf7.1` owns this instrument change.

- [x] Independent architecture advise r1 send-back (`reviews/2026-10-22-advise.md`)
- [x] Answer R1–R4 in design.md and the spec delta (2026-10-22)
- [x] Fresh advise r2 send-back B1–B3 (`reviews/2026-10-22-advise-r2.md`)
- [x] Fresh advise after preparation (accept, `reviews/2026-10-22-advise-r3.md`)
- [x] Claim persistence: card HMAC, client-checked device signature, 120 second button window, single winner, reset clears the verifier (`crates/mjolnir-mesh/src/owner_claim.rs`)
- [x] Recovery: both factors, ownership and authority epoch advance, retire prior recovery credentials
- [x] Button dispatch: verifier present and unclaimed, or a pending recovery whose credential has verified, arms the ceremony only; no verifier keeps the existing WAN-SSH toggle; owned with no verified pending recovery toggles WAN SSH (`button_effect`, `etc/rc.wps/00-mjolnir-wan-admin`)
- [x] Tests for racing claimants, secret-only, button-only, sniffed HMAC with a substituted owner key, ceremony press does not open WAN SSH, no verifier keeps the toggle, unverified pending recovery does not arm, reboot closes the window, five-attempt cap, and hub handlers not receiving the raw secret (`owner_claim` tests; `mjolnir-hello` `claim_relay` tests)

Not owed here:

- Live fleet or radio writes.
- WPS protocol.
- Biscuit engine implementation (`mjolnir-mesh-ai0.1`).
- Chain-merge verification (`add-control-record-integrity`).
