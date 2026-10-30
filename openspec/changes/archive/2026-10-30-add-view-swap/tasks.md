# Tasks

- [x] Allowlist `netstat` → `["/usr/sbin/netstat", "-ib"]`; no shell
- [x] Canvas lists allowlisted names; default open remains `ifconfig`
- [x] Choosing a name cancels the in-flight child and fills that source
- [x] Unknown name refused; non-macOS unsupported with no spawn
- [x] Tests: `netstat` argv, unknown refusal, stale generation dropped on swap
- [x] Focused verify: `CARGO_TARGET_DIR=/tmp/lm-admin-target cargo test --manifest-path admin/src-tauri/Cargo.toml --lib`

44 passed. One baseline failure in unchanged
`net::tests::live_scan_returns_interfaces` (host reports no non-loopback
link-local IPv6 address). All stdout tests passed, including real macOS
`ifconfig` and `netstat -ib`. Non-macOS branches are cfg-gated and did not
run on this host.

Do not add a refresh control. Do not parse output. Do not open a second canvas.
Do not touch `scan_link_local`.
