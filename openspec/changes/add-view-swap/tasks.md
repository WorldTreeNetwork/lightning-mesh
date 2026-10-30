# Tasks

- [ ] Allowlist `netstat` → `["/usr/sbin/netstat", "-ib"]`; no shell
- [ ] Canvas lists allowlisted names; default open remains `ifconfig`
- [ ] Choosing a name cancels the in-flight child and fills that source
- [ ] Unknown name refused; non-macOS unsupported with no spawn
- [ ] Tests: `netstat` argv, unknown refusal, stale generation dropped on swap
- [ ] Focused verify: `CARGO_TARGET_DIR=/tmp/lm-admin-target cargo test --manifest-path admin/src-tauri/Cargo.toml --lib`

Do not add a refresh control. Do not parse output. Do not open a second canvas.
Do not touch `scan_link_local`.
