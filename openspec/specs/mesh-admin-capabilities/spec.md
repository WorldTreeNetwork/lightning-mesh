# mesh-admin-capabilities

Grant-engine holder checks that are true today. Folded from
`update-grant-holder-rules` (2026-11-04). The rest of this capability
is still the open change `add-mesh-admin-capabilities` and is not a
claim of this file.

Code: `crates/mjolnir-admin-capabilities/src/engine.rs`.

## Requirements

### Requirement: Holder is only a verifier fact
The grant engine SHALL refuse a token that asserts a `holder` fact or that carries a rule whose head predicate is `holder`. The verifier SHALL be the only party that injects `holder(<fingerprint>)`. A holder check, and text inside another fact that merely contains `holder(`, SHALL NOT count as defining `holder`.

#### Scenario: A rule waits on verifier time
- GIVEN an authority block that checks `holder($fp)` against a fingerprint and also contains a rule `holder("<that fingerprint>") <- time($t)`
- WHEN the token is authorized
- THEN authorization is refused with `HolderFact` before that rule can mint the fact

#### Scenario: An appended rule mints the owner from the presented key
- GIVEN a valid holder check for the owner fingerprint
- AND a later block whose rule is `holder("<owner fingerprint>") <- holder("<attacker fingerprint>")`
- WHEN the attacker presents their own key
- THEN authorization is refused with `HolderFact`

#### Scenario: A mention is not a definition
- GIVEN a valid holder check and a fact whose text contains `holder(`
- WHEN the matching holder presents the request
- THEN that mention does not itself refuse authorization
