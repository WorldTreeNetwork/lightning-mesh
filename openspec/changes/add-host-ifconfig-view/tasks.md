# Tasks

- [ ] Allowlist `ifconfig` → `["/sbin/ifconfig"]`; no shell; stdin null
- [ ] Timeout 5s, cap 65536 bytes; combined stdout+stderr; visible truncation
- [ ] One child; cancel+reap on new fill and on canvas destroy; drop stale output
- [ ] Well states: loading, success (incl. empty), failure, truncated, unsupported
- [ ] Open canvas fills ifconfig by default on macOS
- [ ] Tests: refuse unknown source; timeout; cap; cancel generation
- [ ] Focused verify: `CARGO_TARGET_DIR=/tmp/lm-admin-target cargo test --manifest-path admin/src-tauri/Cargo.toml --lib`

Do not add a refresh control. Do not parse NICs. Do not touch `scan_link_local`.
