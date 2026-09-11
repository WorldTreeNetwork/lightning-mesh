# add-wps-wan-admin design

WAN SSH is a security surface. Constraints:

1. **Prefix, not the internet.** `nft` accept TCP/22 from the WAN iface's
   connected IPv4 network (`ipcalc.sh`) and each global IPv6 prefix on that
   iface. Never `0.0.0.0/0` / `::/0`.
2. **No UCI.** Insert into `inet fw4 input_wan` with comment
   `mjolnir-wps-admin`. `fw4 reload` and reboot drop the window.
3. **15 minutes, toggle.** Background sleeper in
   `/var/run/mjolnir-wan-admin.pid`. Second press kills it and deletes
   handles.
4. **Consume WPS.** OpenWrt 25.12 `/etc/rc.button/wps` runs `/etc/rc.wps/*`
   and `break`s on first exit 0. Name the hook `00-` so `40-wps_ap` never
   starts `wps_pbc` on the open client AP.
5. **Keys.** Apply *merges* staged pubkeys; it does not clobber extra keys
   already on the box.
6. **Reset stays reset.** GPIO 1 / `KEY_RESTART` is untouched.
