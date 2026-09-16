# Design: HTTPS aliases under a delegated zone

## Decided upstream (not re-litigated here)

From steer 2026-09-15:
- zone `mesh.worldtree.network` plus bring-your-own
- apps **and** router front desks in this phase
- trust first, so this change depends on owner and issuer model plus signed
  records
- `ai0` governs trust

From the Astra-6 consult (accepted in the proposal):
- no shared or wildcard keys
- never recycle an origin across owners
- `classic` profile plus ARI
- CT names stay opaque
- Service Workers aren't custody

## Names

```
a-<app-key-label>.<mesh-label>.<zone>     app host (storage node, Pi, laptop)
n-<node-key-label>.<mesh-label>.<zone>    router front desk (mjolnir-hello)
```

- **`<app-key-label>`** is RFC 4648 base32 (lowercase, no padding, alphabet
  `a-z2-7`) of the first **80 bits** of blake3 over the owner's Ed25519 public
  key: 16 characters. The same function with the node identity key gives
  `<node-key-label>`. 80 bits is collision-resistant for household-scale keys
  and short enough for a DNS label; a longer label does not buy a recycled
  origin (owner change already mints a new one).
- **Labels are key-bound.** An owner change means a new label, so browser
  storage, approvals and cookies never carry over.
- **`<mesh-label>`** derives from the house identity in
  `add-household-trust-contract` (house genesis record), using the same label
  function. A mesh without a claimed house has no mesh label, so no HTTPS
  names.
- **Rejected alternatives:**
  - human-readable subdomains: they leak to CT logs and invite squatting
  - one label per mesh with wildcard certificates: it would copy the wildcard
    private key to every host

## Resolution (offline)

Today's precedent is `MESH_DNS_SERVER_LINE = "/mesh/127.0.0.1#5335"` plus the
`.mesh` rebind whitelist and `reconcile_dnsmasq_uci`
(`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs:6497–6559`).

- meshd adds exactly one more forward, `/<mesh-label>.<zone>/127.0.0.1#5335`,
  and exactly one more rebind-domain entry for that suffix. The parent zone is
  never forwarded or whitelisted.
- An `AliasTable` (a `NameTable` alongside `ServiceTable` and `LeasedNameTable`
  in `dns_responder.rs`) answers A (and later AAAA) records **only** from
  owner-signed records that every node has verified. Node-vouched addresses may
  be used, but the owner signature is what binds a label to a key.
- Public DNS for the zone serves only ACME TXT records. It never serves mesh A
  records, so private addresses don't leak and off-mesh lookups fail harmlessly.

## AliasTable record (`mjolnir-https-alias:v1`)

This change owns the resolution record. `ai0.9` is admin-record integrity and
may supply verify primitives; it does not own this type.

Signed bytes: domain line `mjolnir-https-alias:v1\n` plus **compact JSON**
(UTF-8, no whitespace, keys in this exact order, `/` unescaped, integers as
shortest decimal with no exponent and no leading zeros except `0`, strings
with only mandatory JSON escapes):

`{"owner_pubkey":"<64hex>","fqdn":"<fqdn>","addr":"<dotted-ipv4>","seq":<uint>,"valid_from":<unix>,"valid_until":<unix>}`

Ed25519 over those exact bytes. Transport envelope is three UTF-8 lines, LF,
no extra spaces:

```
owner_pubkey=<64 lowercase hex>
payload=<the compact JSON above, one line>
sig=<128 lowercase hex>
```

Consumers verify `sig` over `mjolnir-https-alias:v1\n`+payload against
`owner_pubkey`, and check the hostname label derives from that full key.
Highest `seq` for one `(owner_pubkey, fqdn)` wins. Equal `seq` with different
payloads: fail closed (serve neither). A record with `valid_until` in the past
is a tombstone (stops answering that owner+fqdn) but the **owner_pubkey stays
in the per-label collision set forever** (survives expiry, tombstone,
compaction, and process restart). If a new key's 16-character label matches
any retained owner_pubkey, fail closed: answer neither, issue neither.

## Issuance authorization

The host builds a CSR with a **host-local TLS key** (not the owner key). The
owner key (the app owner, or the node identity key for `n-` names) signs
canonical bytes:

Domain line `mjolnir-https-issuance:v1\n` plus **the same compact JSON rule**
(UTF-8, no whitespace, keys in this order, `/` unescaped, integers shortest
decimal):

`{"fqdn":"<fqdn>","txt_digest":"<b64url>","acme_account":"<url>","csr_spki_sha256":"<64hex>","nonce":"<32hex>","expires_at":<unix>}`

Transport envelope (three LF lines):

```
owner_pubkey=<64 lowercase hex>
payload=<the compact JSON above, one line>
sig=<128 lowercase hex>
```

`sig` is Ed25519 over `mjolnir-https-issuance:v1\n`+payload. FQDN v1 is ASCII
only. Implementation tests SHALL include golden signed-byte vectors and
malformed encodings (whitespace inserted, escaped solidus, leading-zero
integers). The adapter's collision set is the same durable per-label
owner_pubkey history as AliasTable.

- **The DNS adapter** verifies all of the following before writing
  `_acme-challenge.<fqdn>` TXT:
  - the signature against the **full** owner public key
  - that the 16-char label derives from that key and no other retained key
    collides on it (fail closed)
  - `expires_at` in the future and ≤ 1 h from issue
  - nonce unseen
  - `fqdn` normalized and matches the label function
  - CSR SPKI hash equals `csr_spki_sha256`

  It keeps an append-only log. It holds no mesh authority and can be run by
  anyone (bring-your-own domain means your own adapter).
- **Courier.** The host may be offline. Any gateway with internet relays the
  signed authorization and polls ACME. Private keys never travel.
- **Automation.** An app host can hold a scoped, renewable issuance capability
  from the owner (per the household grant model) instead of asking the owner
  every renewal. That capability model belongs to `ai0.1`. This change consumes
  it.

## Renewal and display

- Let's Encrypt `classic` profile: 90 days today, 64 days from 2027-02-10, 45
  days from 2028-02-16. Renew whenever a gateway is reachable, following ARI.
- The node publishes `not_after` per name in its directory projection.
  hello.mesh shows remaining validity and the renewal state (ok, due, failing,
  expired).
- **Never:**
  - the `shortlived` profile
  - asking users to bypass certificate warnings
  - silently falling back to HTTP on an `https://` link

## Router front desks

mjolnir-hello is `tiny_http` today, with no TLS. **Pin 3:** `tiny_http` plus
rustls (ring). Measure RAM, flash and handshake on one aarch64 fleet node
before calling that slice done. A separate terminator is fallback only if
that measurement fails the router budget.

The certificate key lives in `/etc/mjolnir/` next to the node secret, is never
exported, and is added to sysupgrade keep. Walk-up `http://hello.mesh` on :80
stays. Anycast is unchanged; each router's HTTPS front desk is its own origin.
Shared browser-keystore roaming across routers is lost by design; the installed
signer is the portable identity.

## Threats

| Threat | Answer |
|---|---|
| Someone gets a cert for another owner's name | The authorization signature must verify against the key the label derives from |
| Ingesting node forges a resolution record | AliasTable only uses records every consumer verifies (prerequisite) |
| DNS rebinding through the whitelisted suffix | The exception covers only `<mesh-label>.<zone>`; hello validates Host and SNI; `__Host-` cookies |
| Adapter compromised | It can deny or misissue DNS challenges, but can't obtain private keys. CT monitoring on the zone flags unexpected certs. Bring-your-own domain removes the dependency |
| Long offline expiry | Visible offline validity; renewal through any gateway; plain `.mesh` still works |
| CT name leakage | Labels are opaque key hashes. The mesh label is still correlatable (accepted) |

## Pins (activation 2026-09-15, decide-for-me)

Closes the four advise questions. Steer 2026-09-15 already decided zone and
HTTPS scope; these are the remaining implementation pins.

1. **Label.** 80-bit blake3 prefix, RFC 4648 base32 lowercase no padding
   (`a-z2-7`), 16 characters. Shared function, shared test vectors, used by
   meshd, mjolnir-hello and hello-mesh-web.
2. **Resolution records vs `ai0.9`.** `ai0.9` is admin-record integrity.
   HTTPS alias **resolution** records (label → mesh address, owner-signed,
   verified by every consumer) stay in **this** change, as the b6j steer
   split it. Do not wait for a third sibling. AliasTable MUST refuse
   unverified records. Issuance authorization is also this change.
3. **Router TLS.** `tiny_http` with the `ssl-rustls` feature, rustls **ring**
   provider (already in mjolnir-hello for the mini-app fetcher). Measure RAM,
   flash and handshake on one aarch64 fleet node before calling the adapter
   slice done. A separate terminator is the fallback only if that measurement
   fails the router budget. Cert key in `/etc/mjolnir/`, sysupgrade-kept.
4. **Default adapter operator.** World Tree Network runs the convenience
   ACME DNS adapter for `mesh.worldtree.network` as **best-effort
   infrastructure**, not a mesh availability dependency. A mesh with no
   adapter (or a down adapter) still has `.mesh` HTTP and local AliasTable
   resolution. Bring-your-own zone means your own adapter. The adapter holds
   no mesh authority and no private keys.
