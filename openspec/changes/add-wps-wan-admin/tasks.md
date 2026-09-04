# Tasks

- [x] `openspec/changes/add-wps-wan-admin` proposal and deltas
- [x] `/usr/sbin/mjolnir-wan-admin` arm/disarm/toggle (nft, connected prefixes, 15 min, LED)
- [x] `/etc/rc.wps/00-mjolnir-wan-admin` consumes the WPS press before stock WPS-PBC
- [x] Operator pubkeys merged into `/etc/dropbear/authorized_keys` on apply
- [x] `install-node.sh` stages the three files; `mjolnir-apply` installs them
- [x] Field: arm on wr3000s-a, SSH `root@192.168.0.15`, disarm, `:22` closed again

Not owed here (bullets, not boxes):

- `dropbear.PasswordAuth=off`
- Permanent Allow-SSH-WAN
- Client DHCPv4 / `dsd`
- Fold (after this act lands)
