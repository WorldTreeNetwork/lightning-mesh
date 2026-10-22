# Tasks

`mjolnir-mesh-bf7.1` owns this instrument change.

- [ ] Independent architecture advise of the three-factor ceremony
- [ ] Claim persistence: device signature, provisioning-secret hash, 120 second button window, single winner
- [ ] Recovery: both factors, epoch binding, retire prior recovery credentials
- [ ] Tests for racing claimants, secret-only, button-only, remote secret, and hub handlers not receiving owner key material

Not owed here:

- Live fleet or radio writes.
- WPS protocol.
- Biscuit engine implementation (`mjolnir-mesh-ai0.1`).
- Chain-merge verification (`add-control-record-integrity`).
