# steer add-durable-network-apply

**When.** 2026-09-15
**Depth.** standard (decide-for-me after eyes pop; remaining forks lean auto-log)

## Decided

- **v1 mutation surface:** four UCI files only (`wireless`, `network`, `firewall`, `mjolnir`). Meshd binary replace and wpad swap stay on the unwrapped legacy launcher. (decide-for-me, then Fable send-back A/B 2026-09-15)
  Why: the shipped meshd binary is 10 MiB vs an 8 MiB snapshot cap; wpad swap is a package op whose rollback input is not in the durable tree.
- **Engine language:** Rust crate `crates/mjolnir-apply` (library + tests first; mandatory static musl helper on the OpenWrt adapter slice, outside the v1 mutation surface). (decide-for-me, then Fable send-back C)
  Why: instrument red-first and crash-at-transition tests cannot live in BusyBox ash. Legacy `usr/sbin/mjolnir-apply` stays as launcher. The helper is not optional: boot recovery must parse the journal.
- **On-disk:** JSON journal + snapshot tree under persistent `/etc/mjolnir/txn/` (tests: tempdir). Schema version field. Every durable write fsyncs the file and its parent directory. Not `/tmp`, not `/root/mjolnir-stage`. (decide-for-me)
- **Lock:** node-wide exclusive `flock` on `/etc/mjolnir/txn/lock`, released on process death; durable nonterminal journal is the fence. No mkdir lock. (decide-for-me)
- **Bounds:** default deadline 120s, per-plan timeout, v1 ceiling 600s; plan JSON ≤ 64 KiB; snapshot tree ≤ 8 MiB; nonterminal never GC; last 32 terminal receipts plus ID tombstones so an evicted ID cannot replay. (decide-for-me)
- **Live hardware / fleet deploy / radio reapply:** still separately gated. (auto)
  Why: activation inherited laptop-only; z3th/ri7v/hello fleet-roll stay named go-ahead.

## Skipped

- HTTPS aliases (`add-https-aliases` / `mjolnir-mesh-b6j.1`) — PENDING, not activated
- App manifest publish (`add-app-manifest-publish` / `mjolnir-mesh-ncy.2`) — PENDING, not activated
- Fold of household umbrellas — implementation owed; empty boxes ≠ fold-ready

## Feeds change

Pin the Rust crate, JSON journal under `/etc/mjolnir/txn`, flock+durable-fence lock, v1 mutation allowlist, and storage/timeout bounds in `design.md` so the next implementation-readiness advise can review exact pins rather than a worker guessing them. No SHALLs here. No OpenWrt runtime claimed by this steer.
