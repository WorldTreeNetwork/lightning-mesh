# L23UGSR-5HaxD2HaxD hardware recon checklist

Goal: extract every fact needed to resolve the `TODO(recon)` placeholders in
`ipq5018-mikrotik-l23ugsr.dts`, `device-recipe.mk`, and the base-files hooks.

Two recon phases:
- **Phase A — RouterOS (stock):** the box boots RouterOS. Use its console/Winbox to read
  RouterBOOT info and partition/MAC/SFP facts non-destructively.
- **Phase B — OpenWrt initramfs over serial:** netboot the initramfs ELF (no flash writes)
  and read Linux's view (`/proc/mtd`, `/sys/firmware/...`, `dmesg`). This is where the
  ath11k/BDF unknowns get resolved.

Serial console: 115200 8N1 on the IPQ5018 UART (the draft assumes `blsp1_uart1` on
gpio20/gpio21 — confirm the physical header/TTL pinout first).

---

## Phase A — from RouterOS

> STATUS: Phase A is **already captured** for box 192.168.0.134 — see `RECON-LIVE-ROUTEROS.md`.
> Confirmed: model `L23UGSR-5HaxD2HaxD`, firmware-type `ipq5000`, RouterBOOT 7.16.1,
> 256MB RAM / 128MB NAND, base MAC `F4:1E:57:9F:F5:00` (ether1=+0, sfp1=+1, radio1/2.4G=+2,
> radio2/5G=+3), sfp1 = 2.5G-baseT/baseX capable, boot-protocol `bootp`.
> One open flag from Phase A: RouterOS userland is **32-bit ARM** — verify the AArch64 vs
> AArch32 boot hand-off at first serial netboot (Phase B, see B9 / MAPPING §6.0).
> The table below is retained for completeness / re-runs on other units.

| # | Fact | Command / source | Fills |
|---|------|------------------|-------|
| A1 | RouterBOOT version, model, serial, "factory firmware" | `/system routerboard print` | sanity / model id confirm |
| A2 | Board name & revision | `/system resource print` ; `/system routerboard print` | `compatible`, model |
| A3 | Base MAC address | `/interface ethernet print detail` (lowest MAC) ; sticker | caldata MAC base offsets, `macaddr_hard` sanity |
| A4 | Ethernet port naming/count (ether1 + sfp/sfp-sfpplus1) | `/interface ethernet print` | `dp1`/`dp2` labels, board.d roles |
| A5 | SFP presence, speed, type | `/interface ethernet print detail` for sfp1 (rate, sfp-... fields) | `dp2` phy-mode (sgmii vs 2500base-x), `forced-speed` |
| A6 | LED names/roles | `/system leds print` | LED `function`/`color` mapping |
| A7 | Wireless interfaces / bands | `/interface wireless print` (or `/interface wifi`) | confirm 2.4G=IPQ5018, 5G=QCN6102 |
| A8 | "hard config" dump if accessible | RouterOS does not expose it directly — defer to B6 | wlan_data layout |

> Phase A is mostly confirmation. The load-bearing facts (partitions, GPIOs, caldata) come
> from Phase B.

---

## Phase B — from a booted OpenWrt initramfs (serial)

Build + netboot:
`make ... ` → `bin/targets/qualcommax/ipq50xx/openwrt-...-mikrotik_l23ugsr-5haxd2haxd-initramfs-kernel.bin`,
serve via TFTP, set RouterBOOT boot-device to ethernet/bootp (Phase A: `/system routerboard
settings set boot-device=try-ethernet-once-then-nand boot-protocol=bootp`).

### B1. Flash partition map  → DTS NOR + NAND `reg`s, `chosen` ubiblock
```
cat /proc/mtd
cat /sys/class/mtd/mtd*/name 2>/dev/null
dmesg | grep -iE 'spi-nor|spi-nand|mtd|partition|routerboot|W25N01|winbond|qpic'
ls -l /dev/mtd*
```
Fills: `&blsp1_spi1/flash@0/partitions` (RouterBoot loader + RouterBoot segment offsets/sizes),
`&qpic_nand/nand@0/partitions` (`kernel`, `ubi` sizes), and the `root=/dev/ubiblockN_M` index
in `chosen` (confirm after the FIRST NAND install with `cat /proc/cmdline` + `ubinfo -a`).

### B2. RouterBOOT dynamic partitions registered  → confirm parser works
```
dmesg | grep -iE 'routerbootpart|hard_config|soft_config|dtb_config'
cat /proc/mtd      # expect hard_config / soft_config to appear as their own mtdN
```
Fills: confirms `compatible = "mikrotik,routerboot-partitions"` segment reg is correct (the
parser found the magics). If they don't appear, the `RouterBoot` `reg = <0x80000 0x80000>`
offset is wrong — adjust from B1.

### B3. hard_config sysfs + nvmem MAC  → DTS nvmem MAC, caldata MAC base
```
ls -l /sys/firmware/mikrotik/hard_config/
cat /sys/firmware/mikrotik/hard_config/name 2>/dev/null
cat /sys/firmware/mikrotik/hard_config/mac_base 2>/dev/null   # base MAC
hexdump -C /sys/firmware/mikrotik/hard_config/* 2>/dev/null | head
# nvmem cell exposed by rb_nvmem.c:
cat /sys/bus/nvmem/devices/*/nvmem 2>/dev/null | hexdump -C | head
```
Fills: confirms `macaddr_hard: base-mac-address` resolves; the base MAC for
`<&macaddr_hard N>` and for `ath11k_patch_mac`. Determine the per-interface offset N by
comparing `mac_base` to the actual ether1/sfp/wlan MACs (Phase A A3/A7).

### B4. GPIO / LED / button mapping  → DTS leds{}, keys{}, &tlmm pin groups
```
cat /sys/kernel/debug/gpio                      # all gpio lines + current state
# Toggle candidate LEDs to identify them physically:
echo 46 > /sys/class/gpio/export; echo out > /sys/class/gpio/gpio46/direction
echo 1 > /sys/class/gpio/gpio46/value           # watch which LED lights
dmesg | grep -iE 'gpio-keys|tlmm|pinctrl'
# Press the reset/mode button and watch:
evtest /dev/input/event0    # or: cat /sys/kernel/debug/gpio while pressing
```
Fills: real `gpios = <&tlmm N ...>` for `led_user`/`wlan2g`/`wlan5g`, the reset/mode button
GPIO + active level, and the `&tlmm` `button_pins`/LED pin groups. Cross-check against the
IPQ5018 pinmux to pick the right `function`.

### B5. Ethernet PHY / MDIO / SFP topology  → DTS &switch, &dp1, &dp2, &mdio1, sfp{}
```
dmesg | grep -iE 'mdio|phy|ge_phy|qca808|stmmac|nss-dp|uniphy|sgmii|2500|sfp'
ls /sys/class/mdio_bus/*/                        # which mdio buses + phy addrs
cat /sys/class/net/*/address
ethtool ether1 ; ethtool sfp 2>/dev/null         # link mode, speeds
ls /sys/class/i2c-adapter/                        # SFP I2C bus discovery
dmesg | grep -iE 'sfp|sff'                        # sfp cage detection if driver loaded
```
Fills: whether the 2.5G side is a copper PHY on `mdio1` (→ declare `ethernet-phy@addr` +
`phy-handle`) or a bare SFP cage (→ `sfp{}` node + I2C bus + tx-disable/los/mod-def GPIOs),
the `dp2` phy-mode (`sgmii` 1G vs `2500base-x`), `switch_mac_mode`, and any PHY reset GPIO.
**This resolves the biggest eth uncertainty (2.5G SFP).**

### B6. ath11k radios + BDF/caldata  → caldata hook, ipq-wifi BDF pkg (NOVEL RISK)
```
dmesg | grep -iE 'ath11k|qcn6122|qcn6102|ipq5018|bdf|board-2|qmi|board_id|cal'
# The critical board-id the BDF must match:
dmesg | grep -iE 'qmi-board-id|qmi-chip-id|board_id|fallback board'
# Inspect MikroTik wlan caldata:
ls -l /sys/firmware/mikrotik/hard_config/wlan_data*    # file vs directory (data_0/data_1/...)
hexdump -C /sys/firmware/mikrotik/hard_config/wlan_data 2>/dev/null | head -40
for f in /sys/firmware/mikrotik/hard_config/wlan_data/*; do echo "== $f =="; \
    hexdump -C "$f" | head -8; wc -c < "$f"; done 2>/dev/null
# What firmware files ath11k actually requested:
dmesg | grep -iE 'firmware: (direct|failed)|cal-ahb|board-2.bin'
```
Fills:
- The `qcom,ath11k-calibration-variant` string + the **`ipq-wifi-mikrotik_l23ugsr` board-2.bin**
  rows (`qmi-board-id`, `qmi-chip-id`, `bus=ahb`, `variant=...`). Without a matching BDF,
  neither radio comes up — build the BDF before expecting RF.
- Whether `wlan_data` is a single file or a directory of `data_N` blobs, the per-radio
  index (2.4G vs 5G), the blob **length** (ath11k ≈ 0x20000, NOT ath10k's 0x2f20), and whether
  LZ77/LZOR decompression is applied (driver does it if `MIKROTIK_WLAN_DECOMPRESS_LZ77=y`).
- The exact `caldata_sysfsload_from_file` args for the `11-ath11k-caldata` cases
  (`cal-ahb-c000000.wifi.bin` for 2.4G, `cal-ahb-b00a040.wifi.bin` for 5G).
- Per-radio MAC offset for `ath11k_patch_mac $(macaddr_add <base> N)`.

> If `dmesg` shows ath11k loading firmware but failing to find/parse cal data, dump the raw
> wlan_data blob to a host and compare its structure to a known ath11k cal blob (e.g. from
> Yuncore AX830 `0:ART` 0x1000) to learn the format/offset. Worst case: MikroTik's RouterOS
> calibration is not ath11k-consumable and must be extracted/reformatted — escalate.

### B7. QCN6102 reset GPIO + remoteproc  → DTS &q6v5_wcss boot-args
```
dmesg | grep -iE 'q6v5|wcss|remoteproc|userpd|mpd|spawn'
cat /sys/class/remoteproc/*/state
```
Fills: confirms `boot-args` UPD ID 2 and the QCN6102 **reset GPIO** (draft guesses 15 from
MR3000D-CI/MX2000). If the 5G radio never spawns, the reset GPIO or UPD ID is wrong.

### B8. Serial/UART pinmux confirmation  → DTS serial_0_pins
```
dmesg | grep -iE 'blsp.*uart|ttyMSM|serial'
cat /sys/firmware/devicetree/base/chosen/stdout-path 2>/dev/null
```
Fills: confirms `serial_0_pins` pins/function (gpio20/21 `blsp0_uart0` vs gpio28/29
`blsp0_uart1`). If you already have serial output you implicitly have this.

### B9. RAM / SoC sanity  → memory node, SOC confirm
```
cat /proc/meminfo | head -1            # expect ~256MB
cat /proc/device-tree/model 2>/dev/null
dmesg | grep -iE 'ipq5018|ipq5010|maple|cpu0|cortex-a53'
```
Fills: confirms 256MB and the IPQ5018-class SoC (the `memory { reg = <... 0x10000000> }` and
SOC string). The in-tree `ipq5018.dtsi` already sets memory; verify size matches.

---

## TODO(recon) → command index (quick map)

| DTS / file field | Recon item |
|------------------|-----------|
| NOR `flash@0` partition reg/sizes, "RouterBoot" segment | B1, B2 |
| `hard_config` size, `macaddr_hard` resolves | B2, B3 |
| `chosen` `root=/dev/ubiblockN_M` | B1 (post-install) |
| NAND `kernel`/`ubi` sizes | B1 |
| `dp1`/`dp2` labels + MAC nvmem offsets | A3, A4, B3, B5 |
| `dp2` phy-mode (sgmii vs 2500base-x), `forced-speed`, SFP cage | A5, B5 |
| `&mdio1` external PHY @addr + reset-gpios | B5 |
| `leds{}` gpios/colors/functions | A6, B4 |
| `keys{}` reset/mode gpio + level | B4 |
| `&tlmm` pin groups (button/mdio/serial) | B4, B8 |
| `&q6v5_wcss` boot-args reset GPIO / UPD ID | B7 |
| `qcom,ath11k-calibration-variant` + `ipq-wifi` BDF (board-2.bin) | B6 |
| `11-ath11k-caldata` sysfs offsets/lengths + MAC patch | B6, B3 |
| board.d `02_network` LAN/WAN roles | A4, A5 |
| `memory` size / SOC | B9 |
