# Tasks

- [x] Pure `ula_addr` next to `backhaul_addr` (golden vector
      `fd53:6213:4797:0:1478:922e:5c61:d39f` for the documented node id)
- [x] Assign on `br-mesh` beside `10.254` as `/64`; reconcile after wifi
      reload. Not on `mjolnir0` or `br-lan`
- [x] Project onto `DirectoryNode.ula`; hello `/api/node` serves it
- [x] iroh candidate list: `check_reachability` errors if any candidate
      is Unique Local
- [x] ARCHITECTURE.md / ipv6-parallel-plane.md: exact derivation

Not owed here (bullets, not boxes):

- RA on client AP (`v6.3`)
- babel IPv6 export (`v6.4`)
- AAAA (`v6.5`)
- App TUN (`v6.6`)
- Fold (after act + advise accept)
- `IFA_F_NODAD` netlink flag (design pin; add is hash-derived)

Owed by advise (fable-5.1-arch-review, `reviews/2026-09-11-advise.md`):

- [x] S1: `/48` input `b"mjolnir/mesh/ula/v0"`; golden vector
- [x] S2: `br-mesh` only
- [x] S3: ULA re-added by reconcile loop
- [x] S4: `in`/`out ip fc00::/7 deny` in both babeld renders + tests
- [x] S5: iroh ULA candidates log at error in `check_reachability`
- [x] Spec: on-link SSH scenario
