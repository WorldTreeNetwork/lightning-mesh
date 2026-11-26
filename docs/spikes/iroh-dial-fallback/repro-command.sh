#!/bin/sh
# Re-check the iroh 1.0.0 dial-fallback predicates. Exit 0 when the
# source still matches the no-go finding. Does not dial, and does not
# edit the tree.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
cd "$ROOT"
OUT=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -type d -name 'iroh-1.0.0' | head -1)
if [ -z "$SRC" ]; then
  echo "iroh-1.0.0 source not in the cargo registry" >&2
  exit 2
fi
LOCK="$ROOT/Cargo.lock"
RS="$SRC/src/socket/remote_map/remote_state.rs"
PS="$SRC/src/socket/remote_map/remote_state/path_state.rs"
EP="$SRC/src/endpoint.rs"
for f in "$LOCK" "$RS" "$PS" "$EP"; do
  if [ ! -f "$f" ]; then
    echo "missing $f" >&2
    exit 2
  fi
done

/usr/bin/python3 - "$LOCK" "$RS" "$PS" "$EP" "$SRC" "$OUT/evidence.log" "$OUT/evidence.code" <<'PY'
import pathlib, sys
lock_p, rs_p, ps_p, ep_p, src, log_p, code_p = sys.argv[1:]
lock = pathlib.Path(lock_p).read_text()
rs = pathlib.Path(rs_p).read_text().splitlines()
ps = pathlib.Path(ps_p).read_text().splitlines()
ep = pathlib.Path(ep_p).read_text().splitlines()

def block(lines, start_substr, end_substr):
    start = next(i for i, line in enumerate(lines) if start_substr in line)
    end = next(i for i, line in enumerate(lines) if i > start and end_substr in line)
    return start, end, lines[start:end]

fails = []

needle = 'name = "iroh"\nversion = "1.0.0"\n'
if needle not in lock:
    fails.append("Cargo.lock has no iroh 1.0.0 package")
checksum = "6435544bb3a5c4e6ff7affaa0c0aa0d1bca45bd700226329d5059d3eb54f9dff"
if checksum not in lock:
    fails.append("Cargo.lock iroh checksum changed")

h_start, h_end, handle = block(rs, "fn handle_msg_resolve_remote(", "fn trigger_address_lookup(")
handle_text = "\n".join(handle)
for piece in (
    "self.paths.insert_multiple(addrs, Source::App);",
    "self.paths.resolve_remote(tx);",
    "self.trigger_address_lookup();",
):
    if piece not in handle_text:
        fails.append(f"resolve handler missing {piece}")
order = [handle_text.find(p) for p in (
    "insert_multiple(addrs, Source::App)",
    "resolve_remote(tx)",
    "trigger_address_lookup()",
)]
if order != sorted(order) or any(i < 0 for i in order):
    fails.append(f"resolve handler order is {order}, want insert, resolve, lookup")

t_start, t_end, trigger = block(rs, "fn trigger_address_lookup(", "fn handle_address_lookup_item(")
trigger_text = "\n".join(trigger)
gate = "if self.selected_path.is_some() || self.address_lookup_stream.is_some()"
if gate not in trigger_text:
    fails.append("lookup gate is not selected_path or an in-flight stream")
if "RelayUrl" in trigger_text or "relay_url" in trigger_text:
    fails.append("trigger_address_lookup now mentions a relay URL")

r_start, r_end, resolve = block(ps, "fn resolve_remote(", "fn resolve_requests_is_empty(")
resolve_text = "\n".join(resolve)
if "if !self.paths.is_empty()" not in resolve_text or "tx.send(Ok(())).ok();" not in resolve_text:
    fails.append("resolve_remote does not complete immediately when a path exists")

assigns = [i + 1 for i, line in enumerate(rs) if "selected_path" in line and ("=" in line or ".replace(" in line)]
# Reads also contain '='; keep only writes.
writes = []
for n in assigns:
    line = rs[n - 1]
    if "selected_path = " in line or "selected_path.replace(" in line:
        writes.append(f"{n}:{line.strip()}")
if not any("selected_path = None" in w for w in writes):
    fails.append(f"expected a clear of selected_path, saw {writes}")
if not any("selected_path.replace(" in w for w in writes):
    fails.append(f"expected select_path to replace selected_path, saw {writes}")

doc_start = next(i for i, line in enumerate(ep) if "pub async fn connect(" in line)
doc = "\n".join(ep[max(0, doc_start - 40):doc_start])
if "there is no [`RelayUrl`]" not in doc:
    fails.append("Endpoint::connect docs no longer say lookup is for a missing RelayUrl")

def numbered(lines, start, end):
    return "\n".join(f"{i+1}: {lines[i]}" for i in range(start, end))

code = []
code.append(f"# excerpts from {src}")
code.append(f"# Cargo.lock iroh checksum {checksum}")
code.append("")
code.append(f"## {rs_p} handle_msg_resolve_remote")
code.append(numbered(rs, h_start, h_end))
code.append("")
code.append(f"## {rs_p} trigger_address_lookup")
code.append(numbered(rs, t_start, t_end))
code.append("")
code.append("## selected_path writes")
code.extend(writes)
code.append("")
code.append(f"## {ps_p} resolve_remote")
code.append(numbered(ps, r_start, r_end))
code.append("")
code.append(f"## {ep_p} Endpoint::connect docs")
code.append(numbered(ep, doc_start - 20, doc_start))
pathlib.Path(code_p).write_text("\n".join(code) + "\n")

log = []
log.append(f"source={src}")
log.append("pin=iroh 1.0.0")
log.append(f"checksum={checksum}")
log.append("predicate=supplied addrs inserted, then resolve completes, then lookup is armed")
log.append("predicate=lookup gate is selected_path or an existing stream, not RelayUrl")
log.append("predicate=resolve_remote sends Ok when paths is non-empty")
log.append("selected_path writes:")
log.extend(writes)
if fails:
    log.append("RESULT=drift")
    log.extend(f"FAIL {item}" for item in fails)
    pathlib.Path(log_p).write_text("\n".join(log) + "\n")
    print("\n".join(log))
    sys.exit(1)
log.append("RESULT=predicates hold (no-go still matches this source)")
pathlib.Path(log_p).write_text("\n".join(log) + "\n")
print("\n".join(log))
PY
