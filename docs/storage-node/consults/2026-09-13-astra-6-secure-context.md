# Consult: secure context, certificates, SSH-free control plane, mirroring

- **Reader:** Astra-6 (`gpt-6-astra` via Codex, read-only, high effort)
- **Date:** 2026-09-13
- **Requested by:** Duke
- **Brief:** secure context on `.mesh`, certificate custody, identity migration, SSH-free publish/admin, storage-node mirroring
- **Verbatim output below.** The synthesis and our proposal: [secure context and control plane](../../network-coordination/secure-context-and-control-plane.md).

---

> **CONSULT:** caution

**Recommend a key-authenticated offline core, with public HTTPS as an optional browser access layer.** Unmodified browsers, indefinite offline HTTPS, and freedom from external certificate authorities cannot all be guaranteed simultaneously. Preserve sovereignty in keys, protocols, and exportable data; accept that browser convenience can expire.

1. **Keep `http://hello.mesh` as discovery; give HTTPS endpoints distinct identities.**

   Hello currently starts an HTTP server ([main.rs:42](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/crates/mjolnir-hello/src/main.rs:42)). Keep its anycast name for walk-up discovery and legacy recovery. Offer `https://n-<node-key-hash>.<mesh-label>.<domain>` for each router’s HTTPS front desk, using a local TLS terminator and a certificate key held only by that router.

   Apps receive separate certificates and stable, key-qualified origins such as `https://a-<app-key-hash>.<mesh-label>.<domain>`. App hosts retain their TLS keys. Friendly `.mesh` names remain discovery aliases, not durable security identities.

   **Nobody holds a fleet-wide hello certificate key.** Separate certificates for the same shared hostname would still authorize every participating router to serve the same browser origin. Unique node origins intentionally abandon shared browser keystore roaming; extension/native custody restores portable identity.

   Use delegated public DNS and DNS-01, with bring-your-own-domain support. Delegation automates operation; it does not remove the parent domain’s authority. Strictly, `.mesh` lacks public DNS delegation; Public Suffix List membership is a separate browser-isolation concern.

   Prefer forwarding only configured alias zones into the CRDT-backed responder. Scope dnsmasq rebind exceptions narrowly; reject unknown Host/SNI values, validate browser origins, and use host-only `__Host-` cookies. Never disable rebinding protection globally.

2. **Make certificate expiry an explicit browser availability limit.**

   DNS-01 permits issuance without exposing the app publicly, including delegated challenge zones, but issuance still needs reachable public DNS and a CA. A connected courier can transport challenges and completed certificates while private keys stay on hosts. It cannot renew an entirely disconnected mesh indefinitely. [Let’s Encrypt challenge documentation](https://letsencrypt.org/docs/challenge-types/)

   As of September 2026, LE’s default `classic` profile remains 90 days; `tlsserver` is 45 days; `shortlived` is approximately six days. Default validity becomes 64 days on **February 10, 2027**, then 45 days on **February 16, 2028**. Use `classic` initially, opportunistic renewal plus ARI, and display remaining offline validity. Refuse the six-day profile for this use case. [Profiles](https://letsencrypt.org/docs/profiles/), [transition schedule](https://letsencrypt.org/2025/12/02/from-90-to-45.html)

   A private per-mesh CA is an optional managed-device deployment: local renewal works offline, but users must install trust, and the CA becomes an impersonation authority. On iOS, manual profile installation additionally requires enabling SSL trust. Keep its root off routers. It does not satisfy zero-install walk-up. [Apple guidance](https://support.apple.com/en-ie/102390)

   Public certificates expose names through CT. Use opaque app-specific identifiers, never person names or sensitive service labels; opaque identifiers still permit correlation. [LE CT policy](https://letsencrypt.org/ca/docs/ct-logs/)

   Open feature-rich apps **top-level** initially. HTTPS inside an HTTP parent is still not a secure context; an extension signer does not upgrade the page’s camera/microphone permissions. HTTPS embedding later requires secure ancestors and appropriate permissions. [Secure Contexts](https://www.w3.org/TR/secure-contexts/)

3. **Treat HTTPS custody as improved soft custody; build hard custody outside router-delivered JavaScript.**

   Current storage explicitly persists `secretKey` bytes in IndexedDB ([storage.ts:11](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/hello-mesh-web/src/lib/identity/storage.ts:11)). HTTPS prevents network injection, but its authorized server still controls the code.

   Non-extractable WebCrypto keys reduce direct export; malicious same-origin code can still request signatures and decrypt accessible data. A cached audited bundle improves availability, but **Service Worker TOFU is not durable code pinning**: worker update fetching bypasses the incumbent worker. The origin can replace its supposed verifier. [WebCrypto security considerations](https://www.w3.org/TR/webcrypto/#security-considerations), [Service Worker update algorithm](https://www.w3.org/TR/service-workers/#update-algorithm)

   The honest minimum for hard custody is an independently installed signer with its own trusted approval UI: native app first, extension where supported. It must validate structured operations, target keys and audiences, never offer arbitrary signing to pages. Pure zero-install HTTP walk-up cannot provide that boundary. Preserve the embedded-assert framing refusal; it addresses clickjacking independently of TLS ([design.md:13](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/openspec/changes/add-embedded-assert/design.md:13)).

   For exit, keep a recoverable root with user-held encrypted backup or recovery material; delegate routine signing to protected device keys. A non-extractable root with no portable recovery fails the exit test.

4. **Separate ephemeral discovery leases from durable certificate authorization.**

   Today’s signature covers challenge, name and port, while gossip ingestion skips signature verification ([routes.rs:201](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/crates/mjolnir-hello/src/routes.rs:201), [meshd.rs:1382](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs:1382)). Ownership expires after one hour ([leased_name.rs:40](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/crates/mjolnir-mesh/src/crdt/leased_name.rs:40)). **Do not connect this directly to ACME.**

   Introduce versioned owner-signed records covering namespace, endpoint identity, scheme/port, sequence, validity and delegation. Verify at every consumer; node-vouched addressing must remain explicitly separate. Unsigned renewal timestamps cannot establish owner-authorized freshness.

   For issuance, the owner signs a narrowly scoped authorization containing the exact FQDN, TXT digest, ACME account, CSR public-key hash, nonce and expiry. A replaceable authoritative-DNS adapter verifies it and updates only the matching challenge record. Automated hosts receive renewable, limited issuance capabilities.

   A one-hour reclaim cannot invalidate a weeks-long certificate—or old browser storage and approvals. Therefore **never recycle a security origin across owners**: the new owner gets a new key-qualified hostname. Partitions can disagree about a friendly name; CRDT convergence is not global exclusive ownership. Public DNS remains authoritative for its convenience namespace, not the mesh.

5. **Use one typed control protocol, enforced at the destination node.**

   Keep the existing loopback API private ([meshd.rs:1611](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs:1611)). Add typed admin RPC over authenticated iroh connections. Lightning Admin calls it directly; hello exposes a constrained HTTP adapter that forwards signed envelopes through any router. The destination verifies authorization independently of the proxy.

   Bind signatures to target node, operation, complete arguments, challenge, authorization epoch and request ID. Persist replay protection and return authenticated receipts. Encrypt sensitive payloads end-to-end. Never transmit admin bearer credentials over HTTP.

   Name-owner keys authorize their own publish/unpublish. Node-owner capabilities authorize radio configuration, node-scoped publishing and local hosting resources. Publishing a pointer does not grant permission to install software or consume someone else’s SSD. Bootstrap node ownership through physical pairing or an existing authenticated owner—not “first gossip claim wins.”

   HTTP soft keys may perform bounded self-service publishing. Require independently paired signer capabilities for ownership transfer, firmware/code installation, radio/routing changes, privileged hosting and backup destruction. Browser UI can propose these actions; trusted signer UI approves them. Signatures cannot prove “hard custody”: enforce scoped capabilities issued during trusted pairing, never a client-supplied custody flag.

6. **Replicate explicit content, and migrate identity conservatively.**

   On Pi nodes, use **existing CRDT metadata plus signed content-hash pointers and opt-in `iroh-blobs` pulls**. Blobs supplies content-addressed transfer; it does not supply authorization, retention or application-state replication. [Iroh documentation](https://docs.iroh.computer/protocols/blobs)

   Mirror static bundles, manifests, timestamped directory snapshots and client-encrypted backups. Require publisher permission, storage-owner quotas and explicit pins. Snapshots must not resurrect expired services. Running a mirrored backend needs its own state-consistency design. Replicas use their own origins/certificates; byte replication never authorizes serving another app’s origin.

   Never automatically mirror private keys, CA/DNS credentials, sessions, plaintext private data or arbitrary executable workloads. Keep a complete user-controlled backup/export; deletion cannot be guaranteed across untrusted replicas.

   Preserve the old HTTP origin’s export page. Prefer recovery-phrase import into the trusted signer, verify the public key, then export/import associated data. **Use the existing mnemonic-to-entropy mapping, not BIP39 wallet seed derivation** ([mnemonic.ts:20](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/hello-mesh-web/src/lib/identity/mnemonic.ts:20)). A one-time bridge is optional convenience, still vulnerable to the old HTTP origin; transfer no secrets through URLs. Cross-signing supports rotation where RPs implement it, but neither migrates data automatically nor cures prior compromise.

**I would refuse:** shared wildcard keys, certificate-warning bypasses, Service Workers marketed as hard custody, globally exposed loopback controls, and automatic mirroring of everything published.

**Biggest tradeoff:** sovereign indefinite-offline operation requires an installed trusted client; zero-install browsers retain a certificate renewal dependency.

**Build order:** first export/recovery and verified signed records; second desktop signer, physical pairing and typed publish/admin RPC; third per-host HTTPS aliases and renewal; fourth opt-in Pi replication and restore drills; finally mobile custody and secure embedding.

**Risks/unknowns:** test offline iroh discovery, clock skew, partitioned ownership/revocation, browser private-DNS behavior, certificate expiry, mobile installation availability, and complete exit restoration. Read-only review; no implementation or runtime validation performed.