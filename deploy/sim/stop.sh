#!/usr/bin/env bash
# Graceful stop sim-node-a / sim-node-b. Nets left up (like leaving default).
set -euo pipefail
URI="${VIRSH_URI:-qemu:///system}"
V() { virsh -c "${URI}" "$@"; }
for n in sim-node-a sim-node-b sim-isp-cpe sim-house-nat sim-phone; do
	V shutdown "${n}" >/dev/null 2>&1 || true
done
echo ">> shutdown requested for sim-node-a sim-node-b"
