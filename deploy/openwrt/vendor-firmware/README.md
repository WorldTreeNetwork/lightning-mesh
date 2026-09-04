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

Do **not** use either of these on AP3000 Wall (separate Drive zip).

## Flash order

1. Stock Cudy web UI. Indoor AP3000: Cudy wants stock **2.4.7** first
   (`ap3000-v1/warnning.txt`; download-center `ap3000-1-0`).
2. Upload the matching `*.bin` from this directory.
3. After reboot the box is LAN-only at `192.168.1.1` with DHCP. Isolate it
   from a house LAN that already has a `.1`.
4. Official OpenWrt **25.12.5 or newer** sysupgrade for that board id
   ([firmware selector](https://firmware-selector.openwrt.org/)).
   24.10.5+ is required on units with the newer F50L1G41LC flash
   (SN week ≥ 2543).
5. `deploy/openwrt/install-node.sh --wireless deploy/openwrt/fleet-secrets/wireless.env root@192.168.1.1`

Re-download from Drive if LFS is unavailable. Do not commit the Drive zips
(`**/*.zip` is gitignored).
