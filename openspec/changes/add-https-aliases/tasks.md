# Tasks

Authoritative owed work is bead `mjolnir-mesh-b6j.1`. These boxes mirror what
this change owes once activated.

- [x] Human activation (banner to ACTIVE BUILD) — Duke named `add-https-aliases` / `b6j.1` 2026-09-15
- [x] Advise by an independent non-Claude reader (Sol, Astra-6 or Grok); accept before act
  - r1 send-back, r2 send-back, r3 **accept** (`reviews/2026-09-15-advise-r3.md`, READER sol-arch-review)
- [x] Resolve advise open questions: 80-bit RFC 4648 base32; resolution records in this change (not ai0.9); tiny_http+rustls/ring measured on aarch64; WTN best-effort adapter for the project zone (`design.md` Pins)
- [ ] Label function shared by meshd, mjolnir-hello and hello-mesh-web, with test vectors
- [ ] meshd: forward and rebind-exempt only `<mesh-label>.<zone>`; `AliasTable` answering from verified owner-signed records; negative tests (forged record, parent zone)
- [ ] Issuance authorization format (`mjolnir-https-issuance:v1`), signer side and verifier side, with replay, expiry and wrong-key vectors
- [ ] DNS adapter (replaceable; runs for `mesh.worldtree.network` and for bring-your-own zones)
- [ ] ACME client on app hosts and routers (DNS-01, `classic` profile, ARI, gateway courier)
- [ ] mjolnir-hello TLS on router `n-` origin; cert key in `/etc/mjolnir/` kept across sysupgrade; RAM, flash and handshake measured on aarch64 fleet hardware
- [ ] Directory projection of cert expiry and renewal state; hello.mesh Services display
- [ ] Docs: `docs/join` (services and HTTPS), `docs/deploy/mesh-app-publishing.md`, storage-node D4 marked decided
- [x] Advise send-back: reconcile `proposal.md` ownership of HTTPS resolution records and specify the versioned, canonical owner-signed AliasTable record plus conflict/removal behavior and verification vectors
- [x] Advise send-back: distinguish label owner/node authorization keys from host-local TLS CSR keys, and make 80-bit label collisions fail closed end-to-end with negative vectors
- [x] Advise send-back: pin the byte-exact issuance-authorization encoding, normalized identifiers, signature envelope/domain separation, and golden/malformed/wrong-CSR vectors
- [x] Re-advise after send-back amendments (same non-Claude family)
- [x] Re-advise r2 send-back: compact JSON (no whitespace, fixed key order, unescaped `/`, shortest-decimal integers) plus three-line envelope; golden-byte vectors owed at implementation
- [x] Re-advise r2 send-back: per-label owner_pubkey collision set never GC; equal-seq different payloads fail closed
- [x] Re-advise after r2 send-back amendments (same non-Claude family) — r3 accept
