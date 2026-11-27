# OpenWrt One and Heltec WiFi LoRa 32 V4

## OpenWrt One

| Hardware | Board specification |
|---|---|
| Processor and memory | MediaTek MT7981B (Filogic 820), dual-core Arm Cortex-A53, 1 GiB DDR4 |
| Flash and recovery | 256 MiB SPI NAND, 16 MiB SPI NOR, physical NAND/NOR boot selector |
| Wi-Fi | MediaTek MT7976C, dual-band Wi-Fi 6, 2×2 at 2.4 GHz and 3×3 at 5 GHz |
| Ethernet | One 2.5 GbE WAN and one 1 GbE LAN |
| Power | USB-C PD or WAN PoE with the PoE module installed |
| Expansion and service | mikroBUS socket, M.2 M-key PCIe NVMe (2230/2242), USB-A 2.0 host, USB-C serial console, JTAG |
| Clock | NXP PCF8563TS RTC with CR1220 backup cell holder |

Board drawings, connector details and recovery instructions: [Banana Pi hardware specification](https://docs.banana-pi.org/en/OpenWRT-One/BananaPi_OpenWRT-One) · [OpenWrt One HowTo](https://one.openwrt.org/hardware/OpenWrtOne-HowTo.pdf).

**Bench observation, 2026-11-26.** Ethernet SSH reached the router at `10.42.58.1` from `10.42.58.179`. Its board ID was `openwrt,one`, running OpenWrt 25.12.5 (`r33051-f5dae5ece4`, `mediatek/filogic`), kernel 6.12.94 and squashfs. The boot log registered the console at `/dev/ttyS0` (0x11002000) and the mikroBUS UART at `/dev/ttyS1` (0x11004000). Linux exposed `/sys/class/gpio/mikrobus-reset` for the socket reset line (GPIO2). The repository carries the [matching sysupgrade image and checksums](vendor-firmware/openwrt-one/).

## Heltec WiFi LoRa 32 V4

| Item | Bench identification, 2026-11-26 |
|---|---|
| MCU | ESP32-S3R2, QFN56, silicon revision v0.2, 40 MHz crystal |
| Memory | 2 MB embedded PSRAM, 16 MB quad SPI flash at 3.3 V (manufacturer 0x68, device 0x4018) |
| USB | Native USB Serial/JTAG, VID:PID `303a:1001`, enumerated as `/dev/ttyACM0` |
| Identity | MAC `f8:5b:1b:a3:be:f8`. Factory boot message `ESP32ChipID=F8BEA31B5BF8` |
| Radio identification | Factory image strings `HELTEC_WIFI_LORA_32_V4` and `RF_FREQUENCY:868_HP` identify the high-power SX1262 configuration, consistent with order code HTIT-WB32LAF |
| Factory sketch | Printed “IO Output test...” and “IO Output test end!” over USB |

The board family includes 2.4 GHz Wi-Fi, BLE, a 0.96-inch OLED and Li-Po charging. Heltec rates the high-power variant at 28 ± 1 dBm. The exact RF front end and usable band for this unit still need a PCB/variant-label check. R2 identifies the MCU memory variant. PCB V4.2 uses a GC1109 front end and V4.3 uses a KCT8103L. The factory build setting `LORA_FEM_TYPE=1` does not resolve the board revision. [Heltec specifications and pin map](https://heltec.org/project/wifi-lora-32-v4/) · [Hardware revision log](https://wiki.heltec.org/docs/devices/open-source-hardware/esp32-series/lora-32/wifi-lora-32-v4/hardware-update-log).

## Proposed mikroBUS UART connection

The ESP32 controls the SX1262 and can forward received LoRa records to the router over 3.3 V UART. Use jumpers to Heltec J2 on the USB side of the OLED. Locate pin 1 by the GND marking next to USB. Match both boards' signal labels before applying power.

| OpenWrt One socket | Heltec J2 | Purpose |
|---|---|---|
| GND | 1 GND | Common reference |
| TX | 5 GPIO44 / U0RXD | Router → Heltec |
| RX | 6 GPIO43 / U0TXD | Heltec → router |
| 5V0 | 2 5V | Optional router power, only with Heltec USB power disconnected |

For USB-powered bring-up use just GND, TX and RX. Leave 3V3, Ve (switched output), reset, interrupt, SPI and I²C disconnected. The intended serial setting is 115200 8N1. The factory sketch was observed on USB only. End-to-end forwarding requires Heltec firmware emitting records on hardware UART0 and a Lightning Mesh reader on the router. Header traffic and LoRa-to-mesh forwarding have yet to be demonstrated.
