# Windows build pipeline

Produces a Tauri Windows release bundle (MSI + NSIS) for Lightning Admin by
driving the local **libvirt** VM `morphist-win11` over SSH (alias `win11`,
user `A`, DHCP `192.168.122.214`). Copied from Papyrus `scripts/windows-build/`
— same tar-over-ssh methods, no scp.

This machine already has the guest. You do **not** run Papyrus's
`provision-vm.sh` (that's a from-scratch quickemu install).

## Boot the VM, then build

`virsh` needs sudo. Windows takes ~15–45s after `start` before sshd answers.

```bash
sudo virsh start morphist-win11
# wait until this prints a lease:
sudo virsh net-dhcp-leases default
ssh -o BatchMode=yes win11 "echo ok"

cd admin
./scripts/windows-build/release.sh survey     # toolchain + source trees
./scripts/windows-build/release.sh smoke      # cargo debug of src-tauri
./scripts/windows-build/release.sh            # full bundle → dist-windows/
```

First time (or after a wipe): `./scripts/windows-build/release.sh bootstrap`
installs MSVC Build Tools, rustup (stable-msvc), pinned bun, and Node. It is
idempotent. Rust/MSVC are already on this VM; bootstrap still fills gaps
(Node lives at `C:\Users\A\node`).

Graceful stop when you are done: `sudo virsh shutdown morphist-win11`.

GUI if you need it:

```bash
virt-manager --connect qemu:///system --show-domain-console morphist-win11
```

## Artifacts

```
dist-windows/
├── lightning-admin.exe
└── bundle/
    ├── msi/
    └── nsis/
```

`dist-windows/` is gitignored.

## What `release.sh bundle` does

1. **sync** — `tar` the `admin/` tree to `C:\Users\A\lightning-admin` on the VM
   (excludes `target/`, `node_modules/`, `.git/`, …). Native NTFS, not virtiofs.
2. **bun install** if `node_modules` is stale.
3. **bun tauri build** — frontend via Node (Papyrus lesson: bun's Windows
   module interop can fail vite config load).
4. **fetch** — tar the bundle back over ssh into `dist-windows/`.

## Subcommands

| Command | What it does |
|---------|--------------|
| `bundle` (default) | sync → bun install → bun tauri build → fetch |
| `smoke` | sync → `cargo build` (debug) of `src-tauri` |
| `sync` | source sync only |
| `bootstrap` | install/repair toolchain on the VM |
| `survey` | print toolchain + source-tree status |
| `fetch` | pull artifacts without rebuilding |

Env: `WIN_HOST` (default `win11`), `WIN_USER` (`A`), `WIN_ROOT` (`C:\Users\A`),
`ADMIN` (this `admin/` tree), `SKIP_SYNC=1`.

## Troubleshooting

- **SSH times out / no lease** — domain is `shut off`. `sudo virsh start
  morphist-win11` and wait. Alias is `Host win11` → `192.168.122.214` in
  `~/.ssh/config`.
- **SSH key rejected** — Windows OpenSSH for admin users reads
  `C:\ProgramData\ssh\administrators_authorized_keys`, not `~/.ssh/authorized_keys`.
- **`cargo` MISSING in survey, but smoke works** — default SSH PATH does not
  include `%USERPROFILE%\.cargo\bin`. Survey/build scripts reassemble PATH;
  don't run `cargo` over a raw ssh session without doing the same.
- **VM has no internet** — Docker's FORWARD policy can drop libvirt NAT.
  Check `systemctl status libvirt-docker-bridge.service` (Papyrus host fix).
- **No scp** — Windows OpenSSH wraps every channel through PowerShell, whose
  startup banner corrupts scp. Scripts stream tar over ssh instead.
- **`fetch` / `tar: This does not look like a tar archive`** — the PowerShell
  banner was on stdout ahead of the tar bytes. `release.sh` now emits a marker
  and strips to it (same idea as `~/bin/scpower`). Re-run `release.sh fetch`
  without rebuilding.
