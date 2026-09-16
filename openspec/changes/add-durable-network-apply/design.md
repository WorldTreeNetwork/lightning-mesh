# ADR proposal: target-owned recoverable configuration transactions

## Evidence and scope

`deploy/openwrt/files/usr/sbin/mjolnir-apply` uses a staging-directory mkdir lock,
backs up four UCI files, performs disruptive writes, then writes one text result.
Its rollback reloads services but does not verify restoration. `setsid` in
`install-node.sh` protects against controller loss; neither that nor atomic rename
alone proves crash durability. `household-inventory.md` records the pilot and the
unsupported manual station profile. Preserve the legacy helper while introducing
the transaction path; never silently route production radio updates through an
untested wrapper.

## Plan and admission

The target snapshots its managed configuration plus explicit missing-file markers,
records a content revision, and builds a typed plan with schema version, transaction
ID, node ID, expected revision, affected resources, proposed revision, secret
references and required health checks. No shell snippets or sourced request env.
IDs are opaque bounded values, never paths. Paths/resources come from an allowlist.
Secrets stay in restricted storage and are excluded from public plans, hashes
exposed to clients, logs and receipts. The internal configuration digest can cover
secret bytes but must not become a public password-guessing oracle.

One node-wide lock covers all managed configuration writers, revision recheck,
snapshot and mutation; a per-stage lock is insufficient. Stale previews fail before
mutation. Same ID plus same plan returns the recorded status; same ID with different
plan is a conflict. An interrupted existing transaction blocks a new one until its
recovery is resolved. Out-of-band maintenance writes are detected by re-observation
and explicit conflict handling, not silently overwritten.

## Journal and transitions

States: prepared → applying → verifying → committed; failure or interruption after
prepared enters restoring → restored or recovery-required. Prepared means all
rollback inputs and plan metadata are durable *before* first mutation. Check every
write, snapshot copy, file fsync, rename and containing-directory fsync; disk errors
deny new mutation. Journal/schema corruption is recovery-required, not an empty
history. A typed local state-machine/storage component may land first, but it is
not the completed OpenWrt capability until boot/adapter integration is verified.

Use per-transaction root-owned storage on persistent overlay, not /tmp. Retain
nonterminal transactions and their snapshots; never garbage-collect them. Terminal
receipt retention is bounded without allowing replay of an evicted transaction ID:
retain an admission high-water/tombstone policy or refuse old IDs explicitly. Pin
the exact on-disk format, storage limits and implementation language in the act
packet; a terminal in-memory state is not sufficient evidence for a receipt.

Boot recovery runs before the new managed network configuration is treated as
accepted. For an uncommitted transaction, restore the durable snapshot (including
absence), re-apply old services and verify the old managed state plus its configured
reachability criteria. If restoration cannot be proved, retain artifacts and report
recovery-required. Never rerun arbitrary apply side effects on reboot. A committed
transaction is reconciled to its committed revision, not rolled back merely because
the controller missed the reply. Corrupt/missing commit evidence means uncertainty.

Deadline is target-owned and monotonic within one boot; reboot invalidates a pending
confirmation and takes the recovery path, not a fresh full timeout. Initial default
120s, bounded positive per-plan timeout with a 600s v1 ceiling, measured from the
durably recorded start. This is independent of owner-recovery and grant TTL policies.

## Evidence and receipts

Separate configuration state from observations: applied revision, management
reachability, mesh reachability, upstream association/address/route, upstream-bound
DNS/HTTPS, and downstream client forwarding each have passed/failed/unknown/not-
applicable plus vantage, target/interface and observation boot/time. An upstream
node ping or HTTP request never satisfies a downstream client check. A receipt may
say committed/local-only or committed/client-internet-unknown; it must not say the
house has internet without suitable client-path evidence. Required plan health
criteria determine commit; optional probes remain honestly unknown. Restoration
requires its own evidence, not reuse of pre-apply successes.

No distributed all-or-nothing claim: multi-node rollout is sequential with per-node
receipts and explicit partial completion. Keep a safe wired management path; changing
all mesh channels simultaneously is not a normal unqualified apply.

## Red-first proof and integration boundary

Tests must first fail for stale revision, concurrent apply, duplicate-ID conflict,
controller disappearance, crash at each journal transition, partial snapshot,
unwritable/full storage, corrupt journal, reboot during apply/restore, missing backup,
restoration failure and local-only false-positive internet evidence. Positive control:
valid plan commits once, duplicate request returns its receipt, interrupted apply
restores and verifies, retry after failed recovery stays blocked.

Filesystem/process fault injection is software evidence, not a physical power-loss
or flash-filesystem guarantee. Unit tests for a pure model do not close ai0.5. The
OpenWrt adapter, shared-lock participation, boot ordering and recovery probes must
land and pass focused integration tests before dependents claim durable apply.

## Review refinements and implementation admission

The independent review's residuals are recorded on ai0.5. Make the following
ordering explicit in the implementation packet:

- Recovery-first runs on service restart and before admitting a new apply, not
  only at boot. It obtains the node-wide process lock and inspects durable state;
  a dead PID or leftover directory alone is not proof that takeover is safe.
  The lock must be released by process death, and the durable transaction fences
  the next process from taking a partial configuration as its baseline.
- Health checks passing do not commit. Only a successfully persisted terminal
  commit does. A crash between health-pass and durable commit restores the old
  snapshot; losing a good apply is preferable to an ambiguous commit claim.
- Legacy participation is mandatory even while its UI/launcher is preserved.
  Snapshot coverage must be checked against every allowed side effect. The
  existing firmware/package/key/init-file updater cannot be wrapped and called
  recoverable on the strength of four UCI backups. Either include and verify its
  whole mutation set or reject those operations from the transaction path.

The engine language, on-disk encoding, storage/write bounds and concrete locking
mechanism remain explicit pin-before-code obligations, not permission for a worker
to guess them inside an unreviewed runtime patch. These refinements and those pins
must be included in the next implementation-readiness review. No human policy
question is reopened and no OpenWrt implementation is claimed by this amendment.

## Implementation pins (steer 2026-09-15, decide-for-me)

Recorded so the implementation-readiness advise can review exact pins. Not a
runtime claim. Not a living SHALL until folded.

- **Language.** New workspace crate `crates/mjolnir-apply`: library + unit/fault
  tests first (`CARGO_TARGET_DIR=/tmp/lm-target`). OpenWrt adapter is a later
  slice in this same change: optional static musl helper plus the existing
  BusyBox `usr/sbin/mjolnir-apply` as launcher. Do not rewrite the ash helper
  as the journal.
- **On-disk root.** Persistent overlay `/etc/mjolnir/txn/` (tests: a tempdir via
  env/`TxnPaths`). Never `/tmp`. Never `/root/mjolnir-stage` (legacy stage
  stays the launcher's download area). Layout:
  `lock`, `journal.json`, `receipts/<id>.json`, `tombstones.json`,
  `active/<id>/plan.json`, `active/<id>/snapshot/`.
- **Encoding.** UTF-8 JSON. Top-level `schema_version: 1`. Plan and journal are
  separate files. Snapshot copies allowlisted files and writes an explicit
  missing-file marker for absent sources. Every durable write: write temp, fsync
  file, rename, fsync parent directory. Disk error denies new mutation.
- **Lock.** One node-wide exclusive `flock` on `/etc/mjolnir/txn/lock`. The FD
  is released on process death. Recovery-first on start and before admission
  takes that lock, then inspects the durable journal. A leftover directory or
  dead PID is not itself proof that takeover is safe. Do not use `mkdir` locks.
- **v1 mutation allowlist.** `wireless`, `network`, `firewall`, `mjolnir` UCI;
  the meshd binary replace; the wpad-mesh swap already in `mjolnir-apply`.
  Firmware, package, key, and init-file updater operations are **rejected**
  from this transaction path (not wrapped as recoverable).
- **Bounds.** Default deadline 120s from durably recorded start; per-plan
  timeout must be positive and ≤ 600s. Plan JSON ≤ 64 KiB. Snapshot tree ≤
  8 MiB. Nonterminal transactions and their snapshots are never garbage-
  collected. Keep the last 32 terminal receipts; tombstone evicted IDs so a
  reused ID cannot admit a different plan.
- **Commit ordering.** Health-pass does not commit. Only a successfully
  persisted terminal journal state does. Crash between health-pass and that
  write restores the snapshot.
- **Evidence.** Laptop/fault-injection tests are software evidence. Hardware
  qualification remains `lpv` / `z3th`. Live radio or fleet deploy is a
  separate authorization.
