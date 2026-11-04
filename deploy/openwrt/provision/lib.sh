#!/usr/bin/env bash
# Shared helpers for a stock-OpenWrt → Lightning Mesh bring-up.
# Source this file. The router-side commands stay POSIX sh.
set -euo pipefail

provision_dir() {
	cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

provision_openwrt_dir() {
	cd "$(provision_dir)/.." && pwd
}

# root@host, host, or user@host. No spaces, no shell metacharacters.
provision_host_ok() {
	case "$1" in
		''|*[!A-Za-z0-9@.:_-]*) return 1 ;;
		*@*) ;;
		*) return 1 ;;
	esac
	case "$1" in
		*@*@*) return 1 ;;
	esac
	return 0
}

provision_name_ok() {
	case "$1" in
		''|*[!A-Za-z0-9._-]*) return 1 ;;
		*) return 0 ;;
	esac
}

# status stdout → one 64-hex id, or empty.
provision_parse_node_id() {
	sed -n 's/^  node id:  \([0-9a-fA-F]\{64\}\)$/\1/p' | head -n 1
}

# Insert a member line after the last member of the same group.
# Prints the new file. Status 2 when that node id is already a member.
# Status 3 when the name is already used by a different id.
provision_insert_member() {
	local name="$1" addr="$2" node_id="$3" model="$4" notes="$5" group="$6"
	local line last_group_line=0 n=0 kind g existing_name existing_addr existing_id raw
	local -a lines=()
	line="${name}|${addr}|${node_id}|${model}|${notes}|${group}"
	node_id="$(printf '%s' "$node_id" | tr 'A-F' 'a-f')"
	while IFS= read -r raw || [ -n "$raw" ]; do
		lines+=("$raw")
	done
	for raw in "${lines[@]}"; do
		n=$((n + 1))
		IFS='|' read -r existing_name existing_addr existing_id _model _notes g <<<"$raw"
		kind="$(fleet_kind "$existing_name" "$existing_addr" "$existing_id")"
		if [ "$kind" = member ]; then
			existing_id="$(printf '%s' "$existing_id" | tr 'A-F' 'a-f')"
			if [ "$existing_id" = "$node_id" ]; then
				return 2
			fi
			if [ "$existing_name" = "$name" ]; then
				return 3
			fi
			if [ "$(fleet_group_name "$g")" = "$(fleet_group_name "$group")" ]; then
				last_group_line=$n
			fi
		fi
	done
	n=0
	for raw in "${lines[@]}"; do
		n=$((n + 1))
		printf '%s\n' "$raw"
		if [ "$last_group_line" -eq 0 ]; then
			continue
		fi
		if [ "$n" -eq "$last_group_line" ]; then
			printf '%s\n' "$line"
		fi
	done
	if [ "$last_group_line" -eq 0 ]; then
		printf '%s\n' "$line"
	fi
}

provision_self_test() {
	local dir id_a id_b tmp out rc
	dir="$(provision_openwrt_dir)"
	# shellcheck source=../fleet-file.sh
	source "$dir/fleet-file.sh"
	id_a="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
	id_b="fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
	tmp="$(mktemp)"
	cat >"$tmp" <<EOF
# comment
house-a|10.42.1.1|$id_a|model|note|house
bench-a|192.168.1.1|$id_b|model|note|bench
# tail
EOF
	out="$(provision_insert_member new-b 192.168.1.1 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa "Cudy TR3000 v1" "cable" bench <"$tmp")"
	printf '%s\n' "$out" | grep -q '^bench-a|'
	printf '%s\n' "$out" | grep -q '^new-b|192.168.1.1|'
	# new bench line sits after the existing bench member and before the tail comment
	awk '
		/^bench-a\|/ { seen=1 }
		/^new-b\|/ { if (!seen) exit 1; got=1 }
		/^# tail/ { if (!got) exit 1 }
	' <<<"$out"
	rc=0
	provision_insert_member other 10.0.0.1 "$id_b" model note bench <"$tmp" >/dev/null || rc=$?
	[ "$rc" -eq 2 ]
	rc=0
	provision_insert_member bench-a 10.0.0.2 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa model note bench <"$tmp" >/dev/null || rc=$?
	[ "$rc" -eq 3 ]
	printf '%s\n' "  node id:  $id_a" "  build:    abc" | provision_parse_node_id | grep -qx "$id_a"
	provision_host_ok root@192.168.1.1
	provision_host_ok root@tr3000-b.lan
	! provision_host_ok 'root@192.168.1.1;id'
	! provision_host_ok 192.168.1.1
	provision_name_ok tr3000-b
	! provision_name_ok 'tr 3000'
	rm -f "$tmp"
	echo "provision self-test ok"
}
