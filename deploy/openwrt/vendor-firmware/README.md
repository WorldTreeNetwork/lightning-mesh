# Cudy-signed OpenWrt intermediates

Stock Cudy only accepts **signed** images. These bins are Cudy’s
“remove signature check” step from
[OpenWrt Software Download](https://www.cudy.com/blogs/faq/openwrt-software-download)
(Google Drive folder linked there). They are **not** official OpenWrt.

Checked in with Git LFS. After clone:

```sh
git lfs pull
```

Wrong board image bricks the box. Match the **label**, not the marketing name.

| directory | Cudy Drive zip | board | OpenWrt id after this |
|-----------|----------------|-------|------------------------|
| `ap3000-v1/` | `AP3000 V1.0.zip` | AP3000 / AP3000 V1.1 (indoor) | `cudy_ap3000-v1` |
| `ap3000-outdoor-v1/` | `AP3000 Outdoor 1.0.zip` | AP3000 Outdoor v1 | `cudy_ap3000outdoor-v1` |
| `wr3000s-v1/` | `WR3000S 1.0.zip` | WR3000S v1 (label WR3000S V1.0) | `cudy_wr3000s-v1` |

Do **not** use either AP3000 image on AP3000 Wall (separate Drive zip).
Do **not** use a WR3000, WR3000E, WR3000H, or WR3000P image on a WR3000S.

The WR3000S signed image is
`wr3000s-v1/cudy_wr3000s-v1-sysupgrade_20251119.bin`
(14,945,872 bytes, MD5 `403e276f74fd29917fd4260294d1a137`).
The same-date H and P images are the same size. The stock UI prints the
MD5 before it flashes. This 20251119 build also accepts the newer
F50L1G41LC flash (SN week ≥ 2543).

Official OpenWrt **sysupgrade** images (unsigned — stock Cudy will refuse them)
live in the same directories, named `openwrt-<ver>-…-squashfs-sysupgrade.bin`.
Outdoor v1 and WR3000S v1 currently have **25.12.5**; checksum in each
directory's `SHA256SUMS.openwrt-25.12.5`. Use the plain sysupgrade, not a
`ubootmod` image.

## Flash order

1. Stock Cudy web UI. Indoor AP3000: Cudy wants stock **2.4.7** first
   (`ap3000-v1/warnning.txt`; download-center `ap3000-1-0`).
2. Upload the matching **Cudy-signed** `cudy_*.bin` from this directory.
   Never the `openwrt-*.bin` — stock Cudy rejects unsigned images.
3. After reboot the box is LAN-only at `192.168.1.1` with DHCP. Isolate it
   from a house LAN that already has a `.1`.
4. Official OpenWrt **25.12.5 or newer** sysupgrade for that board id
   (`openwrt-*-squashfs-sysupgrade.bin` here, or the
   [firmware selector](https://firmware-selector.openwrt.org/)).
   24.10.5+ is required on units with the newer F50L1G41LC flash
   (SN week ≥ 2543). Use `sysupgrade -n` (wipe Cudy overlay).
5. `deploy/openwrt/install-node.sh --wireless deploy/openwrt/fleet-secrets/wireless.env root@192.168.1.1`

Re-download Cudy intermediates from Drive if LFS is unavailable. Official
images can be re-fetched from downloads.openwrt.org. Do not commit the Drive
zips (`**/*.zip` is gitignored).
