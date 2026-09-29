#!/usr/bin/env bash
# 0wgr.3 — wrap roam-keep-ip.sh. Dual proto 158 is FAIL. Never bd close.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
. "$HERE/lib.sh"
E2E_SUITE=keep-ip
e2e_log start "keep-ip (dual proto 158 is FAIL, not xfail; will not bd close 5wc/wvg/sz9.1)"
set +e
"$HERE/../roam-keep-ip.sh"
rc=$?
set -e
if [ "$rc" -eq 0 ]; then
	e2e_log pass "keep-ip"
	echo "PASS keep-ip"
	exit 0
fi
e2e_log fail "keep-ip exit=$rc (expected while A still has proto 158)"
echo "FAIL keep-ip (exit $rc)" >&2
exit "$rc"
