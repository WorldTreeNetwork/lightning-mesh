## ADDED Requirements

### Requirement: Key-qualified HTTPS origins

The system SHALL name HTTPS endpoints
`a-<app-key-label>.<mesh-label>.<zone>` (app hosts) and
`n-<node-key-label>.<mesh-label>.<zone>` (router front desks). Each label SHALL
be derived deterministically from the **owner authorization key** (app owner
Ed25519, or the node identity key for `n-` names), and `<mesh-label>` from the
house identity. The TLS leaf key SHALL be generated on the serving host and
SHALL NOT be that owner key. A certificate for the hostname SHALL be issued
only when an issuance authorization signed by the owner key binds
`csr_spki_sha256` of that leaf. A change of owner SHALL produce a different
hostname whenever the 80-bit labels differ. If two distinct owner keys map to
the same 16-character label, AliasTable and the issuance adapter SHALL fail
closed: serve neither name and write no challenge. `<zone>` SHALL default to
`mesh.worldtree.network` and SHALL be configurable per mesh.

#### Scenario: New owner, new origin

- GIVEN an app served at `https://a-<L1>.<M>.mesh.worldtree.network` by owner key K1
- WHEN ownership of that app moves to key K2 whose 80-bit label L2 ≠ L1
- THEN the app is served at `https://a-<L2>.<M>.mesh.worldtree.network` and no certificate for the old hostname is issued to K2

#### Scenario: Label collision fails closed

- GIVEN two verified records whose owner public keys differ but whose 80-bit labels are equal
- WHEN AliasTable or the issuance adapter evaluates either name
- THEN neither address is answered and no ACME challenge is written

#### Scenario: TLS key is not the owner key

- GIVEN an issuance authorization for `a-<L>.<M>.<zone>` signed by owner key K whose label is L, binding CSR SPKI H
- WHEN the host presents a TLS certificate whose SPKI hash is H
- THEN the hostname may be served; the TLS private key is not K

#### Scenario: Bring-your-own zone

- GIVEN a mesh configured with zone `net.example.org`
- WHEN a router front desk gets an HTTPS name
- THEN it is `n-<label>.<mesh-label>.net.example.org` and no request is made to `mesh.worldtree.network`

### Requirement: Offline local resolution of HTTPS names

Every node SHALL answer DNS for `<mesh-label>.<zone>` names from its local
responder without internet access, using only `mjolnir-https-alias:v1` records
whose owner signature it has verified against the full owner public key in the
payload. This change owns that record type (`ai0.9` does not). The node SHALL
forward to its responder, and exempt from DNS rebind protection, only the
suffix `<mesh-label>.<zone>`, never the parent zone. Names without a verified
owner-signed record SHALL NOT resolve locally. For one owner and FQDN, the
highest `seq` wins; a tombstone (highest seq, `valid_until` in the past) SHALL
remove the name. Two different owner keys for the same label SHALL fail closed.

#### Scenario: Internet unplugged

- GIVEN a verified record for `a-<L>.<M>.mesh.worldtree.network` pointing at a storage node
- WHEN every gateway loses internet
- THEN a client on the mesh still resolves the name to the storage node's mesh address

#### Scenario: Forged record

- GIVEN a record for an HTTPS name whose owner signature does not verify
- WHEN it reaches a node through gossip
- THEN that node does not answer DNS for the name from that record

#### Scenario: Parent zone not whitelisted

- GIVEN a mesh with label M
- WHEN dnsmasq configuration is reconciled
- THEN only `M.mesh.worldtree.network` is forwarded and rebind-exempt, not `mesh.worldtree.network`

### Requirement: Owner-authorized certificate issuance

A certificate for an HTTPS name SHALL be obtained with ACME DNS-01. The TLS
private key SHALL remain on the serving host and SHALL NOT be the owner
authorization key. The DNS adapter SHALL publish a challenge record only for a
`mjolnir-https-issuance:v1` authorization whose Ed25519 signature verifies
against the full owner public key the label derives from. Signed bytes SHALL be
the domain line `mjolnir-https-issuance:v1\n` plus canonical JSON with keys in
this order: `fqdn` (lowercase ASCII, no trailing dot), `txt_digest` (unpadded
base64url SHA-256 of the ACME key authorization), `acme_account` (ACME account
URL), `csr_spki_sha256` (64 lowercase hex SHA-256 of the CSR SPKI DER),
`nonce` (32 lowercase hex chars), `expires_at` (decimal unix seconds, ≤ now+3600).
The adapter SHALL reject replayed nonces, expired authorizations, wrong-owner
signatures, FQDNs whose label is not derived from the signing key, and CSRs
whose SPKI hash is not `csr_spki_sha256`. The system SHALL NOT issue or
distribute wildcard keys or share a TLS private key between hosts.

#### Scenario: Authorization for someone else's name

- GIVEN an issuance authorization for `a-<L>.<M>.<zone>` signed by a key whose label is not L
- WHEN it is submitted to the DNS adapter
- THEN no challenge record is written

#### Scenario: Wrong CSR public key

- GIVEN a valid owner signature for label L binding `csr_spki_sha256` H1
- WHEN the ACME CSR's SPKI hash is H2 ≠ H1
- THEN no challenge record is written and no certificate is issued

#### Scenario: Replayed authorization

- GIVEN an issuance authorization already used once
- WHEN it is submitted again before its expiry
- THEN no challenge record is written

#### Scenario: Offline host, online gateway

- GIVEN an app host with no internet and a gateway with internet
- WHEN the host's owner-signed authorization and CSR are relayed through the gateway
- THEN a certificate is issued and returned without the host's private key leaving the host

### Requirement: Honest renewal and expiry

Nodes SHALL renew HTTPS certificates opportunistically whenever a gateway has
internet, using the CA's default 90-day-class profile and renewal information.
Nodes SHALL publish each name's certificate expiry and renewal state
(`ok`, `due`, `failing`, `expired`) in the directory. hello.mesh SHALL display
each name's remaining validity and state. A 6-day-class short-lived profile
SHALL NOT be used. hello.mesh SHALL NOT link an HTTPS name whose certificate has
expired as if it were valid.

#### Scenario: Months offline

- GIVEN a name whose certificate has 10 days left and no gateway with internet
- WHEN a visitor opens Services
- THEN the name shows "10 days of HTTPS left offline"

#### Scenario: Expired while offline

- GIVEN a name whose certificate expired during an outage
- WHEN a visitor opens Services
- THEN the name is labeled expired with how to renew, and no HTTPS link is presented as valid

### Requirement: Router HTTPS front desk

Each router SHALL serve its front desk over TLS at its own
`n-<node-key-label>.<mesh-label>.<zone>` origin, using a certificate key that
never leaves the router and survives sysupgrade. The router SHALL continue to
serve walk-up `http://hello.mesh` unchanged. A router SHALL NOT serve another
router's HTTPS origin.

#### Scenario: Walk-up still works

- GIVEN a router with a valid HTTPS front desk
- WHEN a visitor types `http://hello.mesh`
- THEN the plain-HTTP front desk loads as before

#### Scenario: Wrong router

- GIVEN routers R1 and R2
- WHEN a TLS connection for R1's `n-` hostname reaches R2
- THEN R2 does not present a certificate for R1's hostname
