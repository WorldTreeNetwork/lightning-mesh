#!/usr/bin/env bash
# Install mjolnir on a sim guest. Uses x86_64 musl artifacts and the
# marker-guarding wireless profile. Does not replace fleet aarch64 binaries.
set -euo pipefail
SIM="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENWRT="$(cd "$SIM/../openwrt" && pwd)"
export MJOLNIR_MESHD_BIN="${MJOLNIR_MESHD_BIN:-$SIM/mjolnir-meshd-x86_64}"
export MJOLNIR_TXN_BIN="${MJOLNIR_TXN_BIN:-$SIM/mjolnir-txn-x86_64}"
export MJOLNIR_HELLO_BIN="${MJOLNIR_HELLO_BIN:-$SIM/mjolnir-hello-x86_64}"
export SETUP_WIRELESS="${SETUP_WIRELESS:-$SIM/setup-wireless.sh}"
WIRELESS="${SIM}/wireless.env"
[ -f "$MJOLNIR_MESHD_BIN" ] || { echo "missing $MJOLNIR_MESHD_BIN — deploy/sim/build-meshd.sh"; exit 1; }
[ -f "$MJOLNIR_TXN_BIN" ] || { echo "missing $MJOLNIR_TXN_BIN — deploy/sim/build-meshd.sh mjolnir-txn (from mjolnir-apply)"; exit 1; }
exec "$OPENWRT/install-node.sh" --wireless "$WIRELESS" "$@"
