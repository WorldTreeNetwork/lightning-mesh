# DRAFT Device/ recipe for MikroTik L23UGSR-5HaxD2HaxD
#
# Where it goes: target/linux/qualcommax/image/ipq50xx.mk
#
# KEY DECISION (see MAPPING.md §0/§1): MikroTik RouterBOOT cannot boot a FIT uImage.itb.
# So this device does NOT use the qualcommax FitImage/UbiFit recipe. It uses the MikroTik
# NAND recipe (append-dtb-elf ELF kernel) adapted to SOC=ipq5018 with ath11k firmware pkgs.
# Therefore DEVICE_DTS_CONFIG (the FIT config@ selector) is intentionally ABSENT.
#
# This block is self-contained: it inlines the equivalent of ipq40xx's
# `Device/mikrotik_nand` so it can drop into ipq50xx.mk without cross-target includes.
# (Alternatively, factor a shared `Device/mikrotik_nand` helper into ipq50xx.mk.)

define Device/mikrotik_l23ugsr-5haxd2haxd
	DEVICE_VENDOR := MikroTik
	DEVICE_MODEL := L23UGSR-5HaxD2HaxD
	SOC := ipq5018

	# --- MikroTik NAND image packaging (from ipq40xx Device/mikrotik_nand) ---
	KERNEL_NAME := vmlinux
	# Initramfs ELF that RouterBOOT netboots via BOOTP/TFTP:
	#   openwrt-qualcommax-ipq50xx-mikrotik_l23ugsr-5haxd2haxd-initramfs-kernel.bin
	KERNEL_INITRAMFS := kernel-bin | append-dtb-elf
	# Permanent NAND kernel: ELF -> ubifs one-file image -> ubinized kernel volume
	KERNEL := kernel-bin | append-dtb-elf | package-kernel-ubifs | ubinize-kernel
	IMAGES := sysupgrade.bin
	IMAGE/sysupgrade.bin := sysupgrade-tar | append-metadata

	# --- NAND geometry (W25N01GW, matches every ipq50xx NAND board) ---
	BLOCKSIZE := 128k
	PAGESIZE := 2048
	NAND_SIZE := 128m
	# Required by package-kernel-ubifs (value from MikroTik hAP ac3):
	KERNEL_UBIFS_OPTS = -m $$(PAGESIZE) -e 124KiB -c $$(PAGESIZE) -x none

	# DEVICE_DTS resolves to $(SOC)-<name> = ipq5018-mikrotik_l23ugsr-5haxd2haxd by default
	# (Device/Default: DEVICE_DTS = $$(SOC)-$(lastword $(subst _, ,$(1)))). Our dts file is
	# named ipq5018-mikrotik-l23ugsr.dts in this draft; rename to match, or set explicitly:
	DEVICE_DTS := ipq5018-mikrotik-l23ugsr
	# NOTE: deliberately NO DEVICE_DTS_CONFIG (not a FIT image).

	# --- Packages ---
	#  ath11k-firmware-ipq5018-qcn6122 : IPQ5018 2.4G + QCN6122 5G firmware/m3/q6 blobs
	#  ipq-wifi-mikrotik_l23ugsr-...   : per-board board-2.bin / BDF (MUST be created; see
	#                                    MAPPING.md §6 — top porting risk; qmi-board-id from box)
	#  kmod-ledtrig-gpio               : GPIO LED triggers (MikroTik convention)
	DEVICE_PACKAGES := ath11k-firmware-ipq5018-qcn6122 \
		ipq-wifi-mikrotik_l23ugsr-5haxd2haxd \
		kmod-ledtrig-gpio
endef
TARGET_DEVICES += mikrotik_l23ugsr-5haxd2haxd

# ---------------------------------------------------------------------------
# Companion changes required OUTSIDE this block (see MAPPING.md):
#
# 1. Kernel config (target/linux/qualcommax/ipq50xx/config-default):
#       CONFIG_MIKROTIK=y
#       CONFIG_MIKROTIK_RB_SYSFS=y
#       CONFIG_MIKROTIK_WLAN_DECOMPRESS_LZ77=y
#       CONFIG_NVMEM_LAYOUT_MIKROTIK=y
#       CONFIG_MTD_ROUTERBOOT_PARTS=y
#
# 2. target/linux/qualcommax/ipq50xx/base-files/lib/upgrade/platform.sh :
#    add platform_do_upgrade_mikrotik_nand() (copy from ipq40xx) and dispatch:
#       mikrotik,l23ugsr-5haxd2haxd) platform_do_upgrade_mikrotik_nand "$1" ;;
#
# 3. base-files/etc/hotplug.d/firmware/11-ath11k-caldata : add mikrotik case sourcing
#    caldata from /sys/firmware/mikrotik/hard_config/wlan_data (see MAPPING.md §5).
#
# 4. base-files/etc/board.d/02_network : add
#       mikrotik,l23ugsr-5haxd2haxd) ucidef_set_interfaces_lan_wan "ether1" "sfp" ;;
#    (role mapping TBD).
#
# 5. NEW package: package/firmware/ipq-wifi (or local feed) entry
#    ipq-wifi-mikrotik_l23ugsr-5haxd2haxd providing board-2.bin (BDF).
# ---------------------------------------------------------------------------
