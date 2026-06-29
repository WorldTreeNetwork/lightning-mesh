#!/usr/bin/env bash
# RouterBOOT netboot server for the MikroTik L23UGSR: serves one OpenWrt
# initramfs ELF to the box via BOOTP + TFTP, and ONLY to the box (it ignores
# every other DHCP/BOOTP request on the LAN so the main-pass nodes are untouched).
#
# Non-destructive: RouterBOOT loads this into RAM; RouterOS stays on NAND.
#
#   sudo-less: the script self-sudos dnsmasq.
#   ./netboot.sh <initramfs-kernel.bin> [iface] [box-mac]
#
# Then, separately, flip the box to one-shot etherboot and reboot it:
#   ./trigger-netboot.sh        (sets boot-device=try-ethernet-once-then-nand; reboot)
set -euo pipefail

IMG="${1:?usage: netboot.sh <initramfs-kernel.bin> [iface=eno1] [box-mac=F4:1E:57:9F:F5:00]}"
IFACE="${2:-eno1}"
BOXMAC="${3:-F4:1E:57:9F:F5:00}"
BOOTIP="${BOOTIP:-192.168.0.240}"          # transient lease handed to the box for boot
NETMASK="${NETMASK:-255.255.255.0}"

[ -f "$IMG" ] || { echo "!! image not found: $IMG"; exit 1; }
TFTP="$(mktemp -d /tmp/l23-tftp.XXXXXX)"
cp "$IMG" "$TFTP/boot.bin"
CONF="$(mktemp /tmp/l23-dnsmasq.XXXXXX.conf)"
cat > "$CONF" <<EOF
interface=$IFACE
bind-dynamic
port=0
dhcp-range=$BOOTIP,$BOOTIP,$NETMASK,5m
dhcp-host=$BOXMAC,$BOOTIP,set:l23
dhcp-boot=tag:l23,boot.bin
dhcp-ignore=tag:!l23
enable-tftp
tftp-root=$TFTP
log-dhcp
EOF
echo ">> serving $(basename "$IMG") as boot.bin"
echo ">> iface=$IFACE  box-mac=$BOXMAC  boot-ip=$BOOTIP"
echo ">> tftp-root=$TFTP  conf=$CONF"
echo ">> (Ctrl-C to stop) — watching for the box's BOOTP request..."
exec sudo dnsmasq -d -C "$CONF"
