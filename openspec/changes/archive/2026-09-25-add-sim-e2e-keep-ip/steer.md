# steer add-sim-e2e-keep-ip

**When.** 2026-09-25
**Depth.** decide-for-me (intend recommended)

## Decided

- Dual proto 158 (A still has `/32` after hop) is **FAIL**, not xfail
  (decide-for-me | intend recommended)
  Why: xfail would hide the roam dataplane bug the sim already showed.
- Harness never `bd close` `5wc` / `wvg` / `wvg.1` / `sz9.1`
  (decide-for-me | sim-lab contract)
- Linux STA hop is the instrument; metal phone walk stays `5wc`

## Skipped

- none

## Auto

- [AUTO] Wrap existing `roam-keep-ip.sh`; do not invent a second hop path.
- [AUTO] Advise (other-family) before act on this instrument.

## Feeds change

CI/health runner treats current live hop as red until A withdraws proto 158.
