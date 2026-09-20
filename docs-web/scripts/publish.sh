#!/usr/bin/env bash
# Build docs-web and publish the static tree to IdentiKey Sites.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FP="${IDENTIKEY_FP:-7PfGe1Dsx176UjgvTSaQd9Bc6hXBiCaJrNz3Tcd2noWL}"
KEY="${IDENTIKEY_KEYPAIR_FILE:-$HOME/.config/mjolnir/identikey.json}"
SITE="${SITES_NAME:-lightning-mesh}"

test -f "$KEY" || {
	echo "missing keypair $KEY — mj sites keygen --out ~/.config/mjolnir/identikey.json" >&2
	exit 1
}

bun run build
test -f build/index.html
if find build -type l | grep -q .; then
	echo "build/ contains symlinks (Sites skips them):" >&2
	find build -type l >&2
	exit 1
fi

SEQ=$(date +%s)
echo "publishing $SITE sequence=$SEQ"
mj sites publish ./build \
	--identikey-fp "$FP" \
	--site "$SITE" \
	--keypair-file "$KEY" \
	--sequence "$SEQ"
