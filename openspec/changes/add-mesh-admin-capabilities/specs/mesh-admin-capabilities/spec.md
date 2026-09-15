## ADDED Requirements

### Requirement: Complete destination authorization
The library SHALL verify the complete Biscuit signature chain and checks against
externally trusted issuer, holder, house, node, operation, authority epoch and
time evidence. Unverified request fields and token facts SHALL NOT confer trust.

#### Scenario: Forged trusted facts
- GIVEN a token containing attacker-supplied holder, time or authority facts
- WHEN those facts would make an otherwise forbidden request appear valid
- THEN authorization is refused rather than treating them as verifier evidence

### Requirement: Narrow-only grants
V1 grants SHALL be holder-bound and owner-approved. Attenuation SHALL only reduce
their effective operation, node and time scope. Holder changes SHALL require new
authorized issuance rather than weakening an existing holder check.

#### Scenario: Attenuation contrast
- GIVEN a valid configuration grant for two nodes
- WHEN attenuated to diagnostics on one node
- THEN that diagnostic succeeds and configuration or another node is refused

### Requirement: One canonical signed request
The library SHALL own the versioned canonical bytes binding the complete request,
token identity and challenge to its signer. Consumers SHALL reuse that encoder.

#### Scenario: Modified arguments
- GIVEN a valid signature over a canonical request
- WHEN any target, operation, argument, epoch, token identity or challenge changes
- THEN verification fails

### Requirement: Bounded validity with explicit execution preconditions
Configuration grants SHALL default to900 seconds and SHALL NOT exceed the v1
3600-second policy ceiling. Missing trustworthy validity evidence SHALL deny.
An authorization result SHALL explicitly require downstream durable freshness
reservation before execution and SHALL NOT claim replay-safe execution itself.

#### Scenario: Expiry is not renewed by traffic
- GIVEN an expired administrative grant and continued requests
- WHEN no fresh authorization has issued a replacement
- THEN all privileged requests are denied independently of ordinary connectivity
