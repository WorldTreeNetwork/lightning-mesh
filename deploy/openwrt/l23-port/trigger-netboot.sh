#!/usr/bin/env bash
# Flip the L23's RouterBOOT to ONE-SHOT etherboot and reboot, so it BOOTPs our
# netboot server exactly once, then falls back to NAND (RouterOS). Non-destructive.
#
#   ./trigger-netboot.sh           # arm one-shot etherboot + reboot
#   ./trigger-netboot.sh revert    # restore nand-if-fail-then-ethernet
#   ./trigger-netboot.sh show      # print current boot settings
#
# Env: B=<box-ip> A=<user:pass>   (defaults: 192.168.0.134 / admin:lab)
set -euo pipefail
B="${B:-192.168.0.134}"; A="${A:-admin:lab}"
REST="http://$B/rest"
set_boot() { curl -s -u "$A" "$REST/system/routerboard/settings/set" \
             -H 'content-type: application/json' -d "{\"boot-device\":\"$1\"}"; }
case "${1:-go}" in
  go)
    echo ">> arming one-shot etherboot (try-ethernet-once-then-nand) on $B"
    set_boot "try-ethernet-once-then-nand"; echo
    echo ">> current:"; curl -s -u "$A" "$REST/system/routerboard/settings" | tr ',' '\n' | grep -i boot
    echo ">> rebooting box (it will BOOTP our netboot server once)"
    curl -s -u "$A" "$REST/system/reboot" -X POST -H 'content-type: application/json' -d '{}'; echo
    ;;
  revert) echo ">> restoring nand-if-fail-then-ethernet"; set_boot "nand-if-fail-then-ethernet"; echo ;;
  show)   curl -s -u "$A" "$REST/system/routerboard/settings" | tr ',' '\n' | grep -i boot ;;
  *) echo "usage: $0 [go|revert|show]"; exit 1 ;;
esac
