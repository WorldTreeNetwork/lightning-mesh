# OpenWrt durable-apply adapter notes

Implemented for `mjolnir-mesh-ai0.5`; this is software-only evidence and does
not claim fleet deployment, radio testing, reboot testing, or hardware
qualification.

## Boundary

- `mjolnir-txn` is a binary from the `mjolnir-apply` crate. Its apply adapter
  can replace or remove only `/etc/config/{wireless,network,firewall,mjolnir}`.
- Desired files are supplied in `txn-config/`; an absent desired file is
  represented explicitly as `<name>.missing`. Package operations, wpad swaps,
  daemon binaries, keys, init files, and updater files remain outside this
  transaction.
- The ash launcher and both init phases open and non-blockingly lock fd 9. The
  helper validates and flocks that inherited open file description and never
  reopens `/etc/mjolnir/txn/lock`.
- Evidence supplied by the packet: on 2026-09-16, m3000-b had
  `/usr/bin/flock -> busybox` and `flock -n 9` succeeded. This was not rechecked
  against hardware in this node.

## Recovery phases

- `/etc/init.d/mjolnir-txn-restore` uses `START=18`, before netifd at 19. It
  journals `restoring`, restores the snapshot, and deliberately leaves the
  transaction nonterminal.
- `/etc/init.d/mjolnir-txn-verify` uses `START=97`, after meshd/babeld at 95/96.
  It records `restored` only after the adapter verifies the previous revision
  and every required probe it can honestly establish. Unsupported reachability
  requirements fail closed as `recovery-required`.
- A late verifier never performs a missed early restore after networking has
  started. Missing the early phase is recorded as recovery-required.

## Launcher and artifact

`install-node.sh` now requires and stages `deploy/openwrt/mjolnir-txn-aarch64`,
then the legacy launcher installs the helper and enables both init scripts. A
static artifact can be produced with the repository's aarch64 musl container by
building package `mjolnir-apply`, bin `mjolnir-txn`, for target
`aarch64-unknown-linux-musl` and copying the release binary to that path.

The launcher always runs helper recovery before new work. When
`/root/mjolnir-stage/txn-plan.json` is present it enters the UCI-only durable
path and execs the helper with `txn-config/`. The helper writes the compatibility
`$STAGE/result` only by mapping a persisted terminal receipt; it does not write
that result directly from health-check control flow. The existing package and
binary installer remains a separate legacy path and is not represented as a
recoverable UCI transaction.
