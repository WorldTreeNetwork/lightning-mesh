#!/usr/bin/env bash
# 0wgr.7 — mjolnir-apply snapshot, fail closed, restore. Never bd close.
# Durable txn path (txn-plan.json). Does not RUN_WIRELESS / wifi reload:
# sim client AP is dedicated hostapd (netifd would clobber it).
set -euo pipefail
. "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"
E2E_SUITE=apply-rollback

python3 -c 'import blake3' 2>/dev/null || e2e_fail "host python3 missing blake3 (pip install blake3)"

WORKDIR=$(mktemp -d)
STAGE_REMOTE=/root/mjolnir-stage
CONFIGS=(wireless network firewall mjolnir)
restored=0

revision_of() {
	python3 - "$1" <<'PY'
import pathlib, sys
import blake3

root = pathlib.Path(sys.argv[1])
h = blake3.blake3()
for name in ("wireless", "network", "firewall", "mjolnir"):
    h.update(name.encode())
    path = root / name
    if path.is_file():
        data = path.read_bytes()
        h.update(b"\0present\0")
        h.update(len(data).to_bytes(8, "little"))
        h.update(data)
    else:
        h.update(b"\0missing\0")
print(h.hexdigest())
PY
}

pull_uci() {
	local dest=$1
	mkdir -p "$dest"
	remote "$A" "tar -C /etc/config -cf - wireless network firewall mjolnir" | tar -C "$dest" -xf -
}

push_plan() {
	# Dropbear has no sftp.
	tar -C "$WORKDIR" -cf - txn-plan.json txn-config | remote "$A" "tar -C $STAGE_REMOTE -xf -"
}

restore_uci() {
	[ "$restored" = 1 ] && return 0
	restored=1
	if [ -d "$WORKDIR/orig" ]; then
		tar -C "$WORKDIR/orig" -cf - "${CONFIGS[@]}" | remote "$A" "tar -C /etc/config -xf -" || true
	fi
	remote "$A" "rm -rf $STAGE_REMOTE/txn-plan.json $STAGE_REMOTE/txn-config" || true
}

trap 'restore_uci; rm -rf "$WORKDIR"' EXIT

e2e_log start "apply-rollback"

remote "$A" "test -f /etc/mjolnir/sim-guest" || e2e_fail "missing sim-guest on $A"
oa=$(overlay_ip "$A" | tr -d '\r')
ob=$(overlay_ip "$B" | tr -d '\r')
[ -n "$oa" ] && [ -n "$ob" ] || e2e_fail "missing overlay a=$oa b=$ob"
remote "$A" "ping -c1 -W3 $ob >/dev/null" || e2e_fail "overlay already down"
e2e_log baseline "ping $oa -> $ob ok"

pull_uci "$WORKDIR/orig"
expected=$(revision_of "$WORKDIR/orig")
[ -n "$expected" ] || e2e_fail "empty expected revision"
cp -a "$WORKDIR/orig" "$WORKDIR/mut"
printf '\nconfig e2e '\''apply_rollback'\''\n\toption marker '\''1'\''\n' >>"$WORKDIR/mut/mjolnir"
mkdir -p "$WORKDIR/txn-config"
cp -a "$WORKDIR/mut/mjolnir" "$WORKDIR/txn-config/mjolnir"
# Deliberate apply-revision mismatch: files write, then fail closed, then restore.
# OpenWrtAdapter cannot prove mesh-reachability (Unknown) and then cannot
# verify_restoration either — that path is RecoveryRequired, which blocks
# later applies. Wrong proposed_revision uses ApplyFailed → Restored.
wrong="0000000000000000000000000000000000000000000000000000000000000000"
txid="e2e-apply-rollback-$(date +%s)"
python3 - "$WORKDIR/txn-plan.json" "$txid" "$expected" "$wrong" <<'PY'
import json, sys
path, txid, expected, proposed = sys.argv[1:5]
json.dump(
    {
        "schema_version": 1,
        "transaction_id": txid,
        "node_id": "sim-node-a",
        "expected_revision": expected,
        "proposed_revision": proposed,
        "resources": ["mjolnir"],
        "timeout_secs": 30,
        "required_health": ["applied-revision"],
    },
    open(path, "w"),
    indent=2,
)
print(path)
PY
e2e_log staged "txid=$txid expected=${expected:0:12} proposed=0"

push_plan
remote "$A" "rm -f $STAGE_REMOTE/result; /usr/sbin/mjolnir-apply" >/tmp/e2e-apply-rollback.out 2>&1 || true
result=$(remote "$A" "cat $STAGE_REMOTE/result 2>/dev/null" | tr -d '\r')
e2e_log result "${result:-empty}"
printf '%s\n' "$result" | grep -q 'ROLLED_BACK' || e2e_fail "result not ROLLED_BACK ('$result')"

pull_uci "$WORKDIR/after"
after=$(revision_of "$WORKDIR/after")
[ "$after" = "$expected" ] || e2e_fail "uci revision after restore '$after' != expected '$expected'"
grep -q "apply_rollback" "$WORKDIR/after/mjolnir" && e2e_fail "staged e2e section still in mjolnir UCI"

remote "$A" "ping -c1 -W3 $ob >/dev/null" || e2e_fail "overlay down after restore"
e2e_log overlay "ping $oa -> $ob ok"

restore_uci
e2e_log pass "apply-rollback"
echo "PASS apply-rollback"
