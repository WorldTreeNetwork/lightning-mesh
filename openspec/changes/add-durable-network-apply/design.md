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

