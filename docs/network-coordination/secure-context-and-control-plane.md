# Secure context, certificates, and an SSH-free control plane

**Status: proposal, awaiting Duke's decision (2026-09-13).** Nothing here is
built. Beads: `mjolnir-mesh-b6j.1` (certificates), `mjolnir-mesh-b6j.2`
(control plane), epic `mjolnir-mesh-b6j` (storage nodes).

Sources: an independent recommendation (Opus 5) and an architecture consult
with Astra-6. The consult is saved verbatim in
[consults/2026-09-13-astra-6-secure-context](../storage-node/consults/2026-09-13-astra-6-secure-context.md).
Where the two differed, the choice and the reason are recorded below.

## The problem

1. **No secure context.** `http://hello.mesh` and every `.mesh` app are plain
   HTTP. Browsers withhold these from insecure pages:
   - non-extractable WebCrypto keys
   - Service Workers
   - passkeys
   - camera, microphone and location
   - app install

   `.mesh` has no public DNS delegation, so no public certificate authority
   will issue for it. Today's IdentiKey is therefore soft custody: key bytes
   in IndexedDB, readable by whichever router serves the page
   (`hello-mesh-web/src/lib/identity/storage.ts`). Apps that need HTTPS use
   self-signed certificates and trigger browser warnings.
2. **SSH is the control plane.** `mjolnir-meshd publish` talks to a
   loopback-only control API, and Lightning Admin shells out to SSH scripts.
   Neither works for people who aren't the fleet maintainers.
3. **Storage nodes** (Raspberry Pi 5) are arriving. We need to decide what
   they may mirror.

## The hard limit

A zero-install browser, indefinite offline HTTPS, and freedom from every
external certificate authority can't all hold at once. Public certificates
expire and renewal needs the internet. A private CA needs a trust profile
installed on every device. So:

> **Sovereignty lives in keys, protocols and exportable data. Browser
> convenience is allowed to expire.**

That's consistent with the exit test. Losing HTTPS convenience when you
leave the project's domain is acceptable. Losing keys or data isn't.

## Proposal

### 1. Two tiers of access

| Tier | Origin | Gives | Custody |
|---|---|---|---|
| **Walk-up** (unchanged) | `http://hello.mesh`, anycast on every router | Discovery, directory, soft IdentiKey, `/assert` sign-in | Soft. Stated honestly on the page |
| **Secure** (new) | Key-qualified HTTPS origins under a delegated domain, for example `https://a-<app-key-hash>.<mesh-label>.<domain>` for apps and `https://n-<node-key-hash>.<mesh-label>.<domain>` for a router's own HTTPS front desk | Secure context for apps and front desks | Better soft custody. Still not hard |
| **Hard custody** (new) | An installed signer: Lightning Admin desktop first, then a browser extension, then mobile | Keys the serving host can't read, and approval UI the host can't fake | Hard |

- Friendly `.mesh` names stay **discovery aliases**. The key-qualified HTTPS
  hostname is the security identity.
- The mesh DNS responder answers the delegated domain locally with mesh
  addresses, so HTTPS names work offline. It forwards only the configured
  alias zone into the CRDT-backed responder. The rebind-protection exception
  is scoped to that zone only.
- Bring-your-own-domain is supported, so a mesh never depends on the
  project's domain.

**Decided against one shared HTTPS hello origin.** Anycast HTTPS would need
the same certificate key on every router, so any router operator could serve
the origin. A single designated host (a storage node) was the other option,
but that creates an implicit authority. Per-host origins give up
browser-keystore roaming between routers. The installed signer restores
portable identity.

### 2. Certificates

- **Challenge.** ACME DNS-01 through a replaceable authoritative-DNS adapter
  for the delegated domain. The app host keeps its TLS private key. There are
  no wildcard keys and no keys copied to routers.
- **Authorization.** An issuance is authorized by an **owner-signed issuance
  authorization** covering:
  - the exact FQDN
  - the TXT record digest
  - the ACME account
  - the CSR public-key hash
  - a nonce and an expiry

  The DNS adapter verifies it and writes only that challenge record.
  Automated hosts get renewable, narrowly scoped issuance capabilities.
- **Don't wire the one-hour name lease straight into ACME.** A lease that can
  be reclaimed in an hour can't revoke a certificate valid for weeks. And the
  browser keeps site storage and approvals for an origin. So **a security
  origin is never reused across owners**: a new owner gets a new key-qualified
  hostname.
- **Validity and renewal.** Use Let's Encrypt's `classic` profile (90 days
  today; 64 days from 2027-02-10; 45 days from 2028-02-16). Renew
  opportunistically whenever any gateway has internet, using ARI. Show the
  remaining offline validity in the UI. Don't use the 6-day profile.
- **Certificate Transparency** publishes every name. Hostnames use opaque
  key-derived labels, never person names or sensitive service names.
- **Private per-mesh CA** stays an option for managed-device deployments only.
  Its root is kept off routers. It's not a walk-up path: iOS needs a manual
  profile install plus enabling trust.

### 3. Identity and custody

- Keep the embedded-identity framing refusal
  ([add-embedded-assert](../../openspec/changes/add-embedded-assert/design.md)).
  It defends against clickjacking whether or not TLS exists.
- **HTTPS alone isn't hard custody.**
  - Non-extractable WebCrypto keys stop export, but code on the same origin
    can still request signatures.
  - **Service Worker pinning isn't durable.** Update fetches bypass the
    installed worker, so the origin can replace its own verifier. We won't
    describe it as a custody guarantee.
- The **installed signer** is the hard-custody boundary:
  - it validates structured operations, target keys and audiences
  - it never signs arbitrary bytes for a page
  - it keeps a recoverable root (the recovery phrase) so exit still works
  - routine signing can use protected device keys
- **Migration from `http://hello.mesh`:**
  - Keep the old origin's export page.
  - Users import their recovery phrase into the signer.
  - The signer verifies the public key matches.
  - No secrets travel in URLs.
  - The phrase is the existing **24-word encoding of the raw 32-byte key**
    (`hello-mesh-web/src/lib/identity/mnemonic.ts`). It is **not** BIP39 wallet
    seed derivation, so importers must use the same entropy mapping.
- **Mini-app consequence.** An HTTPS iframe inside the plain-HTTP hello page is
  still not a secure context. Mini-apps that need secure-context features open
  **top-level** until hello itself has a secure origin.

### 4. Signed records (prerequisite)

Today a name claim is signature-checked only by the router that ingests it.
Gossip ingest skips verification (`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`).

- Introduce **versioned owner-signed name records** covering namespace,
  endpoint identity, scheme and port, sequence, validity window and
  delegation.
- **Every consumer verifies them.**
- Node-vouched facts (for example the observed IP) stay explicitly separate
  from owner-signed facts.

### 5. Control plane without SSH

- **One typed admin protocol over authenticated iroh connections.**
  - Lightning Admin calls it directly.
  - mjolnir-hello exposes a constrained HTTP adapter that forwards signed
    envelopes through any router.
  - **The destination node verifies authorization itself.** The forwarding
    router isn't trusted.
  - The existing loopback control API stays private.
- **Every request is bound** to the target node, operation, complete
  arguments, challenge, authorization epoch and request id. Replay protection
  is persisted, and nodes return signed receipts. Sensitive payloads are
  encrypted end-to-end. No bearer admin credentials over HTTP.
- **Who may do what:**

| Operation | Authorized by | Signer required |
|---|---|---|
| Publish, renew or unpublish a name you own | That name's owner key | Browser key is enough (bounded self-service) |
| Pin a device name on a router's LAN, use a node's hosting or storage | Node owner capability for that node | Installed signer |
| Radio or routing changes, firmware or app install | Node owner capability | Installed signer |
| Ownership transfer, destroying backups | Node owner capability | Installed signer |

- **Bootstrapping node ownership** uses **physical pairing** or an existing
  authenticated owner, never "first gossip claim wins". The WPS button is
  already our physical-presence primitive (`mjolnir-wan-admin`), and a pairing
  window on a button press fits it.
- Capabilities are scoped and issued at trusted pairing. Nodes never trust a
  client-declared "hard custody" flag.

### 6. What storage nodes mirror

- **Primitive:** existing CRDT metadata, plus **signed content-hash
  pointers**, plus **opt-in `iroh-blobs` pulls**. Blobs moves bytes. It
  doesn't decide authorization, retention or app state.
- **Mirror**, with publisher permission, storage-owner quotas and explicit
  pins:
  - static app bundles and manifests
  - timestamped directory snapshots (which must never resurrect expired
    services)
  - client-encrypted user backups
- **Never mirror:**
  - private keys, CA or DNS credentials, sessions
  - plaintext private data
  - arbitrary executable workloads
- **A replica serves under its own origin.** Copying bytes never grants the
  right to serve another app's origin.
- Users keep a complete export. Deletion can't be guaranteed across replicas,
  and the docs must say so.

## Build order

1. **Exit and integrity first.** Export and recovery flows. Owner-signed
   name records verified by every consumer.
2. **Hard custody and control plane.**
   - desktop signer in Lightning Admin
   - physical pairing (WPS-button window)
   - typed publish and admin RPC
   - the hello HTTP adapter
3. **HTTPS.** Delegated domain and DNS adapter, per-host key-qualified
   certificates, renewal and offline-validity display.
4. **Storage nodes.** Opt-in content-hash replication on Pi nodes, plus
   restore drills.
5. **Later.** Mobile signer, then secure embedding once hello has a secure
   origin.

## Decisions (steer 2026-09-15, Duke)

1. **Delegated zone: `mesh.worldtree.network`** is the default, with
   bring-your-own-domain supported. Apps are
   `https://a-<app-key-hash>.<mesh-label>.mesh.worldtree.network` and router
   front desks are `https://n-<node-key-hash>.<mesh-label>.mesh.worldtree.network`.
2. **HTTPS scope: apps and per-router front desks** in the HTTPS phase. This was
   chosen over apps-first, so every router needs TLS termination and renewal.
   Walk-up `http://hello.mesh` stays.
3. **Signer: Lightning Admin desktop first**, then browser extension, then
   mobile.
4. **Node ownership: physical presence plus reviewed per-device proof.** A WPS or
   storage-node button press is only the presence step. The existing WPS window
   isn't proof on its own (accepted review finding F-C). The ceremony is defined
   in `bf7.1` (`add-household-owner-claim`).
5. **Build order: trust first.** Signed records and export come first, then
   signer, claim and signed control, then HTTPS, then mirroring.

**Governance.** The household campaign `ai0` governs trust and control:
- owner claim `bf7.1`
- grants `ai0.1` (`add-mesh-admin-capabilities`)
- control-record integrity `ai0.9`
- signed node control `b6j.2` (`add-signed-node-control`)
- the shared contract in `openspec/changes/add-household-trust-contract`

This document keeps HTTPS (`b6j.1`, change `add-https-aliases`), storage-node
mirroring (`b6j`) and name-record signing beyond `ai0.9`'s admin scope.
Sections 3 and 5 above are direction; where they overlap `ai0`, the household
trust contract is authoritative.

Storage nodes use one software image. The inference role turns on when a Hailo
device is detected (D1).

## Risks to test

- offline iroh discovery for the admin protocol
- clock skew against signature validity windows
- partitioned ownership and revocation
- browser Private DNS behaviour with the delegated domain
- certificate expiry during long offline periods
- mobile signer install friction
- a full exit-and-restore drill
