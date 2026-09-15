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
Configuration grants SHALL be bounded by the household security profile defined
in the household-authority contract. The profiles are (token default / ceiling /
stale-authority maximum):
- Strict: 15 minutes / 1 hour / 15 minutes
- Standard: 24 hours / 7 days / 24 hours. Standard is the default.
- Relaxed: 7 days / 30 days / 7 days.

Grant-class caps SHALL also bound them, with the shortest applicable cap winning.

Independently of grant expiry, a configuration grant SHALL authorize only while
the verifier holds authority and revocation information verified within the
profile's stale-authority maximum. Otherwise the result SHALL deny and require
owner reauthorization. The verifier SHALL refuse any configuration-changing or
delegated Admin grant without an expiry. Missing trustworthy validity evidence
SHALL deny.

#### Scenario: Unexpired grant with stale authority
- GIVEN an unexpired Standard-profile configuration grant with a 7-day lifetime
- WHEN the verifier's last fresh authority and revocation verification is older than 24 hours
- THEN authorization is denied with a reauthorization-required reason

#### Scenario: Class cap beats profile
- GIVEN a Relaxed-profile grant whose operations include a firewall change and guest Wi-Fi management
- WHEN its token claims a 30-day lifetime
- THEN the verifier treats it as valid for at most 24 hours

#### Scenario: Grant without expiry for mutation
- GIVEN a configuration-changing grant with no expiry
- WHEN it is presented under any profile
- THEN authorization is denied
An authorization result SHALL explicitly require downstream durable freshness
reservation before execution and SHALL NOT claim replay-safe execution itself.

#### Scenario: Expiry is not renewed by traffic
- GIVEN an expired administrative grant and continued requests
- WHEN no fresh authorization has issued a replacement
- THEN all privileged requests are denied independently of ordinary connectivity
