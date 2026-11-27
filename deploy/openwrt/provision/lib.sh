#!/usr/bin/env bash
# Shared helpers for a stock-OpenWrt → Lightning Mesh bring-up.
# Source this file. The router-side commands stay POSIX sh.
set -euo pipefail

# Empty unless the operator passed --bind-iface. Scripts export this.
PROVISION_BIND_IFACE="${PROVISION_BIND_IFACE:-}"
PROVISION_LAN_MAC="${PROVISION_LAN_MAC:-}"
PROVISION_ALLOW_RENEW="${PROVISION_ALLOW_RENEW:-0}"

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
# `mjolnir-meshd id` exits until a house /128 exists. status does not.
provision_parse_node_id() {
	sed -n 's/^  node id:  \([0-9a-fA-F]\{64\}\)$/\1/p' | head -n 1
}

provision_host_user() {
	case "$1" in
		*@*) printf '%s\n' "${1%%@*}" ;;
		*) printf '%s\n' root ;;
	esac
}

provision_host_addr() {
	printf '%s\n' "${1#*@}"
}

# Real binaries, captured before a wrapper is prepended to PATH.
provision_ssh_bin() {
	if [ -z "${PROVISION_SSH_BIN:-}" ]; then
		PROVISION_SSH_BIN="$(command -v ssh)"
	fi
	printf '%s\n' "$PROVISION_SSH_BIN"
}

provision_scp_bin() {
	if [ -z "${PROVISION_SCP_BIN:-}" ]; then
		PROVISION_SCP_BIN="$(command -v scp)"
	fi
	printf '%s\n' "$PROVISION_SCP_BIN"
}

PROVISION_CLEANUP=()
provision_add_cleanup() {
	PROVISION_CLEANUP+=("$1")
}

provision_on_exit() {
	local path
	for path in "${PROVISION_CLEANUP[@]+"${PROVISION_CLEANUP[@]}"}"; do
		rm -rf "$path"
	done
}
trap provision_on_exit EXIT

# OpenSSH still reads ~/.ssh/config. BatchMode refuses a password prompt.
# accept-new stores the host key of a claimed 10.42 address the first time.
# BindInterface is opt-in (PROVISION_BIND_IFACE or --bind-iface) so a stale
# Host stanza cannot pin the cable to the wrong NIC.
provision_ssh() {
	local -a opts=(-o BatchMode=yes -o ConnectTimeout="${PROVISION_CONNECT_TIMEOUT:-8}" -o StrictHostKeyChecking=accept-new)
	if [ -n "${PROVISION_BIND_IFACE:-}" ]; then
		opts+=(-o "BindInterface=${PROVISION_BIND_IFACE}")
	fi
	"$(provision_ssh_bin)" "${opts[@]}" "$@"
}

# install-node.sh and fleet_render_peers call ssh/scp themselves.
provision_install_ssh_wrapper() {
	local dir ssh_bin scp_bin
	[ -n "${PROVISION_SSH_WRAPPER:-}" ] && return 0
	ssh_bin="$(provision_ssh_bin)"
	scp_bin="$(provision_scp_bin)"
	dir="$(mktemp -d)"
	provision_add_cleanup "$dir"
	cat >"$dir/ssh" <<EOF
#!/bin/bash
opts=(-o BatchMode=yes -o StrictHostKeyChecking=accept-new)
if [ -n "\${PROVISION_BIND_IFACE:-}" ]; then
	opts+=(-o "BindInterface=\${PROVISION_BIND_IFACE}")
fi
exec ${ssh_bin} "\${opts[@]}" "\$@"
EOF
	cat >"$dir/scp" <<EOF
#!/bin/bash
opts=(-o BatchMode=yes -o StrictHostKeyChecking=accept-new)
if [ -n "\${PROVISION_BIND_IFACE:-}" ]; then
	opts+=(-o "BindInterface=\${PROVISION_BIND_IFACE}")
fi
has_o=0
for arg in "\$@"; do
	[ "\$arg" = "-O" ] && has_o=1
done
if [ "\$has_o" = 0 ]; then
	exec ${scp_bin} -O "\${opts[@]}" "\$@"
else
	exec ${scp_bin} "\${opts[@]}" "\$@"
fi
EOF
	chmod +x "$dir/ssh" "$dir/scp"
	PROVISION_SSH_WRAPPER="$dir"
	export PROVISION_SSH_WRAPPER PROVISION_BIND_IFACE
	export PATH="$dir:$PATH"
}

# `ip -4 -o addr` text → one IPv4. A 10.42.x.1 client gateway wins over a
# leftover 192.168.1.1. Any other address is the fallback (wr3000s-bench
# still serves 192.168.1.1 as its client LAN).
provision_parse_br_lan() {
	awk '
	{
		for (i = 1; i <= NF; i++) {
			if ($i ~ /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+\/[0-9]+$/) {
				split($i, a, "/")
				ip = a[1]
				n = split(ip, o, ".")
				if (n == 4 && o[1] == 10 && o[2] == 42 && o[4] == 1) {
					print ip
					found = 1
					exit
				}
				if (fallback == "") fallback = ip
			}
		}
	}
	END {
		if (!found && fallback != "") print fallback
	}
	'
}

# `ip -4 neigh` text → the address whose lladdr matches $1.
# Prefer a 10.42.x.1 gateway.
provision_parse_neigh() {
	local mac
	mac="$(printf '%s' "$1" | tr 'A-F' 'a-f')"
	awk -v mac="$mac" '
	function canon(s,    n, a, i, out) {
		n = split(tolower(s), a, ":")
		out = ""
		for (i = 1; i <= n; i++) {
			if (length(a[i]) == 1) a[i] = "0" a[i]
			out = out (i > 1 ? ":" : "") a[i]
		}
		return out
	}
	BEGIN { mac = canon(mac) }
	{
		ip = ""
		ll = ""
		if ($1 ~ /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/) ip = $1
		for (i = 1; i <= NF; i++) if ($i == "lladdr" && i < NF) ll = canon($(i + 1))
		if (ll != mac || ip == "") next
		n = split(ip, o, ".")
		if (n == 4 && o[1] == 10 && o[2] == 42 && o[4] == 1) {
			print ip
			found = 1
			exit
		}
		if (cand == "") cand = ip
	}
	END {
		if (!found && cand != "") print cand
	}
	'
}

# `ip -4 -o addr` on this machine → one 10.42.x.1 per local 10.42 address.
provision_parse_local_gateways() {
	awk '
	{
		for (i = 1; i <= NF; i++) {
			if ($i ~ /^10\.42\.[0-9]+\.[0-9]+\/[0-9]+$/) {
				split($i, a, "/")
				n = split(a[1], o, ".")
				if (n == 4) print "10.42." o[3] ".1"
			}
		}
	}
	' | sort -u
}

# Renewing DHCP on the default-route iface would drop the working uplink.
provision_cable_renew_allowed() {
	local iface="$1" def="$2"
	[ -n "$iface" ] && [ -n "$def" ] && [ "$iface" != "$def" ]
}

provision_route_iface() {
	if command -v ip >/dev/null 2>&1; then
		ip -4 route get "$1" 2>/dev/null | awk '{
			for (i = 1; i <= NF; i++) if ($i == "dev") { print $(i + 1); exit }
		}'
		return 0
	fi
	route -n get "$1" 2>/dev/null | awk '/interface:/{ print $2; exit }'
}

provision_default_iface() {
	if command -v ip >/dev/null 2>&1; then
		ip -4 route show default 2>/dev/null | awk '{
			for (i = 1; i <= NF; i++) if ($i == "dev") { print $(i + 1); exit }
		}'
		return 0
	fi
	route -n get default 2>/dev/null | awk '/interface:/{ print $2; exit }'
}

# Local IPv4 lines for provision_parse_local_gateways. Linux `ip -4 -o addr`
# or macOS ifconfig rewritten to the same `inet a.b.c.d/24` shape.
provision_local_ipv4() {
	if command -v ip >/dev/null 2>&1; then
		ip -4 -o addr show 2>/dev/null
		return 0
	fi
	ifconfig -a 2>/dev/null | awk '
		$1 == "inet" && $2 ~ /^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$/ {
			print "inet " $2 "/24"
		}
	'
}

# Neighbor lines for provision_parse_neigh. Linux `ip neigh` or macOS arp.
provision_neigh_lines() {
	local dev="${1:-}"
	if command -v ip >/dev/null 2>&1; then
		if [ -n "$dev" ]; then
			ip -4 neigh show dev "$dev" 2>/dev/null
		else
			ip -4 neigh show 2>/dev/null
		fi
		return 0
	fi
	if [ -n "$dev" ]; then
		arp -an -i "$dev" 2>/dev/null
	else
		arp -an 2>/dev/null
	fi | awk '
		$2 ~ /^\([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+\)$/ && $4 ~ /:/ {
			ip = $2
			gsub(/[()]/, "", ip)
			mac = $4
			print ip " lladdr " mac
		}
	'
}

provision_renew_cable() {
	local iface="$1" def
	def="$(provision_default_iface || true)"
	if ! provision_cable_renew_allowed "$iface" "$def"; then
		echo "not renewing DHCP on ${iface:-unknown}: that path is the default route" >&2
		return 1
	fi
	if command -v nmcli >/dev/null 2>&1; then
		nmcli -w 20 device reapply "$iface"
		return $?
	fi
	if command -v dhcpcd >/dev/null 2>&1; then
		dhcpcd -n "$iface"
		return $?
	fi
	if command -v networkctl >/dev/null 2>&1; then
		networkctl renew "$iface"
		return $?
	fi
	# macOS: the cable NIC is not NetworkManager. ipconfig renews only that
	# service and does not touch the Wi-Fi default route.
	if command -v ipconfig >/dev/null 2>&1; then
		ipconfig set "$iface" DHCP
		return $?
	fi
	echo "no DHCP client to renew $iface" >&2
	return 1
}

# Print user@addr for the router's current br-lan gateway.
# Stock contact is often root@192.168.1.1. After meshd claims the client
# /24 that address is gone and br-lan is 10.42.x.1.
# PROVISION_LAN_MAC, when set, matches a neighbor after SSH dies.
# PROVISION_ALLOW_RENEW=1 may reapply DHCP once on a non-default iface.
provision_resolve_host() {
	local host="$1" user addr iface lan found renew_ok
	user="$(provision_host_user "$host")"
	addr="$(provision_host_addr "$host")"
	iface="$(provision_route_iface "$addr" || true)"
	if lan="$(provision_ssh "$host" 'ip -4 -o addr show dev br-lan' 2>/dev/null | provision_parse_br_lan)"; then
		if [ -n "$lan" ]; then
			printf '%s@%s\n' "$user" "$lan"
		else
			printf '%s\n' "$host"
		fi
		return 0
	fi
	found=""
	if [ -n "${PROVISION_LAN_MAC:-}" ]; then
		if [ -n "$iface" ]; then
			found="$(provision_neigh_lines "$iface" | provision_parse_neigh "$PROVISION_LAN_MAC" || true)"
		fi
		if [ -z "$found" ]; then
			found="$(provision_neigh_lines | provision_parse_neigh "$PROVISION_LAN_MAC" || true)"
		fi
	fi
	if [ -z "$found" ]; then
		found="$(provision_local_ipv4 | provision_parse_local_gateways | grep -vx "$addr" | head -n 1 || true)"
		# More than one local 10.42 gateway is ambiguous. Use it only when unique.
		local count
		count="$(provision_local_ipv4 | provision_parse_local_gateways | grep -vx "$addr" | wc -l | tr -d ' ')"
		if [ "$count" != 1 ]; then
			found=""
		fi
	fi
	if [ -z "$found" ] && [ "${PROVISION_ALLOW_RENEW:-0}" = 1 ]; then
		renew_ok=0
		provision_renew_cable "$iface" && renew_ok=1 || true
		if [ "$renew_ok" = 1 ]; then
			sleep 3
			if [ -n "${PROVISION_LAN_MAC:-}" ]; then
				found="$(provision_neigh_lines "$iface" | provision_parse_neigh "$PROVISION_LAN_MAC" || true)"
			fi
			if [ -z "$found" ]; then
				count="$(provision_local_ipv4 | provision_parse_local_gateways | grep -vx "$addr" | wc -l | tr -d ' ')"
				if [ "$count" = 1 ]; then
					found="$(provision_local_ipv4 | provision_parse_local_gateways | grep -vx "$addr")"
				fi
			fi
		fi
	fi
	if [ -z "$found" ]; then
		echo "ssh to $host failed, and no claimed 10.42 gateway was visible on the cable" >&2
		return 1
	fi
	echo ">> $host no longer answers; client gateway is $found" >&2
	printf '%s@%s\n' "$user" "$found"
}

# Rewrite field 2 of the member line with this node id. Status 2 when absent.
provision_replace_member_addr() {
	local node_id="$1" addr="$2"
	node_id="$(printf '%s' "$node_id" | tr 'A-F' 'a-f')"
	awk -F'|' -v id="$node_id" -v addr="$addr" '
		BEGIN { OFS = "|"; found = 0 }
		NF < 3 { print; next }
		tolower($3) == id { $2 = addr; found = 1; print; next }
		{ print }
		END { if (!found) exit 2 }
	'
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
	provision_host_ok root@10.42.149.1
	provision_host_ok root@tr3000-b.lan
	! provision_host_ok 'root@192.168.1.1;id'
	! provision_host_ok 192.168.1.1
	provision_name_ok tr3000-b
	! provision_name_ok 'tr 3000'
	printf '%s\n' '8: br-lan inet 192.168.1.1/24 brd 192.168.1.255 scope global br-lan' \
		| provision_parse_br_lan | grep -qx '192.168.1.1'
	printf '%s\n' '8: br-lan inet 192.168.1.1/24 brd 192.168.1.255 scope global br-lan' \
		'8: br-lan inet 10.42.149.1/24 brd 10.42.149.255 scope global br-lan' \
		| provision_parse_br_lan | grep -qx '10.42.149.1'
	printf '%s\n' '10.42.149.1 dev enp0 lladdr d4:0d:ab:46:2f:e8 REACHABLE' \
		'192.168.1.1 dev enp0 lladdr d4:0d:ab:46:2f:e8 STALE' \
		| provision_parse_neigh 'D4:0D:AB:46:2F:E8' | grep -qx '10.42.149.1'
	printf '%s\n' '? (10.42.149.1) at d4:d:ab:46:2f:e8 on en15 ifscope [ethernet]' \
		| provision_neigh_lines >/dev/null
	printf '%s\n' '10.42.149.1 lladdr d4:0d:ab:46:2f:e8' \
		| provision_parse_neigh 'd4:0d:ab:46:2f:e8' | grep -qx '10.42.149.1'
	printf '%s\n' '3: enp0 inet 10.42.149.246/24 brd 10.42.149.255 scope global dynamic enp0' \
		| provision_parse_local_gateways | grep -qx '10.42.149.1'
	provision_cable_renew_allowed enp0 wlp0
	! provision_cable_renew_allowed wlp0 wlp0
	! provision_cable_renew_allowed '' wlp0
	provision_replace_member_addr "$id_b" 10.42.149.1 <"$tmp" | grep -q '^bench-a|10.42.149.1|'"$id_b"'|'
	rc=0
	printf '%s\n' '# comment' | provision_replace_member_addr "$id_a" 10.42.1.1 >/dev/null || rc=$?
	[ "$rc" -eq 2 ]
	rm -f "$tmp"
	echo "provision self-test ok"
}
