## ADDED Requirements

### Requirement: Opt-in, durable service identity

The publisher SHALL run only on explicit invocation, with one validated service configuration and a persistent Ed25519 application key. Explicit initialization SHALL create the key with owner-only access on Unix and refuse to overwrite it. Missing, corrupt or unsafe key material SHALL cause startup to fail with an actionable diagnostic. The observer’s identity and application key have independent lifecycles.

#### Scenario: Restart with the same service

- GIVEN an initialized key and a valid service configuration
- WHEN the machine reboots and starts the publisher
- THEN it renews the same name using the same public key without operator intervention.
- WHEN the configured key is missing or invalid
- THEN the command exits unsuccessfully before making a claim and leaves existing key material intact.

### Requirement: Router-mediated signed renewal

For a service accepting TCP connections at the configured local address and port, the publisher SHALL fetch a fresh challenge from the configured LAN gateway and POST to that same gateway’s name-claim endpoint. It SHALL sign the UTF-8 bytes of `mjolnir-name-claim:v1\n<challenge>\n<name>\n<decimal-port>` with literal LF separators and no trailing newline. Each submission SHALL send lowercase hex public key and signature, the challenge, name, port, explicit IP, scheme, and the configured app marker when enabled. The v1 signature covers the challenge, name and port. The ingesting router checks it; the address, scheme and marker are outside that signature.

#### Scenario: Publish and renew across a temporary outage

- GIVEN a valid local service, the publisher attempts a claim at startup and renews every 30 seconds while healthy.
- WHEN the gateway is unavailable or a submission outcome is uncertain
- THEN attempts have finite timeouts, retries are bounded in rate, and the next attempt obtains a fresh challenge with the same key and service metadata.
- WHEN an operator stops the publisher or the service stops accepting connections
- THEN it sends no further renewals while stopped or unhealthy; after 90 seconds without renewal, the router’s freshness filter can hide the listing. Ownership may persist for the existing one-hour lease.
- WHEN connectivity and service health return
- THEN renewal resumes. The publisher never chooses another name or creates another key to resolve a conflict.

### Requirement: Observable submission and independent confirmation

The publisher SHALL report the name, public key, target and outcome in operator-readable output and a documented machine-readable mode. It SHALL distinguish local configuration or health failures, gateway failures and a queued claim. Logs SHALL omit private key material. A successful HTTP response acknowledges that hello.mesh spooled the claim. The router applies ownership rules asynchronously.

#### Scenario: Another key already holds the name

- GIVEN an unexpired lease held by another key
- WHEN the HTTP handler accepts a competing request into the spool
- THEN the publisher reports submission only. Acceptance tests verify the winning owner via the full public key on the observed lease and check the projected address independently; a matching name alone is insufficient.

### Requirement: Discover and open the hosted service

The publisher SHALL interoperate with existing leased-name DNS, directory and mini-app projection. The field acceptance path SHALL include a real request from a client on the other AP.

#### Scenario: Open a Pi-hosted app after reboot

- GIVEN the Pi serves a recognizable response on the AP3000 LAN and a client is associated with the m3000
- WHEN the publisher renews the claim
- THEN an independent observer can see a matching `LeasedNamePublish`, the other router resolves the name to the Pi, and its hello.mesh lists the configured target.
- THEN a client opens the listing and receives the expected response from the Pi.
- WHEN the Pi reboots
- THEN the journey succeeds again with the same name and public key. Exercise **Services** for an ordinary web service and **Apps** with the configured path for a marked mini-app.
