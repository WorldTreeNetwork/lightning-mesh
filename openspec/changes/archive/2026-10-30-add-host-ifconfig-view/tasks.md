# Tasks

- [x] Allowlist `ifconfig` → `["/sbin/ifconfig"]`; no shell; stdin null
- [x] Timeout 5s, cap 65536 bytes; combined stdout+stderr; visible truncation
- [x] One child; cancel+reap on new fill and on canvas destroy; drop stale output
- [x] Well states: loading, success (incl. empty), failure, truncated, unsupported
- [x] Open canvas fills ifconfig by default on macOS
- [x] Tests: refuse unknown source; timeout; cap; cancel generation
- [x] Focused verify: `CARGO_TARGET_DIR=/tmp/lm-admin-target cargo test --manifest-path admin/src-tauri/Cargo.toml --lib`

Verification: 43 passed, 1 baseline failure in unchanged
`net::tests::live_scan_returns_interfaces` (host reports no non-loopback
link-local IPv6 address). All nine stdout tests passed, including real macOS
`/sbin/ifconfig`. Frontend script checks passed for result states, IPC failure,
literal transcript rendering, and closing before a pending result arrives.

Do not add a refresh control. Do not parse NICs. Do not touch `scan_link_local`.
