# L23UGSR-5HaxD2HaxD — live-box recon (from RouterOS, via REST)

Box: `192.168.0.134` (mgmt, DHCP lease on default `bridge`), `admin` / `lab`.
REST over **HTTP** works (`curl -s -u admin:lab http://192.168.0.134/rest/...`);
`www-ssl` has no cert → HTTPS unusable. SSH (22) also open.

## Identity
| field | value |
|---|---|
| model | `L23UGSR-5HaxD2HaxD` |
| serial | `HHH0ABNCNAP` |
| firmware-type | `ipq5000` (→ IPQ5010 / IPQ5018-class) |
| RouterBOOT | current 7.16.1, factory 7.14.2 |
| RouterOS | 7.18.2 stable |
| board-name | `L23UGSR-5HaxD2HaxD` |

## CPU / mem / flash
| field | value |
|---|---|
| architecture-name | **`arm` (32-bit userland)** ⚠ verify boot hand-off mode |
| cpu | ARM, 2 cores, 800 MHz |
| total-memory | 268435456 (256 MB) |
| total-hdd (NAND) | 134217728 (128 MB), bad-blocks 0 |

> ⚠ **Pivotal risk:** RouterOS userland is 32-bit ARM. OpenWrt `qualcommax/ipq50xx`
> is **aarch64-only**. Unknown whether RouterBOOT hands the kernel off in AArch64
> (then a 64-bit kernel boots — common "64-bit kernel + 32-bit userland" pattern on
> 256 MB ARMv8) or strictly AArch32. **Resolved empirically at first serial netboot.**

## RouterBOOT boot settings (`/system/routerboard/settings`)
- `boot-protocol`: **`bootp`** → serve BOOTP + TFTP for netboot.
- `boot-device`: `nand-if-fail-then-ethernet` (current). For a one-shot non-destructive
  RAM netboot, set → `try-ethernet-once-then-nand`, then revert/auto-revert.
- `preboot-etherboot`: disabled. `protected-routerboot`: disabled. `silent-boot`: false.
- `cpu-frequency`: 800MHz.

## Interfaces / MAC map (base MAC `F4:1E:57:9F:F5:00`)
| iface | default-name | MAC | notes |
|---|---|---|---|
| copper GbE | `ether1` | `F4:1E:57:9F:F5:00` | **netboot port**; bridge slave; up to 1G |
| SFP | `sfp1` | `F4:1E:57:9F:F5:01` | 2.5G-baseT/baseX capable |
| radio 1 | `wifi1` | `F4:1E:57:9F:F5:02` | one of 2.4/5G (likely 2.4 in-SoC) |
| radio 2 | `wifi2` | `F4:1E:57:9F:F5:03` | one of 2.4/5G (likely QCN6102 5G) |

→ DTS: `local-mac-address`/nvmem can derive from base `…F5:00` (+0 eth, +1 sfp, +2/+3 radios).

## Still needed (deep recon — only from a booted Linux over serial)
NAND/NOR `/proc/mtd` layout + RouterBOOT/hard_config partition offsets, ath11k caldata
location + format (LZ77?), `/sys/firmware/devicetree` from MikroTik's own DTB, GPIO/LED/
button map, ethernet PHY/MDIO + the 2.5G SFP PHY, exact radio→band mapping. See
`RECON-CHECKLIST.md` (from the DTS-draft agent).
