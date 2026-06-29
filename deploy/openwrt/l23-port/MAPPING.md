# L23UGSR-5HaxD2HaxD → OpenWrt port: template digest (MAPPING)

Target: `qualcommax/ipq50xx`, SOC string `ipq5018`.
Device: MikroTik **L23UGSR-5HaxD2HaxD** (IPQ-5010 "Maple", 256MB RAM, 128MB Winbond
W25N01GW spi-nand, RouterBOOT).
Radios: 2.4GHz integrated IPQ5018 (ath11k, `wifi@c000000`) + 5GHz **QCN-6102** treated as
QCN6122 (ath11k AHB, `wifi@b00a040`).

This file digests the two reference template families and states exactly what to copy,
what to change, and the one genuinely novel risk (ath11k caldata from RouterBOOT instead
of a raw `0:ART`/`0:art` partition).

All quoted paths are relative to `/home/dorje/work/IdentiKey/openwrt-l23/openwrt`.

---

## 0. The central insight (read this first)

A normal ipq50xx device (Yuncore AX830, CMCC MR3000D-CI, Linksys MX2000, …) is built with
the qualcommax **FitImage + UbiFit** recipe: the kernel is a **FIT `uImage.itb`** selected
by `DEVICE_DTS_CONFIG := config@…`, placed in a UBI volume; the bootloader (Qualcomm
APPSBL/U-Boot) understands FIT.

MikroTik **RouterBOOT does not understand FIT.** It netboots a raw **ELF** kernel over
BOOTP/TFTP and, for permanent install, loads an **ELF** kernel out of a UBI "kernel" volume
on NAND. Therefore the L23 must be built with the **MikroTik NAND** image recipe
(`Device/mikrotik_nand` from `ipq40xx/image/mikrotik.mk`) — `kernel-bin | append-dtb-elf`
— **not** the qualcommax `FitImage`/`DEVICE_DTS_CONFIG` path. We keep `SOC := ipq5018` and
the ath11k firmware packages from the ipq50xx side, but the *image packaging* comes wholesale
from the MikroTik side. `DEVICE_DTS_CONFIG` (FIT config selector) is unused; `DEVICE_DTS`
just names the `.dtb` that `append-dtb-elf` splices into the ELF.

---

## 1. MikroTik ARM image-recipe pattern (the MikroTik half)

Source: `target/linux/ipq40xx/image/mikrotik.mk`.

### 1a. The reusable NAND base recipe (this is what L23 marries to ipq5018)

```make
define Device/mikrotik_nand
	DEVICE_VENDOR := MikroTik
	KERNEL_NAME := vmlinux
	KERNEL_INITRAMFS := kernel-bin | append-dtb-elf
	KERNEL := kernel-bin | append-dtb-elf | package-kernel-ubifs | \
		ubinize-kernel
	IMAGES := sysupgrade.bin
	IMAGE/sysupgrade.bin := sysupgrade-tar | append-metadata
endef
```

and the concrete NAND device that consumes it (hAP ac3 — our closest structural twin: ipq40xx
**NAND**, RouterBOOT, dynamic RouterBoot partitions, UBI):

```make
define Device/mikrotik_hap-ac3
	$(call Device/mikrotik_nand)
	DEVICE_MODEL := hAP ac3
	SOC := qcom-ipq4019
	BLOCKSIZE := 128k
	PAGESIZE := 2048
	KERNEL_UBIFS_OPTS = -m $$(PAGESIZE) -e 124KiB -c $$(PAGESIZE) -x none
	DEVICE_PACKAGES := kmod-ledtrig-gpio
endef
TARGET_DEVICES += mikrotik_hap-ac3
```

`BLOCKSIZE := 128k` / `PAGESIZE := 2048` already match the ipq50xx NAND boards (W25N01GW),
so these carry over unchanged.

### 1b. The image-command chain, expanded

The macros are defined in `include/image-commands.mk` and
`target/linux/qualcommax/image/Makefile`:

- **`Build/kernel-bin`** — raw kernel `Image`/`vmlinux`.
- **`Build/append-dtb-elf`** (`include/image-commands.mk:19`):
  ```make
  define Build/append-dtb-elf
  	$(TARGET_CROSS)objcopy \
  		--set-section-flags=.appended_dtb=alloc,contents \
  		--update-section \
  		.appended_dtb=$(KDIR)/image-$(firstword $(DEVICE_DTS)).dtb $@
  endef
  ```
  Splices the device `.dtb` into the `.appended_dtb` ELF section of `vmlinux`. The result is
  a **self-contained bootable ELF** with no external dtb — exactly what RouterBOOT expects.
- **`Build/package-kernel-ubifs`** (`include/image-commands.mk:30`): wraps the ELF in a tiny
  one-file (`kernel`) UBIFS image using `KERNEL_UBIFS_OPTS`.
- **`Build/ubinize-kernel`** (`include/image-commands.mk:227`): runs
  `scripts/ubinize-image.sh --kernel …` with `-p $(BLOCKSIZE) -m $(PAGESIZE)` to produce the
  UBI image of the kernel volume.

### 1c. How RouterBOOT netboot consumes the artifact

`KERNEL_INITRAMFS := kernel-bin | append-dtb-elf` produces the **initramfs ELF** artifact:

```
bin/targets/qualcommax/ipq50xx/openwrt-qualcommax-ipq50xx-mikrotik_l23ugsr-5haxd2haxd-initramfs-kernel.bin
```

(`include/image.mk:512-515`: `KERNEL_INITRAMFS_PREFIX = <img-prefix>-initramfs`,
`KERNEL_INITRAMFS_SUFFIX = KERNEL_SUFFIX` → default `-kernel.bin`.) Despite the `.bin`
suffix it is an **ELF**. RouterBOOT, set to "boot device = try ethernet once then NAND"
(or boot protocol = bootp), pulls this file via BOOTP/TFTP and `bootelf`s it straight into
RAM. The initramfs rootfs is embedded, so the box comes up fully in RAM with no flash writes
— the safe bring-up vehicle.

### 1d. How permanent NAND install works

1. Netboot the initramfs ELF (1c). You now have a running OpenWrt in RAM.
2. `sysupgrade` the `…-sysupgrade.bin` (which is `sysupgrade-tar | append-metadata` — a tarball
   carrying `kernel` (the ubinized ELF) + `root` (squashfs)).
3. The platform hook writes the kernel into the NAND **UBI** "kernel" volume and the rootfs
   into UBI. From `target/linux/ipq40xx/base-files/lib/upgrade/platform.sh`:
   ```sh
   platform_do_upgrade_mikrotik_nand() {
   	local fw_mtd=$(find_mtd_part kernel)
   	fw_mtd="${fw_mtd/block/}"
   	[ -n "$fw_mtd" ] || return
   	local board_dir=$(tar tf "$1" | grep -m 1 '^sysupgrade-.*/$')
   	board_dir=${board_dir%/}
   	local kernel_len=$(tar xf "$1" ${board_dir}/kernel -O | wc -c)
   	tar xf "$1" ${board_dir}/kernel -O | ubiformat "$fw_mtd" -y -S $kernel_len -f -
   	CI_KERNPART="none"
   	nand_do_upgrade "$1"
   }
   ```
   and the dispatch:
   ```sh
   mikrotik,hap-ac3)
   	platform_do_upgrade_mikrotik_nand "$1"
   	;;
   ```
   RouterBOOT then loads the ELF kernel from the `kernel` UBI volume (NAND) at every boot.

   > **PORT TASK:** ipq50xx's `target/linux/qualcommax/ipq50xx/base-files/lib/upgrade/platform.sh`
   > does **not** have `platform_do_upgrade_mikrotik_nand`. It must be copied over (plus the
   > `find_mtd_part kernel`/`ubiformat` logic) and a `mikrotik,l23ugsr-5haxd2haxd)` case added.
   > Confirm the NAND "kernel" partition is named `kernel` in the DTS (it is, see §2/§3).

### 1e. Required DEVICE_PACKAGES / DEVICE_VARS

- ipq50xx already does `DEVICE_VARS += BOOT_SCRIPT` at the top of `ipq50xx.mk` (only used by
  glinet); irrelevant here.
- From the MikroTik side, `DEVICE_PACKAGES := kmod-ledtrig-gpio` (LED triggers) is typical.
- From the ipq50xx side (the part that actually matters), the **ath11k firmware + per-board
  BDF**:
  - `ath11k-firmware-ipq5018-qcn6122` — pulls the IPQ5018 2.4G + QCN6122 5G firmware set
    (`ath11k/IPQ5018/hw1.0/*` and `ath11k/QCN6122/hw1.0/*`). This is exactly the package the
    QCN6122-based boards use (yuncore_ax830, cmcc_mr3000d-ci, linksys_mx2000, …).
  - `ipq-wifi-mikrotik_l23ugsr-5haxd2haxd` — the board-specific `board-2.bin`/BDF package
    (see §6, novel risk). Every QCN6122 ipq50xx board ships one
    (`ipq-wifi-yuncore_ax830`, `ipq-wifi-cmcc_mr3000d-ci`, …).
- `KERNEL_UBIFS_OPTS = -m $(PAGESIZE) -e 124KiB -c $(PAGESIZE) -x none` (from hAP ac3) is
  **required** for `package-kernel-ubifs`.

---

## 2. MikroTik RouterBOOT / caldata DTS pattern (the MikroTik half, DTS side)

Source: `target/linux/ipq40xx/dts/qcom-ipq4019-hap-ac3.dts`. Two flash chips: a small **NOR**
holding RouterBOOT + hard_config/soft_config, and the big **NAND** holding kernel + ubi.

### 2a. RouterBOOT partitions on NOR (dynamic parser + hard_config nvmem)

```dts
flash@0 {
	reg = <0>;
	compatible = "jedec,spi-nor";
	spi-max-frequency = <24000000>;

	partitions {
		compatible = "fixed-partitions";
		#address-cells = <1>;
		#size-cells = <1>;

		partition@0 {
			label = "Qualcomm";
			reg = <0x0 0x80000>;
			read-only;
		};

		partition@80000 {
			compatible = "mikrotik,routerboot-partitions";
			#address-cells = <1>;
			#size-cells = <1>;
			label = "RouterBoot";
			reg = <0x80000 0x80000>;

			hard_config {
				read-only;
				size = <0x2000>;

				nvmem-layout {
					compatible = "mikrotik,routerboot-nvmem";

					macaddr_hard: base-mac-address {
						#nvmem-cell-cells = <1>;
					};
				};
			};

			dtb_config {
				read-only;
			};

			soft_config {
			};
		};
	};
};
```

Mechanics (from `target/linux/generic/files/drivers/mtd/parsers/routerbootpart.c`):
- The outer `partition@80000` carries `compatible = "mikrotik,routerboot-partitions"`; the
  `routerbootpart` parser (matched at line 350 `{ .compatible = "mikrotik,routerboot-partitions" }`)
  walks the segment at `RB_BLOCK_SIZE` (0x1000) intervals looking for magics `Hard`/`Soft`/FDT
  and registers `hard_config`, `soft_config`, `dtb_config` as **dynamic** sub-partitions —
  their offsets are discovered at runtime, not hard-coded. So we do **not** need exact offsets
  for hard/soft/dtb config (only `size = <0x2000>` for hard_config, copied verbatim).
- `hard_config`'s `nvmem-layout { compatible = "mikrotik,routerboot-nvmem"; }` is handled by
  `rb_nvmem.c` (matched at `rb_nvmem.c:212` `{ .compatible = "mikrotik,routerboot-nvmem" }`).
  It TLV-parses hard_config and exposes the `base-mac-address` cell (label from `rb_nvmem.c:30`).
  Consumers reference `<&macaddr_hard N>` to get base+N MACs.

### 2b. MAC address consumption (no `0:ART`, no `mtd_get_mac_*`)

```dts
&gmac {
	status = "okay";
	nvmem-cells = <&macaddr_hard 0>;
	nvmem-cell-names = "mac-address";
};
&swport1 { …; nvmem-cells = <&macaddr_hard 4>; nvmem-cell-names = "mac-address"; };
…
```

i.e. MACs come from the RouterBOOT hard_config nvmem cell, **base + offset**.

### 2c. WLAN caldata on MikroTik = sysfs `wlan_data`, not an MTD ART partition

This is the crux. ipq40xx MikroTik ath10k boards pull caldata out of the `rb_hardconfig`
**sysfs** node, not a flash partition. From
`target/linux/ipq40xx/base-files/etc/hotplug.d/firmware/11-ath10k-caldata`:

```sh
mikrotik,cap-ac|mikrotik,hap-ac2|mikrotik,hap-ac3|…)
	wlan_data="/sys/firmware/mikrotik/hard_config/wlan_data"
	( [ -f "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data" 0x0 0x2f20 ) || \
	( [ -d "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data/data_0" 0x0 0x2f20 )
	;;
```

`rb_hardconfig.c` exposes `/sys/firmware/mikrotik/hard_config/wlan_data` (driver header
comment lines 11-19), optionally LZO/LZ77/LZOR-decompressed (`MIKROTIK_WLAN_DECOMPRESS_LZ77`,
`hc_lzor_prefix[]`). Newer boards expose a **directory** `wlan_data/` with per-radio files
`data_0`, `data_1`, `data_2` … (the multi-tag ERD scheme, `RB_WLAN_ERD_ID_MULTI_8001/8201`
in `rb_hardconfig.c:71-73`).

→ The L23 cannot use `caldata_extract "0:ART" …` like AX830/MR3000D-CI. It must use
`caldata_sysfsload_from_file` against `wlan_data`. See §6 for why ath11k makes this harder
than ath10k.

---

## 3. ipq5018 + QCN6122 radio + ethernet DTS pattern (the SoC half)

Closest single-radio-twin reference: `target/linux/qualcommax/dts/ipq5018-mr3000d-ci.dts`
(IPQ5018 2.4G + QCN6102 5G, exactly our radio combo). Base SoC nodes live in the in-kernel
`ipq5018.dtsi` and OpenWrt's `ipq5018-ess.dtsi` + `ipq5018-qcn6122.dtsi`.

### 3a. Includes

```dts
#include "ipq5018.dtsi"
#include "ipq5018-ess.dtsi"
#include "ipq5018-qcn6122.dtsi"
```

`ipq5018-qcn6122.dtsi` defines the 5G radio `wifi1: wifi@b00a040` (`compatible =
"qcom,qcn6122-wifi"`, AHB), remaps the remoteproc PDs (`q6_wcss_pd1` = IPQ5018, `q6_wcss_pd2`
= QCN6122), and grows `&q6_region` to `0x3000000`.

### 3b. The two radios

```dts
&wifi {                                  /* IPQ5018 2.4G, wifi@c000000 */
	status = "okay";
	qcom,rproc = <&q6_wcss_pd1>;
	qcom,ath11k-calibration-variant = "MikroTik-L23UGSR";
	qcom,ath11k-fw-memory-mode = <1>;
	qcom,bdf-addr = <0x4c400000>;
};
&wifi1 {                                 /* QCN6102 5G, wifi@b00a040 */
	status = "okay";
	qcom,rproc = <&q6_wcss_pd2>;
	qcom,userpd-subsys-name = "q6v5_wcss_userpd2";
	qcom,ath11k-calibration-variant = "MikroTik-L23UGSR";
	qcom,ath11k-fw-memory-mode = <1>;
	qcom,bdf-addr = <0x4d100000>;
	qcom,m3-dump-addr = <0x4df00000>;
};
```

QCN6102 reset GPIO is passed via the remoteproc boot-args (MR3000D-CI / MX2000 use GPIO 15):

```dts
&q6v5_wcss {
	status = "okay";
	/* QCN6102 → UPD ID 2 (firmware default would be 3) */
	boot-args = <
		/* type:       */ 0x1   /* PCIE0 */
		/* length:     */ 4
		/* UPD ID:     */ 2
		/* reset GPIO: */ 15
		/* reserved:   */ 0 0>;
};
```

The `wifi@c000000` (2.4G) and `wifi@b00a040`/`wifi@b00b040` (QCN6122 #1/#2) nodes themselves
come from `ipq5018.dtsi` and `ipq5018-qcn6122.dtsi`; the board DTS only sets `status` +
`qcom,rproc` + calibration-variant + bdf-addr. Firmware path bindings (the part the task
called out): from `ipq5018-qcn6122.dtsi`:
```dts
firmware-name = "ath11k/IPQ5018/hw1.0/q6_fw.mdt",
		"ath11k/IPQ5018/hw1.0/m3_fw.mdt",
		"ath11k/QCN6122/hw1.0/m3_fw.mdt";
```

### 3c. Ethernet / MDIO / PHY

IPQ5018 has two GMAC datapaths (`dp1`,`dp2` from `ipq5018-ess.dtsi`) and two MDIO buses
(`mdio0` @0x88000 with the **internal GE phy at addr 7**, `mdio1` @0x90000 for external PHYs).
From `ipq5018.dtsi`:
```dts
mdio0: mdio@88000 { … ge_phy: ethernet-phy@7 {
	compatible = "ethernet-phy-id004d.d0c0"; reg = <7>;
	resets = <&gcc GCC_GEPHY_MISC_ARES>; }; };
mdio1: mdio@90000 { … };
```

**ether1 (1G copper)** = MAC0 → internal GE phy, exactly the AX830 `dp1` pattern:
```dts
&switch {
	status = "okay";
	switch_mac_mode = <MAC_MODE_SGMII_CHANNEL0>;
	qcom,port_phyinfo {
		port@1 { port_id = <1>; mdiobus = <&mdio0>; phy_address = <7>; };
		port@2 { port_id = <2>; /* … uniphy/SGMII for the 2.5G side … */ };
	};
};
&dp1 {
	status = "okay";
	label = "lan";                              /* L23: ether1 */
	nvmem-cells = <&macaddr_hard 0>;            /* RouterBOOT MAC, not 0:ART */
	nvmem-cell-names = "mac-address";
	phy-mode = "sgmii";
};
&mdio0 { status = "okay"; };
```

**2.5G SFP** = MAC1 → uniphy → SFP cage. **This is the least certain part of the board.**
The IPQ5018 uniphy on `dp2` is normally driven at 1G SGMII (AX830 uses a QCA8081 2.5G PHY on
`mdio1` addr 28 with `phy-mode="sgmii"`; MX2000 uses a fixed 1G SGMII link to a QCA8337).
For an **SFP** the closest in-tree pattern is the ipq807x `sfp` + `sfp_i2c` node set
(`ipq8074-rt-ax89x.dts` LEDs/gpios `sfp_tx_disable`/`sfp_mod_def0`, and the `usxgmii`/
`2500base-x` phy-modes in `ipq8074-nbg7815.dts`/`ipq8072-301w.dts`). For 2.5G the link is
`2500base-x` (or `sgmii` if the SFP is forced to 1G). This must be confirmed on hardware
(SFP I2C bus, tx-disable/los/mod-def GPIOs, whether the MAC runs 1G-SGMII or 2.5G-2500BASE-X).
The draft DTS encodes `dp2` + an `sfp`/`sfp-eth` node with everything marked `TODO(recon)`.

### 3d. NAND (kernel + ubi) and NOR (RouterBOOT) coexistence

The L23 has both flashes (like hAP ac3): NOR on `&blsp1_spi1` for RouterBOOT (§2a), NAND on
`&qpic_nand`/`nand@0` for kernel+ubi. The NAND node uses the ipq50xx spi-nand binding
(AX830 pattern), but **fixed-partitions** `kernel` + `ubi` (hAP ac3 pattern) instead of
`qcom,smem-part`, because RouterBOOT's NAND layout is MikroTik's, not Qualcomm SMEM's:
```dts
&qpic_nand {
	pinctrl-0 = <&qpic_pins>; pinctrl-names = "default";
	status = "okay";
	nand@0 {
		compatible = "spi-nand"; reg = <0>;
		nand-ecc-engine = <&qpic_nand>; nand-bus-width = <8>;
		partitions {
			compatible = "fixed-partitions";
			#address-cells = <1>; #size-cells = <1>;
			partition@0  { label = "kernel"; reg = <0x0       0xa00000>; };   /* TODO(recon) size */
			partition@a00000 { label = "ubi"; reg = <0xa00000 0x7600000>; };  /* TODO(recon) size */
		};
	};
};
```

---

## 4. Kernel CONFIG symbols to add for ipq50xx

ipq50xx today does **not** build the MikroTik platform glue. Generic defaults
(`target/linux/generic/config-6.12`) have them **off**:
```
# CONFIG_MIKROTIK is not set
# CONFIG_MTD_ROUTERBOOT_PARTS is not set
# CONFIG_NVMEM_LAYOUT_MIKROTIK is not set
```
The ipq40xx MikroTik subtarget turns them on (`target/linux/ipq40xx/mikrotik/config-default`).
For ipq50xx, add to `target/linux/qualcommax/ipq50xx/config-default` (or the subtarget config):

```
CONFIG_MIKROTIK=y
CONFIG_MIKROTIK_RB_SYSFS=y                 # routerboot.o rb_hardconfig.o rb_softconfig.o → /sys/firmware/mikrotik/*
CONFIG_MIKROTIK_WLAN_DECOMPRESS_LZ77=y     # rb_lz77.o — decompress LZ77/LZOR factory caldata
CONFIG_NVMEM_LAYOUT_MIKROTIK=y             # rb_nvmem.o — hard_config "base-mac-address" nvmem cell
CONFIG_MTD_ROUTERBOOT_PARTS=y              # routerbootpart.c — dynamic hard/soft/dtb_config parser
```

Symbol → file map (from `…/platform/mikrotik/Makefile` and `…/platform/mikrotik/Kconfig`):
- `MIKROTIK_RB_SYSFS` → `routerboot.o rb_hardconfig.o rb_softconfig.o`  (selects `LZO_DECOMPRESS`, `CRC32`)
- `NVMEM_LAYOUT_MIKROTIK` → `rb_nvmem.o`  (depends on `NVMEM_LAYOUTS`, already `=y` in qualcommax)
- `MIKROTIK_WLAN_DECOMPRESS_LZ77` → `rb_lz77.o`  (depends on `MIKROTIK_RB_SYSFS`)
- `MTD_ROUTERBOOT_PARTS` → `drivers/mtd/parsers/routerbootpart.c`

Already present in qualcommax `config-6.12` (no action): `CONFIG_NVMEM_LAYOUTS=y`,
`CONFIG_MTD_NAND_QCOM=y`, `CONFIG_MTD_SPI_NAND=y` (ipq50xx `config-default`),
`CONFIG_MTD_UBI=y`, `CONFIG_UBIFS_FS=y`, `CONFIG_QCOM_Q6V5_MPD=y` (ath11k MPD remoteproc).
Note ipq50xx keeps `CONFIG_MTD_QCOMSMEM_PARTS=y` — harmless; the L23 NAND uses
fixed-partitions, the NOR uses routerbootpart.

---

## 5. Caldata hotplug adaptation for ipq50xx (ath11k from RouterBOOT)

The stock ipq50xx hook
(`target/linux/qualcommax/ipq50xx/base-files/etc/hotplug.d/firmware/11-ath11k-caldata`)
keys every board off an MTD ART partition, e.g.:
```sh
yuncore,ax830)
	caldata_extract "0:ART" 0x1000 0x20000          # 2.4G blob
	…
"ath11k/QCN6122/hw1.0/cal-ahb-b00a040.wifi.bin")
	yuncore,ax830)
		caldata_extract "0:ART" 0x4c000 0x20000     # QCN6122 5G blob
```
The L23 has **no ART partition**. The adaptation marries the ipq50xx `FIRMWARE` case labels
(the ath11k blob names) to the ipq40xx MikroTik **sysfs** source. Draft additions:

```sh
"ath11k/IPQ5018/hw1.0/cal-ahb-c000000.wifi.bin")      # 2.4G IPQ5018
	mikrotik,l23ugsr-5haxd2haxd)
		wlan_data="/sys/firmware/mikrotik/hard_config/wlan_data"
		# TODO(recon): confirm single-file vs dir, and which data_N is the 2.4G blob,
		# and the correct length (ath11k blob is ~0x20000, NOT ath10k's 0x2f20).
		( [ -d "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data/data_0" 0x0 0x20000 ) || \
		( [ -f "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data" 0x0 0x20000 )
		# CONFIRMED base MAC F4:1E:57:9F:F5:00; radio1(2.4G)=base+2:
		ath11k_patch_mac $(macaddr_add F4:1E:57:9F:F5:00 2) 0   # TODO(recon): derive base from nvmem, not literal
		ath11k_set_macflag
		;;
	;;
"ath11k/QCN6122/hw1.0/cal-ahb-b00a040.wifi.bin")      # 5G QCN6102
	mikrotik,l23ugsr-5haxd2haxd)
		wlan_data="/sys/firmware/mikrotik/hard_config/wlan_data"
		( [ -d "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data/data_1" 0x0 0x20000 ) || \
		( [ -f "$wlan_data" ] && caldata_sysfsload_from_file "$wlan_data" <off> 0x20000 )
		# CONFIRMED radio2(5G)=base+3:
		ath11k_patch_mac $(macaddr_add F4:1E:57:9F:F5:00 3) 0   # TODO(recon): derive base from nvmem
		ath11k_set_macflag
		;;
	;;
```
`caldata_sysfsload_from_file` is provided by `/lib/functions/caldata.sh` (sourced at the top
of the hook). The ath10k MikroTik path proves the sysfs source works; what is unknown is the
**ath11k blob layout inside MikroTik's wlan_data** (see §6).

---

## 6. THE NOVEL RISKS — call-outs

### 6.0 AArch32 vs AArch64 boot hand-off (NEW, from live recon)

Live recon (`RECON-LIVE-ROUTEROS.md`, box 192.168.0.134) reports
`architecture-name = arm` — RouterOS runs a **32-bit ARM userland**. But
`qualcommax/ipq50xx` is **aarch64-only** (Cortex-A53 64-bit kernel). It is unverified whether
MikroTik RouterBOOT 7.16.1 `bootelf`s the netbooted kernel in **AArch64** (the common "64-bit
kernel + 32-bit userland" arrangement on 256MB ARMv8 — in which case our aarch64 ELF boots
fine) or strictly **AArch32** (in which case this entire ipq50xx/aarch64 approach cannot boot
and a different strategy is needed). This is resolved empirically the first time the initramfs
ELF is netbooted over serial — and it gates everything. Treat as the #1 unknown.

### 6.1 ath11k caldata/BDF from RouterBOOT (vs raw 0:ART)

ath10k (ipq40xx MikroTik) needs only a per-radio `pre-cal`/`cal` blob (`0x2f20` bytes) and a
generic `board.bin`. The hotplug just copies `wlan_data` → `/lib/firmware/…/pre-cal-*.bin`
and patches the MAC. ath11k is materially different and this is the top porting risk:

1. **ath11k needs a board-specific BDF (`board-2.bin`) AND per-radio cal blobs.** On the
   reference ipq50xx boards the BDF ships as `ipq-wifi-<board>` (a package that drops
   `board-2.bin` into `/lib/firmware/ath11k/IPQ5018/hw1.0/` and `…/QCN6122/hw1.0/`) and is
   selected by `qcom,ath11k-calibration-variant`. MikroTik does **not** ship an OpenWrt BDF;
   we must **create `ipq-wifi-mikrotik_l23ugsr-5haxd2haxd`** with a board-2.bin whose
   `bus=ahb,qmi-chip-id=…,qmi-board-id=…,variant=MikroTik-L23UGSR` rows match what the L23's
   ath11k firmware requests. The qmi-board-id is read off the live box (`dmesg | grep -i
   "qmi-board-id\|bdf\|board_id"`). **Until that BDF exists and matches, neither radio
   initialises.**

2. **The cal blob format/offset inside MikroTik `wlan_data` is unknown for ath11k.** For
   ath10k MikroTik, `wlan_data` (or `data_0`/`data_2`) maps cleanly to the ath10k pre-cal
   structure at `0x0`/`0x2f20`/`0x8000` etc. For ath11k the per-radio cal blob is ~0x20000
   and the IPQ5018+QCN6122 split lives at distinct offsets on the reference boards
   (2.4G `0x1000`, QCN6122 `0x4c000`/`0x26800`). Whether MikroTik even stores ath11k-format
   cal data in hard_config — or stores its own RouterOS calibration that ath11k cannot
   consume directly — is **unverified**. Possible outcomes, in increasing pain:
   - (best) `wlan_data/data_0`,`data_1` are usable ath11k cal blobs → §5 works as drafted.
   - (medium) blobs need LZ77/LZOR decompression (already covered by
     `MIKROTIK_WLAN_DECOMPRESS_LZ77`) and/or an offset/length we must reverse from a dump.
   - (worst) RouterOS calibration is in a format ath11k rejects, requiring extraction +
     reformat, or sourcing cal data another way. **This is the single biggest unknown that
     can block RF bring-up even after the box boots.**

3. **MAC sourcing differs.** Reference boards do `ath11k_patch_mac $(macaddr_add <label> N)`
   where `<label>` comes from `0:ART`/`devinfo`/`0:appsblenv`. On L23 the base MAC is the
   RouterBOOT `macaddr_hard` nvmem cell; the per-radio offset N is unknown (recon).

Mitigation order: (a) get the box netbooting initramfs first (no caldata needed to boot);
(b) dump `/sys/firmware/mikrotik/hard_config/*` and `dmesg` to learn the BDF/board-id and the
wlan_data layout; (c) build the `ipq-wifi-` BDF; (d) finalise the caldata hook offsets.

---

## 7. board.d / network userspace

- `target/linux/qualcommax/ipq50xx/base-files/etc/board.d/02_network` — add a
  `mikrotik,l23ugsr-5haxd2haxd)` case. With one copper + one SFP this is most likely
  `ucidef_set_interfaces_lan_wan "lan" "wan"` (ether1=lan, sfp=wan) — but the role mapping is
  a product decision; the AX830 single-LAN+WAN case is the closest precedent.
- `…/board.d/01_leds` — add LED netdev triggers once LED gpios are known (recon).
- `…/lib/upgrade/platform.sh` — add `platform_do_upgrade_mikrotik_nand` (copy from ipq40xx)
  and a `mikrotik,l23ugsr-5haxd2haxd)` dispatch (see §1d).
- Optionally `…/init.d/bootcount` — MikroTik RouterBOOT has no s_env bootcount, so leave the
  L23 out of that list.

---

## 8. Quick reference table

| Concern                      | Take from                              | File |
|------------------------------|----------------------------------------|------|
| Image recipe (ELF netboot + UBI NAND) | `Device/mikrotik_nand` / `mikrotik_hap-ac3` | `ipq40xx/image/mikrotik.mk` |
| append-dtb-elf / ubinize-kernel | image macros                        | `include/image-commands.mk:19,30,227` |
| RouterBOOT NOR partitions + hard_config nvmem | hAP ac3 NOR `flash@0`        | `ipq40xx/dts/qcom-ipq4019-hap-ac3.dts` |
| Dynamic partition parser     | routerbootpart                         | `generic/files/drivers/mtd/parsers/routerbootpart.c` |
| hard_config nvmem MAC cell   | rb_nvmem                               | `generic/files/drivers/platform/mikrotik/rb_nvmem.c` |
| wlan_data sysfs caldata source | hAP ac3 ath10k hook                  | `ipq40xx/base-files/etc/hotplug.d/firmware/11-ath10k-caldata` |
| ipq5018 2.4G + QCN6102 5G radios | MR3000D-CI                          | `qualcommax/dts/ipq5018-mr3000d-ci.dts` + `ipq5018-qcn6122.dtsi` |
| ipq5018 eth/mdio/ge_phy/dp1/dp2 | ess + AX830                         | `qualcommax/files/.../ipq5018-ess.dtsi`, `ipq5018-ax830.dts` |
| NAND spi-nand node (W25N01GW) | AX830                                 | `qualcommax/dts/ipq5018-ax830.dts` |
| ath11k caldata offsets (ART reference) | ipq50xx ath11k hook           | `qualcommax/ipq50xx/.../11-ath11k-caldata` |
| Kernel CONFIG for RouterBOOT | ipq40xx mikrotik config                | `ipq40xx/mikrotik/config-default` |
| NAND sysupgrade flow         | platform.sh mikrotik_nand              | `ipq40xx/base-files/lib/upgrade/platform.sh` |
