# Implementation-readiness advisory — add-durable-network-apply pins

> **ADVISE:** send-back
> **READER:** fable-5.1-arch-review (assignee, stand-in) via mjolnir-mesh-ai0.5
> **SPAWN:** .spawns/mjolnir-mesh-ai0.5-1789521209-410761-c487ed32

Reader role, instrument rigor, packet `pkt-ai0-apply-pins-advise`. No code, no
fold, no deploy, no push, no live radio. Question under review: are the
2026-09-15 steer/design pins exact enough that a worker implements the
journal/lock/allowlist/bounds without inventing a second on-disk format or
wrapping a non-recoverable updater as recoverable. The prior architecture
advise (`reviews/2026-09-15-advise.md`) is accepted and not reopened.

Blind pass written from the proposal **Why**, `living-specs/spec.md`,
`mjolnir-apply:45-50` (mkdir lock) and `:194-210` (rollback), and
`install-node.sh:163-176` (setsid launcher) **before** opening design.md,
steer.md or tasks.md. Facts checked on disk afterwards: the built static
`deploy/openwrt/mjolnir-meshd-aarch64` is **10 MiB**; the init order is
meshd 95 / babeld 96 / hello 97 (OpenWrt `network` is 19).

## 1. Blind take (written before opening design.md / steer.md / tasks.md)

Ten things I would pin or refuse, from the Why, the living spec, `mjolnir-apply:45`
and `install-node.sh:163` only. These are concerns the author must have answered.

1. **Lock: pin `flock` on a fixed path, not `mkdir`.** The `mkdir $STAGE/.lock` +
   `trap EXIT rmdir` pair at `mjolnir-apply:46` orphans on SIGKILL/power loss and
   then refuses every future apply. A kernel-held `flock` on an open fd is released
   by the kernel on any death, including reboot. I would pin the path, the non-block
   semantics, and that the legacy `mjolnir-apply` must take the *same* flock. Refuse
   a pidfile lock (pid reuse after reboot) and refuse `mkdir` with an mtime heuristic.
   Open fact to pin: the fleet busybox must actually ship the `flock` applet.
2. **Journal: pin one on-disk shape and one writer.** Directory per transaction,
   state file replaced by tmp + fsync + rename + fsync(dir). Pin the filenames, the
   state enum, and the field list. Refuse a second format; in particular
   `$STAGE/result` must become a *derived* view written from the journal (or be
   dropped), so install-node.sh's poll and the receipt never disagree.
3. **Language: refuse JSON generated or parsed by ash.** Busybox ash has no JSON
   tooling; a hand-rolled writer/parser is the second format in disguise and cannot
   detect corruption. Pin a typed Rust component. The tradeoff is which binary hosts
   it: a subcommand of a binary that apply itself replaces (`mjolnir-meshd`) means
   recovery may run under a half-replaced tool. Prefer a separate small static binary
   or pin that the recovery binary is outside the v1 mutation surface.
4. **v1 allowlist: pin UCI config files only.** The legacy helper mutates packages
   (wpad swap), the meshd binary, init scripts, and keys. None of those are
   snapshot-restorable by file copy under a reboot (package db, running services).
   Pin the allowed set (`/etc/config/{wireless,network,dhcp,firewall,mjolnir}`
   or a stated subset) and refuse firmware (`sysupgrade`), package manager, key
   material (`/etc/mjolnir/secret`, dropbear keys), and init/updater scripts. The
   legacy helper remains a legacy path, never wrapped as "recoverable".
5. **Bounds: pin numbers and the refusal behavior.** Plan byte cap, snapshot byte
   cap, retained receipt count, tombstone retention. Each cap must *refuse before
   mutation*, never truncate; retention must never evict a nonterminal transaction.
6. **Health-pass does not commit.** Pin the ordering: gate passes → write `committed`
   record durably → only then any cleanup. Crash between gate pass and commit record
   is treated as uncommitted and restored. State this cost (a lost good apply) once.
7. **Recovery-first, twice.** (a) Every admission of a new transaction first
   resolves any nonterminal journal; a dead-but-not-rebooted applier must be handled
   by the next invocation, not by an orphaned lock. (b) Boot: pin *where* in the init
   order. Restoring `/etc/config/*` must happen before netifd reads them (START < 20),
   but verifying restoration needs meshd/babeld up (START 95/96). That is two boot
   phases, not one service; the pin must say so or a worker will guess.
8. **Rollback must verify restoration.** `rollback()` at `mjolnir-apply:194` copies
   files and writes `ROLLED_BACK` with no post-restore gate. Pin the restoration
   checks (own overlay address re-asserted, a baseline target answers) and that
   failure yields `recovery-required` with artifacts retained, never `restored`.
9. **No false client internet in either direction.** The existing gate proves
   overlay re-assertion and a neighbour ping (`mjolnir-apply:505-542`), which is mesh
   health only. Pin `not-applicable` for local-only policy vs `unknown` for
   unmeasured, and refuse any node-vantage probe counting as downstream client.
10. **Refuse: laptop tests are not OpenWrt completion.** A green fault-injection
    suite on ext4 says nothing about overlayfs-on-JFFS2/UBIFS rename+fsync or a real
    power cut. Hardware qualification stays `z3th`; ai0.5 is not closed by cargo test.

**One tradeoff:** durable journaling means fsync'd writes to the same flash overlay
that holds the config being mutated. Two or three fsyncs per transition on a
household router is cheap; per-receipt writes on every probe are not. Pin the
transition set that fsyncs, and note that `sysupgrade` wipes `/etc/mjolnir/txn`
unless kept, which is fine only because firmware is outside the allowlist.

---

## 2. Comparison — steelman against the blind take

The pins (design.md *Implementation pins* + steer.md *Decided*) are read as
concerns the author had to satisfy. Most are answered exactly.

| # | Blind concern | Pin | Verdict |
|---|---|---|---|
| 1 | `flock`, not `mkdir`; kernel-released; legacy takes the same lock | "One node-wide exclusive `flock` on `/etc/mjolnir/txn/lock` … released on process death … Do not use `mkdir` locks"; refinements: "Legacy participation is mandatory" | **Answered.** Lock liveness is solved by construction. Residual: *how* the ash launcher takes the flock is not written (see note N3) and the busybox `flock` applet is assumed, not recorded. |
| 2 | One on-disk shape, one writer, `$STAGE/result` derived | Layout `lock`, `journal.json`, `receipts/<id>.json`, `tombstones.json`, `active/<id>/{plan.json,snapshot/}`; `schema_version: 1`; temp+fsync+rename+fsync(dir) | **Answered for the journal.** `$STAGE/result` is not mentioned; install-node.sh still polls it. Note N2. |
| 3 | Refuse ash JSON; typed Rust; where the recovery binary lives | "New workspace crate `crates/mjolnir-apply` … Do not rewrite the ash helper as the journal"; adapter is "optional static musl helper plus the existing BusyBox … as launcher" | **Answered on language. Unanswered on the executor**: if the helper is *optional*, nothing on the node can read the JSON journal at boot. Send-back item C. |
| 4 | Allowlist = UCI only; refuse firmware/package/key/init; legacy stays legacy | "`wireless`, `network`, `firewall`, `mjolnir` UCI; the meshd binary replace; the wpad-mesh swap … Firmware, package, key, and init-file updater operations are rejected" | **Contradiction.** The wpad swap *is* a package-manager operation (`pm_remove` + `pkg_ensure`, `mjolnir-apply:251-258`). Send-back item B. |
| 5 | Bounds with refuse-before-mutation, nonterminal never evicted | 64 KiB plan, 8 MiB snapshot, 32 receipts, tombstones, nonterminal never GC | **Answered as numbers. Contradicts #4**: the meshd binary is 10 MiB, so any binary-replace transaction cannot snapshot its rollback input under an 8 MiB cap. Send-back item A. Tombstone growth is unbounded (note N4). |
| 6 | Health-pass does not commit; crash window restores | "Health-pass does not commit. Only a successfully persisted terminal journal state does. Crash between … restores the snapshot" | **Answered exactly.** |
| 7a | Recovery-first on process start and before admission | "Recovery-first on start and before admission takes that lock, then inspects the durable journal. A leftover directory or dead PID is not itself proof" | **Answered exactly.** |
| 7b | Boot: where in init order, and the two-phase problem | tasks.md "boot/service recovery-first"; design "Boot recovery runs before the new managed network configuration is treated as accepted" | **Missing pin.** No START number, no statement that file restore must precede netifd (19) while verification must follow meshd/babeld (95/96). Send-back item C. |
| 8 | Rollback verifies restoration or reports recovery-required | design: "verify the old managed state plus its configured reachability criteria … recovery-required"; delta *Failed restoration* | **Answered.** |
| 9 | `not-applicable` vs `unknown`; no node-vantage probe counts as client | design *Evidence and receipts*: six observations × passed/failed/unknown/not-applicable + vantage; "committed/local-only or committed/client-internet-unknown" | **Answered exactly.** |
| 10 | Laptop ≠ OpenWrt completion | "Laptop/fault-injection tests are software evidence. Hardware qualification remains lpv / z3th" | **Answered exactly.** |
| tradeoff | Which transitions fsync; sysupgrade wipes `/etc/mjolnir/txn` | "Every durable write: write temp, fsync file, rename, fsync parent directory" | **Answered** (every transition fsyncs; acceptable at this write rate). Sysupgrade is moot because firmware is rejected. |
| — | Deadline clock on an RTC-less router | "Default deadline 120s from durably recorded start … monotonic within one boot; reboot invalidates a pending confirmation" | **Missing pin** on *which clock* and *how reboot is detected*. Send-back item D. |

## 3. Send-back items (exact pins still owed)

Each is a one-line pin fix in design.md *Implementation pins*. Proposed wording
is offered so the re-steer is fast; the author decides.

**A. Snapshot cap vs meshd binary (contradiction).** The v1 allowlist includes
"the meshd binary replace"; the rollback input for that op is the current
`/usr/bin/mjolnir-meshd`, which is 10 MiB on disk today. The snapshot tree cap
is 8 MiB. As written, every transaction that replaces the binary is refused at
prepare, or a worker silently exempts the binary from the cap, or raises the
cap. Pin one: either **(i)** drop the binary replace from v1 (UCI-only v1, binary
stays on the legacy launcher path), or **(ii)** cap at ≥ 2× the shipped binary
(e.g. 32 MiB) and state that the cap is checked against the snapshot *before*
first mutation. Recommendation: (i). It also removes the case where the
recovery executor's own binary is inside the mutation set.

**B. wpad swap is a package operation (contradicts the rejection and crash
durability).** The same bullet allowlists "the wpad-mesh swap already in
`mjolnir-apply`" and rejects "package … updater operations". The swap is
`pm_remove wpad-basic-mbedtls` + `pkg_ensure wpad-mesh-mbedtls`
(`mjolnir-apply:251-258`); its only rollback input is the previous `.apk`/`.ipk`
in `/root/mjolnir-stage/pkgs`, which the pins exclude from the durable tree.
After a reboot mid-swap, boot recovery cannot restore from `active/<id>/snapshot/`
because the package file was never a durable rollback input, and the package
database is not a file copy. This breaks "Prepared means all rollback inputs …
durable before first mutation". Pin: **reject the wpad swap from the transaction
path in v1** (legacy launcher keeps doing it, unwrapped), and make the packet's
own judgment explicit in the pin: *the v1 allowlist is the four UCI files*.

**C. Boot recovery executor and ordering (missing pin, contradicts
recovery-first).** The adapter slice says the static musl helper is *optional*
and the ash helper must not be the journal. If the helper is optional, no
program on the node can parse `journal.json` at boot, so boot recovery cannot
run. Pin: **the static helper is mandatory for the OpenWrt adapter slice**, it is
outside the v1 mutation surface, and it ships as an init script with two
phases: **restore** (copy `active/<id>/snapshot/` back to `/etc/config/*`,
journal → `restoring`) at `START` < 19 so netifd boots on the restored files;
**verify** (the plan's required reachability checks) at `START` > 96, writing
`restored` or `recovery-required`. Until verify has written a terminal state,
new admission stays blocked. If the author prefers a single late service that
restores files and then `wifi reload` + service restarts, pin that instead; either
is implementable, the current text is neither.

**D. Deadline clock (missing pin).** These routers have no RTC; the wall clock at
boot is stale until NTP steps it, possibly during the apply. "120s from the
durably recorded start" measured on wall time can expire instantly or never.
Pin: **record `/proc/sys/kernel/random/boot_id` and `CLOCK_MONOTONIC` (or
`/proc/uptime`) at `prepared`**; the deadline compares monotonic time within the
same boot_id; a different boot_id on recovery is the reboot path. Wall time is
recorded for receipts only, never for control.

## 4. Notes (body, not verdict)

- **N1. `flock` applet.** The pins assume BusyBox `flock` exists on the fleet
  image (OpenWrt's default busybox config enables it). Record `busybox | grep -w
  flock` output from one node in the adapter packet before relying on it from
  ash; the Rust side uses the syscall and does not depend on it.
- **N2. `$STAGE/result` must be a derived view.** install-node.sh polls it and
  treats `OK*` as success. Pin that on the adapter slice the launcher writes
  `result` only from the receipt, after the terminal journal fsync, so the
  two can never disagree. Otherwise it is the second on-disk format the packet
  warns about.
- **N3. Legacy lock participation mechanics.** Pin the one-liner: launcher does
  `exec 9>/etc/mjolnir/txn/lock; flock -n 9 || exit`, then execs the helper with
  fd 9 inherited, or the helper takes the lock itself and the launcher never
  mutates outside it. Either is fine; say which.
- **N4. Tombstones are unbounded** while receipts are capped at 32. With opaque
  bounded IDs this is bytes per transaction, acceptable, but say so
  ("tombstones are never evicted") or replace with a monotonic admission
  counter. Also pin cardinality: `active/` holds at most one transaction and
  `journal.json` names it, otherwise "journal" vs "active/<id>" is ambiguous.
- **N5. What is exact and should not be reopened:** flock over mkdir, JSON
  under `/etc/mjolnir/txn`, Rust crate over ash journal, temp+fsync+rename+
  fsync(dir) on every transition, 64 KiB plan cap, 32-receipt retention,
  nonterminal never GC, health-pass-does-not-commit, recovery-first before
  admission, laptop tests as software evidence only. A worker can implement
  all of these from the text as written.

## 5. Acceptance criterion (contrast) — checked

- **if_true does not yet hold.** A worker implementing the allowlist as written
  either wraps a package operation (wpad) as recoverable with its rollback input
  outside the durable tree, or must guess whether to drop it. A worker
  implementing the bounds as written either refuses every binary replace or
  guesses an exemption. A worker implementing boot recovery has no executor
  pinned and no init position.
- **if_false is reached** on three of its clauses: item B contradicts crash
  durability (undurable rollback input), item C leaves recovery-first
  unimplementable at boot, item D lets a wall-clock deadline expire a live
  transaction falsely. No false client-internet path was found; §9 is exact.

## 6. Verdict rationale

Send-back is a verdict on pin exactness, not on the architecture, which stands.
Items A and B are one decision (v1 = four UCI files, everything else stays on
the unwrapped legacy launcher); items C and D are one paragraph each. With those
four lines re-pinned this reader would accept without a further blind pass.
Nothing here claims OpenWrt runtime behavior; hardware remains `lpv`/`z3th`.
