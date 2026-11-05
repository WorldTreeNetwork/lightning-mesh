# Tasks

- [x] Refuse a `holder` fact and a rule whose head predicate is `holder` inside `reject_asserted_holder`, using `query_all` and `Authorizer::dump`, before the verifier injects `holder` or `time`.
- [x] Cover a same-block `holder("victim") <- time($t)`, an appended `holder(<owner>) <- holder(<attacker>)`, a `note` that only mentions `holder(`, and a clean holder check that still authorizes.
- [x] Run `CARGO_TARGET_DIR=/tmp/lm-act-target cargo test -p mjolnir-admin-capabilities --offline --tests`.

Out of scope:

- Replay ledger, hello adapter, iroh (`add-signed-node-control`)
- Editing `add-mesh-admin-capabilities` or folding it
- New operation codes
