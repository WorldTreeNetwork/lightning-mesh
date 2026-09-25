#!/usr/bin/env bash
# Start lightning-sim libvirt nets + node-a/node-b (q35). Like morphist-win11.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SIM="${REPO}/deploy/sim"
DISKS="${SIM}/disks"
IMG_GZ="${SIM}/openwrt-x86-64-generic-ext4-combined-efi.img.gz"
URI="${VIRSH_URI:-qemu:///system}"
V() { virsh -c "${URI}" "$@"; }

need_img() {
	[ -f "${IMG_GZ}" ] || { echo "missing ${IMG_GZ} — run deploy/sim/build-image.sh" >&2; exit 1; }
}

ensure_net() {
	local xml="$1" name
	name="$(sed -n 's:.*<name>\([^<]*\)</name>.*:\1:p' "${xml}" | head -1)"
	if ! V net-info "${name}" >/dev/null 2>&1; then
		V net-define "${xml}"
	fi
	V net-start "${name}" >/dev/null 2>&1 || true
	V net-autostart "${name}" >/dev/null 2>&1 || true
}

ensure_disk() {
	local name="$1"
	mkdir -p "${DISKS}"
	if [ ! -f "${DISKS}/base.qcow2" ]; then
		echo ">> converting ${IMG_GZ} -> ${DISKS}/base.qcow2"
		local raw="${DISKS}/base.raw"
		gzip -dc "${IMG_GZ}" >"${raw}"
		qemu-img convert -f raw -O qcow2 "${raw}" "${DISKS}/base.qcow2"
		rm -f "${raw}"
	fi
	if [ ! -f "${DISKS}/${name}.qcow2" ]; then
		qemu-img create -f qcow2 -F qcow2 -b "${DISKS}/base.qcow2" "${DISKS}/${name}.qcow2" 256M
	fi
}

ensure_domain() {
	local name="$1" mac="$2" tmpl="${3:-${SIM}/libvirt/domain.xml.in}" tmp
	ensure_disk "${name}"
	tmp="$(mktemp)"
	sed -e "s|NAME|${name}|g" \
		-e "s|NVRAM|${DISKS}/${name}.nvram|g" \
		-e "s|DISK|${DISKS}/${name}.qcow2|g" \
		-e "s|MGMTMAC|${mac}|g" \
		"${tmpl}" >"${tmp}"
	if V dominfo "${name}" >/dev/null 2>&1; then
		rm -f "${tmp}"
	else
		V define "${tmp}"
		rm -f "${tmp}"
	fi
	V start "${name}" >/dev/null 2>&1 || true
}

need_img
ensure_net "${SIM}/libvirt/net-mgmt.xml"
ensure_net "${SIM}/libvirt/net-isp.xml"
ensure_net "${SIM}/libvirt/net-nat2.xml"
ensure_domain sim-node-a 52:54:00:4c:4d:0a
ensure_domain sim-node-b 52:54:00:4c:4d:0b
ensure_domain sim-isp-cpe 52:54:00:4c:4d:0c "${SIM}/libvirt/domain-cpe.xml.in"
ensure_domain sim-house-nat 52:54:00:4c:4d:0d
ensure_domain sim-phone 52:54:00:4c:4d:0e "${SIM}/libvirt/domain-phone.xml.in"
echo ">> waiting for mgmt leases"
for i in $(seq 1 40); do
	if V net-dhcp-leases lightning-sim-mgmt 2>/dev/null | grep -q 10.99.0.10 \
		&& V net-dhcp-leases lightning-sim-mgmt 2>/dev/null | grep -q 10.99.0.11; then
		V net-dhcp-leases lightning-sim-mgmt
		echo ">> ssh -o BatchMode=yes root@10.99.0.10  # node-a"
		echo ">> ssh -o BatchMode=yes root@10.99.0.11  # node-b"
		echo ">> ssh -o BatchMode=yes root@10.99.0.12  # isp-cpe"
		echo ">> ssh -o BatchMode=yes root@10.99.0.13  # house-nat"
		exit 0
	fi
	sleep 2
done
echo ">> no leases yet; domains started. Check: virsh -c ${URI} net-dhcp-leases lightning-sim-mgmt" >&2
V list --all
exit 0
