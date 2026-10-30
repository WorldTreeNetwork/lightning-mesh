# Design: host ifconfig fill

## Numbers

| Bound | Value | Why |
|---|---|---|
| Timeout | 5 seconds | `ifconfig` returns immediately; hang is a stuck child, not slow I/O |
| Cap | 65536 bytes | Typical `ifconfig` is a few KiB; cap is a runaway-output fence |
| Argv | `/sbin/ifconfig` only | Parent ADR; no extra flags, no shell |

## IPC

`fill_canvas_source(name: String) -> FillResult` (Tauri command). Name must
match the allowlist (`ifconfig`). Unknown name: no spawn, `status: "refused"`.

```
FillResult {
  source: String,
  status: "success" | "failure" | "truncated" | "unsupported" | "refused",
  exit_code: Option<i32>,
  text: String,          // combined stdout+stderr, lossy UTF-8
  truncated: bool,
}
```

Loading is the webview before the command returns. The canvas page owns the
well text; it does not interpret HTML.

## Lifetime

`CanvasState` holds at most one `Child`. A new fill `kill`s then `wait`s the
previous. `on_destroyed` for `canvas-*` does the same. Output from a reaped
child is dropped (generation counter).

## Tests

Unit tests: allowlist refuse; timeout path with a fixture command (not
`/sbin/ifconfig`); cap truncation with a fixture that writes past 65536;
cancel generation. Do not require a live Mac `ifconfig` for those cases.
A macOS-only test may spawn the real binary when present.
