## ADDED Requirements

### Requirement: vwifi is the lab air

Node guests SHALL run `vwifi-client` against a host `vwifi-server` with
two `mac80211_hwsim` radios. 802.11s SHALL establish between node-a and
node-b. `vwifi-ctrl` SHALL be able to change link loss. vwifi TCP SHALL
use the management network.

#### Scenario: Mesh plink

- GIVEN two node guests with vwifi up
- WHEN both mesh points share mesh_id and channel
- THEN `iw` reports ESTAB between them
