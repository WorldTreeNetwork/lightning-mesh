# Windows build pipeline

Produces a Tauri Windows release bundle (MSI + NSIS) for Lightning Admin by
driving the local morphist-win11 VM over SSH (alias `win11`). Copied from
Papyrus `scripts/windows-build/` — same tar-over-ssh methods, no scp.

```bash
./scripts/windows-build/release.sh            # full bundle → dist-windows/
./scripts/windows-build/release.sh smoke      # cargo debug of src-tauri
./scripts/windows-build/release.sh survey     # toolchain on the VM
```

Artifacts land in `dist-windows/`:

```
dist-windows/
├── lightning-admin.exe
└── bundle/
    ├── msi/
    └── nsis/
```

`release.sh bundle` (default):

1. **sync** — `tar` the `admin/` tree to `C:\Users\A\lightning-admin` on the VM
   (excludes `target/`, `node_modules/`, `.git/`, …). Native NTFS, not virtiofs.
2. **bun install** if `node_modules` is stale.
3. **bun tauri build** — frontend via Node (Papyrus lesson: bun's Windows
   module interop can fail vite config load).
4. **fetch** — tar the bundle back over ssh into `dist-windows/`.
