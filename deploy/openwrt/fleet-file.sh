#!/usr/bin/env bash
# Which installation a fleet tool walks.
#
# One file is one working set: the nodes you update together, in rollout
# order. A second installation is another file, not more lines in this one.
# Guild membership (who may administer a place) lives in the local guild
# log, not here. Who answers SSH right now lives in the reachability JSON
# written by fleet-scan.sh, not here.
#
#   LIGHTNING_FLEET_FILE=/path/to.conf    explicit file
#   LIGHTNING_FLEET=name                  deploy/openwrt/fleet.d/name.conf
#   (unset)                               ~/.config/lightning-admin/fleet-location
#                                         (one stem), else fleet-nodes.conf
#   LIGHTNING_FLEET_REACH=/path/to.json   scan result. A named installation
#                                         uses fleet-reach.<name>.json so one
#                                         site's dials are not the other's.
#
# Columns: name|ssh_addr|node_id|model|notes|group
# A member has a name, an SSH address, and a 64-hex node id.
# `name|-|-|model|notes` records hardware with no node id yet. Rollout skips it.
# `group` defaults to `house` when the column is empty. Gossip peers are the
# other members of the same group. `bench` is a cabled unit, not a house peer.
# `away` keeps a node id that is not on this backhaul and is not a house peer.
# ssh_addr is the fallback dial address. fleet_dial_addr prefers the scan.
#
# Source this file. `fleet_conf <deploy/openwrt dir>` prints the path.
# `fleet_kind <name> <addr> <node_id>` prints member, pending, or skip.
# `fleet_peer_ids <conf> <self_id>` prints the other members of that group.
# `fleet_render_peers <conf> <self_id> <ssh_host>` replaces the UCI peer list.
# `fleet_print_references <conf> <house-guild-id file>` prints the operator view.
# Or run: bash deploy/openwrt/fleet-file.sh --reference <house-guild-id file>

# One stem, or empty. LIGHTNING_FLEET_LOCATION_FILE overrides the path
# (use an empty file to ignore the laptop pin).
fleet_location_stem() {
	local f stem
	if [ -n "${LIGHTNING_FLEET_LOCATION_FILE+x}" ]; then
		f="$LIGHTNING_FLEET_LOCATION_FILE"
	else
		f="${HOME}/.config/lightning-admin/fleet-location"
	fi
	[ -n "$f" ] && [ -f "$f" ] || return 1
	stem="$(head -n 1 "$f" | tr -d '[:space:]')"
	case "$stem" in
		'') return 1 ;;
		*[!A-Za-z0-9._-]*)
			echo "fleet-location must be a file stem (letters, numbers, '.', '_', '-'): '$stem'" >&2
			return 2
			;;
	esac
	printf '%s\n' "$stem"
}

# Named installation, if one is selected. Status 1 when this is the default file.
fleet_selected_name() {
	local stem
	if [ -n "${LIGHTNING_FLEET:-}" ]; then
		stem="$LIGHTNING_FLEET"
	else
		stem="$(fleet_location_stem)" || {
			rc=$?
			[ "$rc" -eq 1 ] && return 1
			return "$rc"
		}
	fi
	case "$stem" in
		''|*[!A-Za-z0-9._-]*)
			echo "LIGHTNING_FLEET must be a file stem (letters, numbers, '.', '_', '-'): '$stem'" >&2
			return 2
			;;
	esac
	printf '%s\n' "$stem"
}

fleet_conf() {
	local dir="$1"
	local name path rc
	if [ -n "${LIGHTNING_FLEET_FILE:-}" ]; then
		if [ ! -f "$LIGHTNING_FLEET_FILE" ]; then
			echo "inventory missing: $LIGHTNING_FLEET_FILE" >&2
			return 1
		fi
		printf '%s\n' "$LIGHTNING_FLEET_FILE"
		return 0
	fi
	if name="$(fleet_selected_name)"; then
		path="$dir/fleet.d/${name}.conf"
		if [ ! -f "$path" ]; then
			echo "inventory missing: $path" >&2
			return 1
		fi
		printf '%s\n' "$path"
		return 0
	else
		rc=$?
		[ "$rc" -eq 1 ] || return "$rc"
	fi
	path="$dir/fleet-nodes.conf"
	if [ ! -f "$path" ]; then
		echo "inventory missing: $path" >&2
		return 1
	fi
	printf '%s\n' "$path"
}

fleet_valid_node_id() {
	[ "${#1}" -eq 64 ] || return 1
	case "$1" in *[!0-9a-fA-F]*) return 1 ;; esac
}

fleet_kind() {
	local name="$1" addr="$2" node_id="$3"
	case "$name" in
		''|\#*) printf 'skip\n'; return 0 ;;
	esac
	case "$addr" in
		''|-) printf 'pending\n'; return 0 ;;
	esac
	case "$node_id" in
		''|-) printf 'pending\n'; return 0 ;;
	esac
	fleet_valid_node_id "$node_id" || { printf 'skip\n'; return 0; }
	printf 'member\n'
}

# Sixth column. Empty means house, so a five-column line still peers with house.
fleet_group_name() {
	local group="$1"
	case "$group" in
		''|house) printf 'house\n' ;;
		*[!A-Za-z0-9._-]*) printf 'house\n' ;;
		*) printf '%s\n' "$group" ;;
	esac
}

# Group of one member id. Empty (and status 1) when that id is not a member.
fleet_member_group() {
	local conf="$1" self_id="$2"
	local name addr node_id _model _notes group
	self_id="$(printf '%s' "$self_id" | tr 'A-F' 'a-f')"
	while IFS='|' read -r name addr node_id _model _notes group || [ -n "$name" ]; do
		[ "$(fleet_kind "$name" "$addr" "$node_id")" = member ] || continue
		node_id="$(printf '%s' "$node_id" | tr 'A-F' 'a-f')"
		if [ "$node_id" = "$self_id" ]; then
			fleet_group_name "$group"
			return 0
		fi
	done <"$conf"
	return 1
}

fleet_peer_ids() {
	local conf="$1" self_id="$2" want
	local name addr node_id _model _notes group
	want="$(fleet_member_group "$conf" "$self_id")" || return 0
	self_id="$(printf '%s' "$self_id" | tr 'A-F' 'a-f')"
	while IFS='|' read -r name addr node_id _model _notes group || [ -n "$name" ]; do
		[ "$(fleet_kind "$name" "$addr" "$node_id")" = member ] || continue
		[ "$(fleet_group_name "$group")" = "$want" ] || continue
		node_id="$(printf '%s' "$node_id" | tr 'A-F' 'a-f')"
		[ "$node_id" = "$self_id" ] || printf '%s\n' "$node_id"
	done <"$conf"
}

# Scan file from fleet-scan.sh. Not the working set and not a guild.
# A named installation keeps its own dial file. The default file stays the
# other site's pins (the same 10.42.x address is a different radio there).
fleet_reach_file() {
	local stem
	if [ -n "${LIGHTNING_FLEET_REACH:-}" ]; then
		printf '%s\n' "$LIGHTNING_FLEET_REACH"
		return 0
	fi
	if stem="$(fleet_selected_name 2>/dev/null)"; then
		printf '%s\n' "${HOME}/.config/lightning-admin/fleet-reach.${stem}.json"
		return 0
	fi
	printf '%s\n' "${HOME}/.config/lightning-admin/fleet-reach.json"
}

# Print the address a scan recorded for this node id. Status 1 when absent.
fleet_reach_addr() {
	local node_id="$1" file
	file="$(fleet_reach_file)"
	[ -f "$file" ] || return 1
	python3 - "$file" "$node_id" <<'PY'
import json, sys
path, nid = sys.argv[1], sys.argv[2].lower()
try:
    data = json.load(open(path))
except (OSError, ValueError):
    sys.exit(1)
for key, row in (data.get("nodes") or {}).items():
    if str(key).lower() != nid:
        continue
    ssh = str((row or {}).get("ssh") or "").strip()
    if ssh.startswith("root@"):
        ssh = ssh[5:]
    if ssh:
        print(ssh)
        sys.exit(0)
sys.exit(1)
PY
}

# Address rollout tools dial. The scan wins when it has this node id.
fleet_dial_addr() {
	local node_id="$1" fallback="$2" got
	if got="$(fleet_reach_addr "$node_id" 2>/dev/null)"; then
		printf '%s\n' "$got"
	else
		printf '%s\n' "$fallback"
	fi
}

fleet_peer_script() {
	local peers peer_id
	peers="$(fleet_peer_ids "$1" "$2")" || return 1
	printf '%s\n' 'set -e' 'uci -q delete mjolnir.meshd.peer || true'
	while IFS= read -r peer_id; do
		[ -n "$peer_id" ] || continue
		printf "uci add_list mjolnir.meshd.peer='%s'\n" "$peer_id"
	done <<<"$peers"
	printf '%s\n' 'uci commit mjolnir'
}

fleet_render_peers() {
	local script
	script="$(fleet_peer_script "$1" "$2")" || return 1
	ssh -o BatchMode=yes -o ConnectTimeout=6 "$3" 'sh -s' <<<"$script"
}

# Build a small host formatter against the library, so the displayed reference
# is router_ula's Rust Display, never a shell reimplementation of its hash.
# The temporary Cargo project and its build output stay outside the checkout.
fleet_print_references() (
	set -euo pipefail
	local conf="$1" guild_file="$2" repo_dir scratch crate_path
	repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
	scratch="$(mktemp -d)"
	trap 'rm -rf "$scratch"' EXIT
	crate_path="$repo_dir/crates/mjolnir-mesh"
	crate_path="${crate_path//\\/\\\\}"
	crate_path="${crate_path//\"/\\\"}"
	cat >"$scratch/Cargo.toml" <<EOF
[package]
name = "lightning-fleet-reference"
version = "0.0.0"
edition = "2021"
[dependencies]
mjolnir-mesh = { path = "$crate_path" }
[[bin]]
name = "lightning-fleet-reference"
path = "main.rs"
EOF
	cp -f "$repo_dir/Cargo.lock" "$scratch/Cargo.lock"
	cat >"$scratch/main.rs" <<'RS'
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let inventory = std::fs::read_to_string(args.next().ok_or("missing inventory")?)?;
    let guild = std::fs::read_to_string(args.next().ok_or("missing house guild file")?)?;
    let view = mjolnir_mesh::tun::link::fleet_reference_inventory(&inventory, &guild)
        .ok_or("house guild file must contain a 32-byte hex guild id")?;
    print!("{view}");
    Ok(())
}
RS
	CARGO_TARGET_DIR="${LIGHTNING_REFERENCE_TARGET_DIR:-/tmp/lm-fleet-reference-target}" \
		cargo run --quiet --manifest-path "$scratch/Cargo.toml" -- "$conf" "$guild_file"
)

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
	set -euo pipefail
	# The laptop pin must not change this self-test.
	export LIGHTNING_FLEET_LOCATION_FILE=/dev/null
	DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
	if [ "${1:-}" = --reference ]; then
		if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
			echo "usage: $0 --reference <house-guild-id file> [inventory.conf]" >&2
			exit 1
		fi
		fleet_print_references "${3:-$(fleet_conf "$DIR")}" "$2"
		exit
	fi
	got="$(fleet_conf "$DIR")"
	[ "$got" = "$DIR/fleet-nodes.conf" ]
	members=0
	pending=0
	while IFS='|' read -r name addr node_id _model _notes; do
		case "$(fleet_kind "$name" "$addr" "$node_id")" in
			member) members=$((members + 1)) ;;
			pending) pending=$((pending + 1)) ;;
		esac
	done <"$got"
	[ "$members" -ge 1 ]
	[ "$pending" -ge 1 ]
	tmp="$(mktemp)"
	trap 'rm -f "$tmp"' EXIT
	printf 'only|-|-|model|note\n' >"$tmp"
	LIGHTNING_FLEET_FILE="$tmp"
	[ "$(fleet_conf "$DIR")" = "$tmp" ]
	[ "$(fleet_kind only - -)" = pending ]
	id_a="$(printf '%064d' 1)"
	id_b="$(printf '%064d' 2)"
	id_c="$(printf '%064d' 3)"
	mac='80:af:ca:92:a6:79'
	cat >"$tmp" <<EOF
# name|backhaul_addr|node_id|model|notes
a|10.254.1.1|$id_a|model|ethernet $mac
b|10.254.1.2|$id_b|model|note
c|10.254.1.3|$id_c|model|note
pending|-|-|model|ethernet $mac
EOF
	peers="$(fleet_peer_ids "$tmp" "$id_b")"
	[ "$peers" = "$(printf '%s\n' "$id_a" "$id_c")" ]
	[ -z "$(fleet_peer_ids "$tmp" "$(printf '%064d' 4)")" ]
	script="$(fleet_peer_script "$tmp" "$id_b")"
	[ "$script" = "$(printf '%s\n' 'set -e' 'uci -q delete mjolnir.meshd.peer || true';
		printf "uci add_list mjolnir.meshd.peer='%s'\n" "$id_a" "$id_c";
		printf '%s\n' 'uci commit mjolnir')" ]
	case "$peers$script" in
		*"$mac"*|*pending*) echo 'non-peer data emitted' >&2; exit 1 ;;
	esac
	id_d="$(printf '%064d' 5)"
	cat >"$tmp" <<EOF
a|10.254.1.1|$id_a|model|note|house
b|10.254.1.2|$id_b|model|note|house
d|192.168.1.1|$id_d|model|cabled|bench
EOF
	[ "$(fleet_member_group "$tmp" "$id_d")" = bench ]
	[ "$(fleet_peer_ids "$tmp" "$id_b")" = "$id_a" ]
	[ -z "$(fleet_peer_ids "$tmp" "$id_d")" ]
	[ "$(fleet_peer_ids "$tmp" "$id_a")" = "$id_b" ]
	[ "$(fleet_kind pending - "$id_a")" = pending ]
	[ "$(fleet_kind pending 10.254.1.4 -)" = pending ]
	[ "$(fleet_kind bad 10.254.1.4 "$mac")" = skip ]
	[ "$(fleet_kind bad 10.254.1.4 "${id_a}0")" = skip ]
	unset LIGHTNING_FLEET_FILE
	if LIGHTNING_FLEET='../fleet-nodes' fleet_conf "$DIR" >/dev/null 2>&1; then
		echo "path escape was accepted" >&2
		exit 1
	fi
	loc_home="$(mktemp -d)"
	loc_dir="$(mktemp -d)"
	mkdir -p "$loc_home/.config/lightning-admin" "$loc_dir/fleet.d"
	printf 'a|10.254.1.1|%s|model|note|embassy\n' "$id_a" >"$loc_dir/fleet.d/embassy.conf"
	printf 'embassy\n' >"$loc_home/.config/lightning-admin/fleet-location"
	unset LIGHTNING_FLEET LIGHTNING_FLEET_REACH
	saved_location_file="$LIGHTNING_FLEET_LOCATION_FILE"
	unset LIGHTNING_FLEET_LOCATION_FILE
	got_loc="$(HOME="$loc_home" fleet_conf "$loc_dir")"
	[ "$got_loc" = "$loc_dir/fleet.d/embassy.conf" ]
	reach_default="$(HOME="$loc_home" fleet_reach_file)"
	[ "$reach_default" = "$loc_home/.config/lightning-admin/fleet-reach.embassy.json" ]
	reach_explicit="$(HOME="$loc_home" LIGHTNING_FLEET_REACH=/tmp/explicit-reach.json fleet_reach_file)"
	[ "$reach_explicit" = /tmp/explicit-reach.json ]
	printf '../x\n' >"$loc_home/.config/lightning-admin/fleet-location"
	if HOME="$loc_home" fleet_conf "$loc_dir" >/dev/null 2>&1; then
		echo "location path escape was accepted" >&2
		exit 1
	fi
	: >"$loc_home/.config/lightning-admin/fleet-location"
	got_empty="$(HOME="$loc_home" fleet_conf "$DIR")"
	[ "$got_empty" = "$DIR/fleet-nodes.conf" ]
	export LIGHTNING_FLEET_LOCATION_FILE="$saved_location_file"
	rm -rf "$loc_home" "$loc_dir"
	echo "fleet-file ok ($members members, $pending pending in $got)"
fi
