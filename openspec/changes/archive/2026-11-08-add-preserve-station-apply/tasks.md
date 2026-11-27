# Tasks

- [x] `settings_export_radio` exports `RADIOSET_FUNCTIONS` from `mjolnir.radioset.function`
- [x] `RUN_WIRELESS=1` does not fall through to the z3th last-wins FATAL when that table is set
- [x] Live STA with no radioset table is refused before the last-wins template
- [x] Test or shell check: owned station + radioset table → setup-wireless radioset branch, no FATAL (`sh -n` apply + radioset branch already in setup-wireless.sh)
