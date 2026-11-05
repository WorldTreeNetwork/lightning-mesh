# update-grant-holder-rules

## Decision

`reject_asserted_holder` fails closed when either of these is true:

- `query_all("data($h) <- holder($h)")` returns any fact
- `Authorizer::dump` lists a rule whose head name is `holder`

A query error still becomes `AuthzError::Token`. The check `check if holder($fp), $fp == "<hex>"` is not a fact and not a holder-headed rule. A `note` whose text contains `holder(` is neither.

The verifier still injects `holder("<hex>")` after this refusal. `authorize`'s signature and `ReplayObligation` stay as they are.

## Why this shape

`query_all` during parse applies rules only against facts that already hold. `holder("victim") <- time($t)` in the same block as the check is invisible until something adds `time`. `holder("<owner>") <- holder("<attacker>")` fires once the verifier injects the presented holder. Dumping rule heads catches both before injection. That is the check identikey-capability shipped in `e4fe668`.

## Unverified hypotheses

None. The spike `holder-rule-dump` was skipped on 2026-11-04. Upstream tests and the pinned biscuit-auth 6.0.0 API are the evidence. This change's own tests are the mesh check.
