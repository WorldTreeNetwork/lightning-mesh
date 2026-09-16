# Advise pins r2 — add-durable-network-apply

> **ADVISE:** accept
> **READER:** fable-5.1-arch-review (assignee, stand-in) via mjolnir-mesh-ai0.5
> **SPAWN:** .spawns/mjolnir-mesh-ai0.5-1789521651-415806-12bdbbaa

## Blind take (written before opening design.md / tasks.md / prior review)

Inputs: proposal Why, living specs (wan-lan-admin, mjolnir-settings, living-specs),
`deploy/openwrt/files/usr/sbin/mjolnir-apply` (mkdir lock at :46, `date +%s`
deadline at :516, `UCI_CONFIGS` at :35), launchers in `install-node.sh:163/173`,
init START numbers (meshd 95, babeld 96, hello 97).

1. Pin: the reboot-recoverable surface is UCI text only — the four files already named at `mjolnir-apply:35`; binaries and package swaps (wpad, meshd) are not restorable from a boot helper and must stay in the legacy path with its unverified rollback, explicitly labelled as such.
2. Pin: one journal directory per transaction on persistent storage; every state write is tmp+fsync+rename; "active" means exactly one entry under `active/`; the result is derived from the journal, never a second hand-written result file that can disagree with it.
3. Pin: lock is an `flock` on a well-known fd owned by the launcher and inherited by the helper, so a dead helper or a reboot releases it — the `mkdir` lock at :46 leaks across a crash and refuses every later apply until a human `rmdir`s it.
4. Pin: deadline is never wall-clock — routers have no RTC and `date +%s` (:516) jumps when NTP lands mid-apply; boot_id + CLOCK_MONOTONIC detects a reboot by identity, not by elapsed time.
5. Pin: two boot phases — restore before `network` (START < 19) when an active uncommitted txn exists; verify after meshd/babeld (> 96) so the health baseline is meaningful; both phases read the same journal and neither re-implements the legacy helper.
6. Refuse: any mutation of `/etc/config` before the snapshot is complete and durable; a snapshot too large for the cap, or unwritable flash, refuses the whole apply with nothing changed.
7. Refuse: a second concurrent transaction, and a boot helper that depends on anything not present at START < 19 (network, NTP, procd-managed services); the helper must be a static binary that needs only rootfs.
8. Tradeoff: restoring on boot before network means a reboot mid-apply always loses the change, even one that would have succeeded — accept it; the receipt says restored and the controller resubmits against the new revision.
9. Concern: a reconnecting controller retrieving by ID must get a definitive answer after the txn is gone from `active/`; tombstones on flash must therefore be permanent, and the author must say how that squares with bounded storage.
10. Concern: `install-node.sh:163/173` still polls `$STAGE/result`; the pin must say whether the legacy launcher keeps writing that file or the poller moves to the derived result, so no worker maintains two formats.

---

Reader role, instrument rigor, packet `pkt-ai0-apply-pins-advise-r2`. No code,
no fold, no deploy, no push, no live radio. Question under review: after the
send-back in `reviews/2026-09-15-advise-pins.md` (items A–D), are the
`design.md` *Implementation pins* now exact enough that a worker implements
journal / lock / UCI-only allowlist / boot helper / monotonic deadline without
guessing a second format or wrapping wpad/meshd as recoverable. The accepted
architecture advise (`reviews/2026-09-15-advise.md`) is not reopened.

Facts checked on disk after the blind take: `deploy/openwrt/mjolnir-meshd-aarch64`
is 10,069,584 bytes (10 MiB, matches the pin's stated reason); init order is
meshd 95 / babeld 96 / hello 97; `install-node.sh:163/173` still launch the
legacy helper with `setsid` and poll `$STAGE/result`.

## Comparison — send-back items A–D against the re-pin

| Item | Owed pin (r1) | Re-pin text (design.md *Implementation pins*) | Verdict |
|---|---|---|---|
| **A** snapshot cap vs 10 MiB meshd binary | drop binary replace from v1, or raise cap; cap checked before first mutation | "The four UCI files only … Meshd binary replace … stay on the **unwrapped legacy launcher** … 8 MiB … checked against the snapshot **before** first mutation; exceeding refuses, never truncates" | **Closed.** Option (i) taken. Cap and surface no longer contradict; the recovery executor's own binary is out of the mutation set by the same stroke. |
| **B** wpad swap is a package op | reject wpad swap from the transaction path; v1 = four UCI files | "the wpad-mesh package swap stay[s] on the unwrapped legacy launcher. Firmware, package manager, key material, and init/updater scripts are rejected … Snapshot rollback inputs are those four files (plus explicit missing-file markers)" | **Closed.** The allowlist and the rejection list no longer overlap. "Prepared means all rollback inputs durable before first mutation" is now satisfiable by file copy alone. |
| **C** boot recovery executor + init ordering | helper mandatory, outside mutation surface, two phases with START numbers, admission blocked until terminal | "static musl helper is **mandatory**, outside the v1 mutation surface … **restore** at `START` < 19 … journal → `restoring` … **verify** at `START` > 96 … `restored` or `recovery-required`. Until verify writes a terminal state, new admission stays blocked" | **Closed.** Exactly the two-phase shape. `steer.md:10` still says "optional static musl helper later" (note N1, stale log line, not a competing pin). |
| **D** deadline clock | boot_id + CLOCK_MONOTONIC at `prepared`; different boot_id = reboot path; wall time receipts-only | "At `prepared`, record `/proc/sys/kernel/random/boot_id` and `CLOCK_MONOTONIC` (or `/proc/uptime`) … A different `boot_id` on recovery is the reboot path, not a wall-clock expiry. Wall time is receipts-only" | **Closed.** Note N3 on the parenthetical. |

r1 body notes N2–N4 were also taken up as pins: derived `$STAGE/result` written
only from the receipt after the terminal journal fsync; launcher `exec 9>lock;
flock -n 9 || exit` with fd 9 inherited; tombstones never evicted; `active/`
holds at most one transaction and `journal.json` names it.

## Comparison — this pass's blind take against the pins

| # | Blind concern | Answered by | Verdict |
|---|---|---|---|
| 1 | UCI-only recoverable surface; binaries/packages stay legacy, labelled | v1 allowlist bullet | exact |
| 2 | one journal dir per txn; tmp+fsync+rename; `active/` = one; result derived | On-disk root, Encoding, Bounds, Derived result | exact |
| 3 | flock held by launcher, inherited, kernel-released | Lock, Legacy lock participation | exact on liveness; N2 on helper-side semantics |
| 4 | boot_id + monotonic, never wall time | Deadline clock | exact |
| 5 | two boot phases <19 / >96 on the same journal | Boot recovery executor | exact |
| 6 | refuse before any mutation on cap or disk error | Bounds, Encoding ("Disk error denies new mutation") | exact |
| 7 | refuse second txn; helper needs only rootfs at START<19 | Lock + "active/ at most one"; helper is static musl and restore phase is file copy only | exact |
| 8 | reboot mid-apply loses a good apply; accept | Commit ordering | stated, accepted |
| 9 | permanent tombstones vs bounded flash | Bounds: "Tombstones are never evicted (bytes per opaque ID; acceptable)" | answered, cost named |
| 10 | who writes `$STAGE/result` | Derived result file | exact |

No concern in the take is unanswered. Nothing in the re-pin contradicts the
accepted architecture (health-pass does not commit, recovery-first, legacy
participation, honest connectivity vantage).

## Notes (body, not verdict)

- **N1. `steer.md:10` is stale.** It still reads "optional static musl helper
  later" while `design.md:124` and `:161` pin **mandatory**. `steer.md` is the
  decision log and its own *Feeds change* section names `design.md` as the pin
  surface, so a worker reading the pins does not guess. Fix the word in the log
  when convenient (restore-only edit, no ceremony per living-specs). Not
  editable from this packet's path allowlist.
- **N2. Helper-side flock with an inherited fd.** Two pins interact: "the ash
  launcher does `exec 9>/etc/mjolnir/txn/lock; flock -n 9 || exit` and execs the
  helper with fd 9 inherited" and "Recovery-first on start and before admission
  takes that lock". `flock(2)` locks belong to the open file description. The
  inherited fd 9 shares the launcher's description, so `flock(9, LOCK_EX|LOCK_NB)`
  inside the helper succeeds as a no-op conversion. A fresh `open()` of the lock
  path followed by `LOCK_NB` would create a second description and be refused
  by the launcher's own lock. One-line worker rule for the adapter packet: *if
  launched with the lock fd inherited (pin the convention, e.g. fd 9 or an
  env var naming it), flock that fd; otherwise (init phases, direct
  invocation) open the path and flock it.* This is implementation detail under
  an exact pin, not a missing pin; it belongs in the adapter slice's own
  packet, not in a third design round.
- **N3. `CLOCK_MONOTONIC` "(or `/proc/uptime`)".** `/proc/uptime` is
  `CLOCK_BOOTTIME`; on a router that never suspends the two are equal. The
  only worker rule is: record and compare with the same source, both inside the
  Rust helper. The parenthetical does not create a second format because the
  ash side never reads the journal.
- **N4. mkdir lock at `mjolnir-apply:46`.** The anchor invariant holds: the
  legacy helper keeps its `mkdir` single-instance guard and unverified
  `rollback()` for the operations that stay on the legacy path. The pin "Do not
  use `mkdir` locks" governs the transaction path; the launcher's `exec 9>`
  flock wraps the legacy run so legacy participation is mandatory. The mkdir
  guard becomes redundant but harmless once the launcher holds the flock.
- **N5. Busybox `flock` fd form.** The pin already requires the adapter packet
  to record `busybox | grep -w flock` from a fleet node before ash relies on
  it. This laptop has no busybox to verify; the applet in OpenWrt's default
  config accepts the `flock [-sxun] FD` form the pin uses. Still a record-it
  obligation, not an assumption.

## Acceptance criterion (contrast) — checked

- **if_true holds.** A worker implementing from the pins has one on-disk shape
  (`/etc/mjolnir/txn/` layout, `schema_version: 1`, tmp+fsync+rename+fsync(dir)),
  one lock (flock on a pinned path, launcher-held, inherited), one allowlist
  (four UCI files; every non-file operation named as legacy or rejected), one
  boot executor (mandatory static helper, restore <19, verify >96), and one
  deadline clock (boot_id + monotonic). `$STAGE/result` is pinned as derived.
  Neither wpad nor the meshd binary can be wrapped as recoverable under the
  text.
- **if_false is not reached.** None of A–D is missing, and none is newly
  contradicted by another pin. N1 is a stale log sentence in a non-pin file;
  N2 is a semantics clarification under an already-exact pin.

## Verdict rationale

Accept. Items A and B were one decision and it was taken the safe way (UCI-only
v1). Items C and D are pinned with START numbers and clock sources, not
adjectives. The notes are implementation-packet material for the adapter
slice; none reopens architecture and none requires a third pins round.
Nothing here claims OpenWrt runtime behaviour; hardware remains `lpv` / `z3th`,
and laptop fault-injection stays software evidence only.
