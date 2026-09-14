# Storage node hardware

**Status: spec draft.** Reference builds we're designing against. Part
numbers and prices are illustrative until a build is validated on the mesh.

## Reference build A: storage (Raspberry Pi 5 + SSD)

| Part | Choice | Notes |
|---|---|---|
| Board | Raspberry Pi 5, 8 GB (16 GB if hosting several apps) | Quad Cortex-A76, aarch64. Same CPU architecture as the mesh routers' binaries target |
| Storage | M.2 NVMe SSD HAT + NVMe SSD (1–4 TB) | The Pi 5 exposes **one PCIe 2.0 x1 lane** (Gen 3 works but isn't officially supported). About 400–800 MB/s; plenty for a mesh with Wi-Fi links |
| Boot | Boot from NVMe, microSD kept as recovery | Set `BOOT_ORDER` in the Pi EEPROM |
| Power | Official 27 W USB-C supply | NVMe plus USB peripherals exceed 15 W supplies |
| Cooling | Active Cooler | Sustained app hosting throttles without it |
| Network to mesh | **Ethernet into a mesh router's LAN port** | Wi-Fi on the Pi is 2.4/5 GHz client only. Don't use it as the mesh link |

## Reference build B: local AI (Raspberry Pi 5 + Hailo)

| Part | Choice | Notes |
|---|---|---|
| Board | Raspberry Pi 5, 8 GB or 16 GB | |
| Accelerator | Raspberry Pi AI HAT+ (Hailo-8L 13 TOPS or Hailo-8 26 TOPS) | Uses the same single PCIe lane |
| Storage | microSD or USB 3 SSD | See the PCIe limitation below |

**PCIe limitation:** the Pi 5 has one PCIe lane, so an NVMe HAT and the AI HAT+
can't both use it directly. To get both SSD and Hailo on one Pi, either:
- use a HAT with a PCIe switch that carries both an NVMe drive and a Hailo
  module (a dual-M.2 "NVMe + AI" base), accepting shared bandwidth, or
- put storage on USB 3 (5 Gbps) and give the PCIe lane to the Hailo.

**Decision needed:** whether "storage" and "AI" are one node class with an
optional accelerator, or two classes. See [decisions](decisions.md#d1-one-node-class-or-two).

## What the mesh needs from the hardware

| Requirement | Why | Build A | Build B |
|---|---|---|---|
| aarch64 Linux | Reuse the mesh's Rust toolchain and cross-builds | yes | yes |
| Wired link to a router | Stable, fast, and doesn't cost mesh radio airtime | yes | yes |
| 1 TB+ durable storage | App data, mirrors, backups | yes | USB SSD |
| Survives power loss | Mesh sites lose power | Journaling filesystem; see software spec | same |
| Unattended recovery | Nobody walks to the box | Network boot fallback, watchdog | same |

## Existing unit

`lightning-raspi` already runs the walkie-talkie app at `10.42.12.165`, wired to
a mesh router. It's the first storage-node-shaped deployment. Record its exact
board revision, storage and HAT here once confirmed.
