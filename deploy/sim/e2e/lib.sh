#!/usr/bin/env bash
# Shared helpers for deploy/sim/e2e. Never call `bd close`.
set -euo pipefail
SIM_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
A="${SIM_A:-root@10.99.0.10}"
B="${SIM_B:-root@10.99.0.11}"
CPE="${SIM_CPE:-root@10.99.0.12}"
NAT="${SIM_NAT:-root@10.99.0.13}"
PHONE="${SIM_PHONE:-root@10.99.0.14}"
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=accept-new)

e2e_log() {
	printf '{"ts":"%s","suite":"%s","event":"%s","msg":%s}\n' \
		"$(date -Iseconds)" "${E2E_SUITE:-unknown}" "$1" "$(printf '%s' "$2" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')"
}

e2e_fail() {
	e2e_log fail "$1"
	echo "FAIL: $1" >&2
	exit 1
}

remote() { "${SSH[@]}" "$1" "$2"; }

overlay_ip() {
	remote "$1" "ip -4 -o addr show br-mesh 2>/dev/null | awk '{print \$4}' | cut -d/ -f1 | head -1"
}

mesh_if() {
	remote "$1" "iw dev | awk '/Interface/{d=\$2} /type mesh point/{print d; exit}'"
}
