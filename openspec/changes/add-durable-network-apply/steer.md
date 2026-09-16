# steer add-durable-network-apply

**When.** 2026-09-15
**Depth.** standard (decide-for-me after eyes pop; remaining forks lean auto-log)

## Decided

- **v1 mutation surface:** current `mjolnir-apply` set only — UCI `wireless network firewall mjolnir`, meshd binary, wpad-mesh swap. (decide-for-me)
  Why: the firmware/package/key/init-file updater cannot be called recoverable on four UCI backups. Reject those ops from this transaction path until a later slice covers them.
- **Engine language:** Rust crate `crates/mjolnir-apply` (library + tests first; optional static musl helper later). (decide-for-me)
  Why: instrument red-first and crash-at-transition tests cannot live in BusyBox ash. Legacy `usr/sbin/mjolnir-apply` stays as launcher until the adapter lands.
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
