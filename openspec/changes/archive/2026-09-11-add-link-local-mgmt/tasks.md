# Tasks

- [x] Collect this node's kernel `fe80::/10` on `br-lan` and `br-mesh`
      (not `mjolnir0` `overlay_link_local`)
- [x] Project onto `DirectoryNode` as additive fields; omit when empty
- [x] Directory / hello tests: `/api/node` includes the LL when present;
      older consumers survive missing keys
- [x] Admin `scan_link_local` still lists scoped LL; no overlay/DHCP
      required (no Admin code change; existing scan is the surface)
- [x] ARCHITECTURE.md: LL is first-contact mgmt; overlay `10.254` stays
      mesh-wide SSH

Not owed here (bullets, not boxes):

- Derived ULA (`v6.2`)
- SLAAC RA (`v6.3`)
- Fold (after this act lands)
