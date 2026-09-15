# Tasks

Authoritative owed work is bead `mjolnir-mesh-b6j.1`. These boxes mirror what
this change owes once activated.

- [ ] Human activation (banner to ACTIVE BUILD)
- [ ] Advise by an independent non-Claude reader (Sol, Astra-6 or Grok); accept before act
- [ ] Resolve advise open questions: label length and alphabet; resolution-record scope vs `ai0.9`; tiny_http+rustls vs terminator; default adapter operator
- [ ] Label function shared by meshd, mjolnir-hello and hello-mesh-web, with test vectors
- [ ] meshd: forward and rebind-exempt only `<mesh-label>.<zone>`; `AliasTable` answering from verified owner-signed records; negative tests (forged record, parent zone)
- [ ] Issuance authorization format (`mjolnir-https-issuance:v1`), signer side and verifier side, with replay, expiry and wrong-key vectors
- [ ] DNS adapter (replaceable; runs for `mesh.worldtree.network` and for bring-your-own zones)
- [ ] ACME client on app hosts and routers (DNS-01, `classic` profile, ARI, gateway courier)
- [ ] mjolnir-hello TLS on router `n-` origin; cert key in `/etc/mjolnir/` kept across sysupgrade; RAM, flash and handshake measured on aarch64 fleet hardware
- [ ] Directory projection of cert expiry and renewal state; hello.mesh Services display
- [ ] Docs: `docs/join` (services and HTTPS), `docs/deploy/mesh-app-publishing.md`, storage-node D4 marked decided
