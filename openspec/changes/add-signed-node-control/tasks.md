# Tasks

- [x] After `update-grant-holder-rules` lands, reconcile this contract (receipt bytes, ledger store, hello path, iroh name) and only then set ACTIVE BUILD under the `ai0.10` grant.
- [x] Advise the reconciled contract before act. Reader: Astra.
- [x] Fresh advise after preparation

Owed from [2026-11-04 independent advise](reviews/2026-11-04-advise.md), before act:

- [x] Pin atomic reservation/ledger ownership, durable completion, explicit initialization, and consumed reservations across failure/cancellation/restart; clarify at-most-once dispatch and correct the contradictory empty-ledger journey. Add concurrent duplicate, reopen/replay, persistence-failure, and crash-after-reserve verification obligations.
- [x] Pin the shared envelope and exact receipt schema, size/framing/error/timeout rules, hello-to-destination forwarding, and receipt signer binding. Add canonical/signature vectors and a local HTTP-to-iroh destination round trip with an untrusted entry.
- [x] Pin destination-owned authority/time/high-water/challenge/key context, local-node binding, missing-context refusal, and the supported executor/verified-arguments/outcome contract (including plan digest/revision matching if apply is supported). Add wrong-local-node, absent-context, verified-dispatch, and holder-forgery refusal verification obligations.

Remaining wire-contract completions from [2026-11-04 independent re-review](reviews/2026-11-04-advise-2.md), before act:

- [x] Pin iroh request/response framing and exchange boundaries, hello destination selection/forwarding, and malformed-input/absent-context/timeout response behavior distinct from signed outcome receipts. Require a local HTTP-to-iroh round trip plus truncated/trailing/timeout and no-context error cases.
- [x] Pin receipt acceptance against the expected destination key, obligation, and operation; correct the substitution scenario so signature validity alone is insufficient. Explicitly type the outer receipt body field, reject unknown outcomes/non-canonical/trailing receipt data, and require canonical/signature vectors plus valid-signature receipt-substitution tests. Outcome 1 must not imply that no effects occurred.

Contract boxes accepted by [2026-11-04 third independent review](reviews/2026-11-04-advise-3.md). Required implementation and verification remain below.

Later implementation, still owed once the banner is ACTIVE BUILD and advise has accepted:

- [ ] Reserve `ReplayObligation` in a durable ledger before execution. Missing, corrupt, or unreadable state refuses.
- [ ] Forward the same signed envelope through hello HTTP and over authenticated iroh. The destination verifies. HTTP does not carry a bearer admin credential.
- [ ] Return a destination-signed receipt bound to that obligation.
- [ ] Tests: one success through an untrusted forwarder, refusal on altered target or arguments, refusal on replay, refusal when the ledger cannot be read.
- [ ] Ledger verification: concurrent duplicate, reopen/replay, persistence failure, crash after reserve, and persisted monotone high-water floor.
- [ ] Wire verification: local HTTP-to-iroh destination round trip with an untrusted entry; truncated/trailing/oversize input, timeout, and no-context error cases distinct from signed receipts.
- [ ] Receipt verification: canonical/signature vectors, expected destination-key binding, valid-signature substitution for a different expected obligation or operation, and unknown-outcome/non-canonical/trailing-data refusal; outcome 1 does not establish absence of effects.
- [ ] Destination verification: wrong local node, absent context, verified dispatch and plan digest/revision matching, and holder-forgery refusal.

Out of scope:

- Fleet rollout
- Name publish
- New admin wire operation codes
