# Tasks

Owed by this Admin radio-slice (`nod-admin-network-name` / st1.3) only:

- [x] Tauri command stages a wireless env (`CLIENT_SSID`, open
      encryption representable) and runs `update-fleet.sh --wireless`
      (existing sequential health-gated apply). Does not SSH-mutate UCI
      inline.
- [x] Lightning Admin UI: network name field, Apply, progress and
      per-node result (updated / skipped / halted). Copy says
      "network name".
- [x] Apply disabled when the name is empty. UTF-8, 32-octet SSID max.
- [x] Tests: command/env rendering; UI copy does not say guild.
- [x] README / `admin/README.md`: Admin is how operators set the
      fleet network name; CLI `update-fleet.sh --wireless` remains.

Not owed here (bullets, not boxes):

- Guild verbs — `add-lightning-admin-guild` / `mjolnir-mesh-st1.5`
- hello.mesh identikey wrap — `mjolnir-mesh-st1.4`
- Discovery rewrite (link-local scan stays)
- Forcing the live fleet off `Lightning Mesh` in this change
