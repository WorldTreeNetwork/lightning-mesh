#!/usr/bin/env bash
# Which installation a fleet tool walks.
#
# One file is one installation: the nodes updated, flashed, and upgraded
# together, and one gossip peer set. A second
# household is another file, not more lines in this one.
#
#   LIGHTNING_FLEET_FILE=/path/to.conf    explicit file
#   LIGHTNING_FLEET=name                  deploy/openwrt/fleet.d/name.conf
#   (unset)                               deploy/openwrt/fleet-nodes.conf
#
# Columns: name|backhaul_addr|node_id|model|notes
# A member has a name, backhaul address, and 64-hex node id.
# `name|-|-|model|notes` records hardware in this installation that has no
# node id yet. Rollout tools skip those lines.
#
# Source this file. `fleet_conf <deploy/openwrt dir>` prints the path.
# `fleet_kind <name> <addr> <node_id>` prints member, pending, or skip.
# `fleet_peer_ids <conf> <self_id>` prints the other members' ids.
# `fleet_render_peers <conf> <self_id> <ssh_host>` replaces the UCI peer list.
# `fleet_print_references <conf> <house-guild-id file>` prints the operator view.
# Or run: bash deploy/openwrt/fleet-file.sh --reference <house-guild-id file>

fleet_conf() {
	local dir="$1"
	local name path
	if [ -n "${LIGHTNING_FLEET_FILE:-}" ]; then
		if [ ! -f "$LIGHTNING_FLEET_FILE" ]; then
			echo "inventory missing: $LIGHTNING_FLEET_FILE" >&2
			return 1
		fi
		printf '%s\n' "$LIGHTNING_FLEET_FILE"
		return 0
	fi
	if [ -n "${LIGHTNING_FLEET:-}" ]; then
		name="$LIGHTNING_FLEET"
		case "$name" in
			''|*[!A-Za-z0-9._-]*)
				echo "LIGHTNING_FLEET must be a file stem (letters, numbers, '.', '_', '-'): '$name'" >&2
				return 1
				;;
		esac
		path="$dir/fleet.d/${name}.conf"
		if [ ! -f "$path" ]; then
			echo "inventory missing: $path" >&2
			return 1
		fi
		printf '%s\n' "$path"
		return 0
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

fleet_peer_ids() {
	local conf="$1" self_id="$2"
	local name addr node_id model notes
	self_id="$(printf '%s' "$self_id" | tr 'A-F' 'a-f')"
	while IFS='|' read -r name addr node_id model notes || [ -n "$name" ]; do
		[ "$(fleet_kind "$name" "$addr" "$node_id")" = member ] || continue
		node_id="$(printf '%s' "$node_id" | tr 'A-F' 'a-f')"
		[ "$node_id" = "$self_id" ] || printf '%s\n' "$node_id"
	done <"$conf"
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
	[ "$(fleet_peer_ids "$tmp" "$(printf '%064d' 4)")" = "$(printf '%s\n' "$id_a" "$id_b" "$id_c")" ]
	[ "$(fleet_kind pending - "$id_a")" = pending ]
	[ "$(fleet_kind pending 10.254.1.4 -)" = pending ]
	[ "$(fleet_kind bad 10.254.1.4 "$mac")" = skip ]
	[ "$(fleet_kind bad 10.254.1.4 "${id_a}0")" = skip ]
	script="$(fleet_peer_script "$tmp" "$id_b")"
	[ "$script" = "$(printf '%s\n' 'set -e' 'uci -q delete mjolnir.meshd.peer || true';
		printf "uci add_list mjolnir.meshd.peer='%s'\n" "$id_a" "$id_c";
		printf '%s\n' 'uci commit mjolnir')" ]
	case "$peers$script" in
		*"$mac"*|*pending*) echo 'non-peer data emitted' >&2; exit 1 ;;
	esac
	unset LIGHTNING_FLEET_FILE
	if LIGHTNING_FLEET='../fleet-nodes' fleet_conf "$DIR" >/dev/null 2>&1; then
		echo "path escape was accepted" >&2
		exit 1
	fi
	echo "fleet-file ok ($members members, $pending pending in $got)"
fi
