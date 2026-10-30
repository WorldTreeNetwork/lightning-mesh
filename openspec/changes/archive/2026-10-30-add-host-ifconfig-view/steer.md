# steer add-host-ifconfig-view

**When.** 2026-10-30
**Depth.** lean (inherited from activate-and-run)

## Decided

- Stdout: raw `/sbin/ifconfig` bytes, allowlisted argv (auto on parent).
- Timeout 5s, cap 65536 bytes (named here as the parent required).
- No poll / refresh control in this landing.

## Skipped

None.

## Feeds change

Fill the existing `canvas-well` from `Command::new("/sbin/ifconfig")`.
Keep discovery on `scan_link_local`.
