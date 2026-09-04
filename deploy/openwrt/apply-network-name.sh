#!/usr/bin/env bash
# Wireless-only fleet network-name apply (Lightning Admin / st1.3).
#
# Stages CLIENT_SSID + the current mjolnir-apply + setup-wireless.sh, then
# runs mjolnir-apply RUN_WIRELESS=1. Does NOT stage mjolnir-meshd — a stale
# workstation binary must not downgrade the daemon (field 2026-09-04).
#
# Usage:
#   apply-network-name.sh --env FILE [root@host ...]
#   apply-network-name.sh --ssid 'Lightning Mesh' [root@host ...]
#   LIGHTNING_FLEET_SSH='root@192.168.0.15 root@192.168.0.22' apply-network-name.sh --env FILE
#
# With no hosts, uses overlay addrs from fleet-nodes.conf (same order as
# update-fleet.sh). Unreachable nodes are skipped; a halt stops the rest.
# Zero successful applies is a failure (Admin must not report OK for a
# no-op skip of the whole inventory).
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONF="$DIR/fleet-nodes.conf"
STAGE=/root/mjolnir-stage
HEALTH_TIMEOUT=120
SSID=""
ENV_FILE=""
HOSTS=()

while [ $# -gt 0 ]; do
	case "$1" in
		--env)  ENV_FILE="${2:?--env needs a file}"; shift 2 ;;
		--ssid) SSID="${2:?--ssid needs a value}"; shift 2 ;;
		--health-timeout) HEALTH_TIMEOUT="${2:?}"; shift 2 ;;
		-*) echo "unknown option: $1" >&2; exit 2 ;;
		*) HOSTS+=("$1"); shift ;;
	esac
done

if [ -n "${LIGHTNING_FLEET_SSH:-}" ] && [ "${#HOSTS[@]}" -eq 0 ]; then
	# shellcheck disable=SC2206
	HOSTS=(${LIGHTNING_FLEET_SSH})
fi

if [ "${#HOSTS[@]}" -eq 0 ]; then
	[ -f "$CONF" ] || { echo "inventory missing: $CONF (or pass root@host / LIGHTNING_FLEET_SSH)" >&2; exit 1; }
	while IFS='|' read -r name addr _rest; do
		case "$name" in ''|\#*) continue ;; esac
		HOSTS+=("root@$addr")
	done <"$CONF"
fi

TMP_ENV=""
cleanup() { [ -n "$TMP_ENV" ] && rm -f "$TMP_ENV"; }
trap cleanup EXIT

if [ -z "$ENV_FILE" ]; then
	[ -n "$SSID" ] || { echo "usage: apply-network-name.sh --env FILE | --ssid NAME [root@host ...]" >&2; exit 2; }
	TMP_ENV="$(mktemp "${TMPDIR:-/tmp}/lightning-admin-wireless.XXXXXX.env")"
	# Keep in lockstep with admin/src-tauri render_wireless_env.
	{
		echo "# Network name phones join — not a guild. Association is not membership."
		echo "# Staged by apply-network-name.sh; sourced by mjolnir-apply (set -a)."
		# POSIX single-quote
		q=$(printf "%s" "$SSID" | sed "s/'/'\\\\''/g")
		echo "CLIENT_SSID='$q'"
		echo "CLIENT_ENC='none'"
		echo "CLIENT_KEY=''"
		echo "CLIENT_AP_2G_ENC='none'"
	} >"$TMP_ENV"
	ENV_FILE="$TMP_ENV"
fi
[ -f "$ENV_FILE" ] || { echo "--env file not found: $ENV_FILE" >&2; exit 1; }

APPLY="$DIR/files/usr/sbin/mjolnir-apply"
SETUP="$DIR/setup-wireless.sh"
[ -f "$APPLY" ] || { echo "missing $APPLY" >&2; exit 1; }
[ -f "$SETUP" ] || { echo "missing $SETUP" >&2; exit 1; }

SSH=(ssh -o BatchMode=yes -o ConnectTimeout=6)
SCP=(scp -O -o BatchMode=yes -o ConnectTimeout=6)

label_of() {
	local h="$1"
	h="${h#root@}"
	echo "$h"
}

UPDATED=()
SKIPPED=()
HALTED=""

apply_one() {
	local HOST="$1"
	local NAME
	NAME="$(label_of "$HOST")"
	echo
	echo "===== $NAME — $HOST ====="
	if ! "${SSH[@]}" -o ConnectTimeout=6 "$HOST" true 2>/dev/null; then
		echo ">> UNREACHABLE — skipping"
		SKIPPED+=("$NAME")
		return 0
	fi

	"${SSH[@]}" "$HOST" "mkdir -p $STAGE"
	# Never let a leftover staged daemon ride this radio apply.
	"${SSH[@]}" "$HOST" "
		if [ -f $STAGE/mjolnir-meshd ]; then
			mv -f $STAGE/mjolnir-meshd $STAGE/mjolnir-meshd.HOLD
		fi
	"
	"${SCP[@]}" "$ENV_FILE" "$HOST:$STAGE/wireless.env"
	"${SCP[@]}" "$APPLY" "$HOST:$STAGE/mjolnir-apply"
	"${SCP[@]}" "$SETUP" "$HOST:$STAGE/setup-wireless.sh"
	"${SSH[@]}" "$HOST" "chmod +x $STAGE/mjolnir-apply $STAGE/setup-wireless.sh
cat > $STAGE/apply.env <<EOF
HEALTH_TIMEOUT=$HEALTH_TIMEOUT
RUN_WIRELESS=1
EOF
rm -f $STAGE/result
if command -v setsid >/dev/null 2>&1; then
	(setsid $STAGE/mjolnir-apply </dev/null >$STAGE/apply.log 2>&1 &)
else
	(nohup $STAGE/mjolnir-apply </dev/null >$STAGE/apply.log 2>&1 &)
fi"

	echo ">> waiting for result (SSH may drop — WPS WAN window dies on fw4/wifi reload)"
	local DEADLINE RES=""
	DEADLINE=$(( $(date +%s) + HEALTH_TIMEOUT + 90 ))
	while [ "$(date +%s)" -lt "$DEADLINE" ]; do
		RES=$("${SSH[@]}" -o ConnectTimeout=5 "$HOST" "cat $STAGE/result 2>/dev/null" 2>/dev/null || true)
		[ -n "$RES" ] && break
		sleep 5
	done
	if [ -z "$RES" ]; then
		echo ">> ROLLOUT HALTED at $NAME — result='<unreadable>'. Press WPS or use overlay/LAN, then:"
		echo ">>   ssh $HOST 'cat $STAGE/result; tail -40 $STAGE/apply.log'"
		HALTED="$NAME"
		return 1
	fi
	echo ">> $NAME: $RES"
	"${SSH[@]}" "$HOST" "tail -15 $STAGE/apply.log" 2>/dev/null || true
	case "$RES" in
		OK*)
			UPDATED+=("$NAME")
			# Restore HOLDed meshd so a later full install-node still sees it.
			"${SSH[@]}" "$HOST" "
				if [ -f $STAGE/mjolnir-meshd.HOLD ] && [ ! -f $STAGE/mjolnir-meshd ]; then
					mv -f $STAGE/mjolnir-meshd.HOLD $STAGE/mjolnir-meshd
				fi
			" 2>/dev/null || true
			return 0
			;;
		*)
			echo ">> ROLLOUT HALTED at $NAME — result='$RES'"
			HALTED="$NAME"
			return 1
			;;
	esac
}

for HOST in "${HOSTS[@]}"; do
	if ! apply_one "$HOST"; then
		break
	fi
done

echo
echo "===== fleet rollout summary ====="
if [ "${#UPDATED[@]}" -eq 0 ]; then
	echo "updated:     none"
else
	echo "updated:     ${UPDATED[*]}"
fi
if [ "${#SKIPPED[@]}" -eq 0 ]; then
	echo "unreachable: none"
else
	echo "unreachable: ${SKIPPED[*]}"
fi
[ -n "$HALTED" ] && echo "halted:      $HALTED"

if [ -n "$HALTED" ]; then
	exit 1
fi
if [ "${#UPDATED[@]}" -eq 0 ]; then
	echo ">> no node applied — overlay inventory unreachable from here? set LIGHTNING_FLEET_SSH to WAN root@addrs (after WPS)." >&2
	exit 1
fi
exit 0
