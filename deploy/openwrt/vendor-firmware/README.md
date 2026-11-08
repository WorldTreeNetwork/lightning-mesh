# Vendor firmware (Git LFS)

Checked in with Git LFS. After clone:

```sh
git lfs pull
```

Wrong board image bricks the box. Match the **label**, not the marketing name.

## OpenWrt One

The One ships as OpenWrt already (NAND production, NOR recovery). There is
no vendor-signed intermediate. The fleet image is official **25.12.5**
`squashfs-sysupgrade.itb` (FIT, not a Cudy `.bin`):

| directory | file | board id |
|-----------|------|----------|
| `openwrt-one/` | `openwrt-25.12.5-mediatek-filogic-openwrt_one-squashfs-sysupgrade.itb` | `openwrt,one` |

Checksum in `openwrt-one/SHA256SUMS.openwrt-25.12.5`
(`771fbaca6cde230ac7167630674d51cdb4370d36bc4a9b22e36eb1969113779c`,
matching `downloads.openwrt.org` `releases/25.12.5` sha256sums).

The box is already OpenWrt, so flash from SSH:

```sh
scp -O -o BindInterface=en15 \
  deploy/openwrt/vendor-firmware/openwrt-one/openwrt-25.12.5-mediatek-filogic-openwrt_one-squashfs-sysupgrade.itb \
  root@192.168.1.1:/tmp/sysupgrade.itb
ssh -o BindInterface=en15 root@192.168.1.1 'sysupgrade -n /tmp/sysupgrade.itb'
```

`-n` wipes the snapshot overlay. Do **not** use `factory.ubi`,
`nor-factory.bin`, or a `ubootmod` image for this step. USB recovery
(NAND boot switch, hold **Reset**, FAT32 stick) wants the same `.itb`
renamed to `openwrt-mediatek-filogic-openwrt_one-squashfs-sysupgrade.itb`.

## Cudy-signed OpenWrt intermediates

Stock Cudy only accepts **signed** images. These bins are Cudy’s
“remove signature check” step from
[OpenWrt Software Download](https://www.cudy.com/blogs/faq/openwrt-software-download)
(Google Drive folder linked there). They are **not** official OpenWrt.

| directory | Cudy Drive zip | board | OpenWrt id after this |
|-----------|----------------|-------|------------------------|
| `ap3000-v1/` | `AP3000 V1.0.zip` | AP3000 / AP3000 V1.1 (indoor) | `cudy_ap3000-v1` |
| `ap3000-outdoor-v1/` | `AP3000 Outdoor 1.0.zip` | AP3000 Outdoor v1 | `cudy_ap3000outdoor-v1` |
| `tr3000-v1/` | `TR3000 V1_20251118.zip` | TR3000 v1, 128 MB (label TR3000 1.0, stock id R47) | `cudy_tr3000-v1` |
| `wr3000s-v1/` | `WR3000S 1.0.zip` | WR3000S v1 (label WR3000S V1.0) | `cudy_wr3000s-v1` |

Do **not** use either AP3000 image on AP3000 Wall (separate Drive zip).
Do **not** use a WR3000, WR3000E, WR3000H, or WR3000P image on a WR3000S.
Do **not** use the TR3000 v1 image on the 256 MB flash SKU. That board is
`cudy_tr3000-256mb-v1` and its intermediate is the separate Drive folder
`TR3000 256MB Flash V1 (not for TR3000 V1)`. Stock rejects the wrong one.

The WR3000S signed image is
`wr3000s-v1/cudy_wr3000s-v1-sysupgrade_20251119.bin`
(14,945,872 bytes, MD5 `403e276f74fd29917fd4260294d1a137`).
The same-date H and P images are the same size. The stock UI prints the
MD5 before it flashes. This 20251119 build also accepts the newer
F50L1G41LC flash (SN week ≥ 2543).

The TR3000 v1 signed image is
`tr3000-v1/cudy_tr3000-v1-sysupgrade_20251112_release.bin`
(14,945,870 bytes, MD5 `4551e961e22c0dafab13f5251d64d042`,
SHA256 `9ae7664b8d31dd5a5a21ece038c4dce88cf53aa78150e1f2d41dc9f18f26b9a5`).
It is the file inside Drive zip `TR3000 V1_20251118.zip`. Its metadata
accepts `cudy,tr3000-v1` and stock id `R47`, and the image names both
F50L1G41LB and F50L1G41LC, so it is the intermediate for units made from
November 2025 (SN week ≥ 2544) as well as earlier 128 MB boards.

Official OpenWrt **sysupgrade** images (unsigned — stock Cudy will refuse them)
live in the same directories, named `openwrt-<ver>-…-squashfs-sysupgrade.bin`.
Outdoor v1, TR3000 v1, and WR3000S v1 currently have **25.12.5**; checksum in
each directory's `SHA256SUMS.openwrt-25.12.5`. Use the plain sysupgrade, not a
`ubootmod` image. The TR3000 v1 25.12.5 image is
`8d482ea1855edd6a72357ecd2b7cc5ab38548d2a5eb83f6b533b9bedaf2f77b4`
(9,554,194 bytes), matching
`downloads.openwrt.org` `releases/25.12.5` sha256sums.

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
