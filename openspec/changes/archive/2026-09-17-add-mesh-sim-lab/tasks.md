# Tasks

Authoritative owed work is epic `mjolnir-mesh-sim` and `mjolnir-mesh-sim.1`.
These boxes are what **this architecture change** owes. Image, vwifi,
libvirt, install, NAT, and the roam harness are later beads.

- [x] Human activation (banner to ACTIVE BUILD) — Duke said `activate` 2026-09-17
- [x] Advise by an independent non-Grok reader; accept before act — r1 **send-back** (`reviews/2026-09-17-advise.md`, READER sol-arch-review); r2 **accept** (`reviews/2026-09-17-advise-r2.md`, READER sol-arch-review). Fable 5.1 infra-red (Claude OAuth expired).
- [x] S1 positive sim-guest identity (not Cudy deny-list); refuse unmarked metal including non-Cudy; checks before first wireless write (pinned 2026-09-17 r1)
- [x] S2 mgmt negative-control scenario: partition vwifi → babel/`10.254` die, SSH lives; overlay via mgmt is a failed run (pinned 2026-09-17 r1). Harness *test* is `sim.4`/`sim.7`, not this change.
- [x] S3 daily guest is PC/q35; `-M virt` only for armsr smoke (pinned 2026-09-17 r1)
- [x] `deploy/sim/README.md` lab contract: host, guests, nets, vwifi, isolation, evidence rule, how to start/stop (like `admin/scripts/windows-build/README.md`)
- [x] Amend `ARCHITECTURE.md` pointer to `mesh-sim-lab`

Not owed here (bullets, not boxes):

- `sim.2` x86 image / musl meshd
- `sim.3` libvirt XML / domains
- `sim.4` vwifi-server/client
- `sim.5` install-node sim profile
- `sim.6` NAT VMs
- `sim.7` roam keep-IP harness
- `sim.8` aarch64 smoke
- `sim.9` DNA-like STA script
- Closing `5wc` / `wvg` / `sz9.1`
- Fold (after act lands)
