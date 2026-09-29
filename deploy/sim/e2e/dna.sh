#!/usr/bin/env bash
# 0wgr.4 — wrap dna-probe.sh. Does not close sz9.1.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$HERE/lib.sh"
E2E_SUITE=dna
e2e_log start "dna (RFC 4436 INIT-REBOOT; will not bd close sz9.1)"
set +e
"$HERE/../dna-probe.sh"
rc=$?
set -e
[ "$rc" -eq 0 ] || e2e_fail "dna-probe.sh exit $rc"
e2e_log pass "dna"
echo "PASS dna (observation recorded; sz9.1 left open)"
