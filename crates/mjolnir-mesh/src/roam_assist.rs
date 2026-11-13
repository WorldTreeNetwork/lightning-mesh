// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Roam assistance: 802.11k neighbour reports between mesh client APs
//! (change `add-client-roam-continuity`).
//!
//! Each node reads its own neighbour report from hostapd over ubus, gossips
//! it, and installs the reports of other nodes' client APs that carry the
//! same SSID. This is advice to the phone: nothing here disassociates or
//! steers a client. Parsers and payload builders are pure over the ubus
//! JSON; the daemon runs `ubus`.
//!
//! The entry, its merge and its expiry live in
//! [`crdt::neighbour_report`](crate::crdt::neighbour_report).

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::crdt::hlc::HLC;
use crate::crdt::neighbour_report::{
    NeighbourReportBook, NeighbourReportEntry, bssid_key, parse_bssid, valid_nr_hex, valid_ssid,
};

/// One hostapd neighbour: the `[bssid, ssid, nr_hex]` triple that
/// `rrm_nr_get_own` returns, `rrm_nr_list` lists and `rrm_nr_set` takes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Neighbour {
    pub bssid: [u8; 6],
    pub ssid: String,
    /// Neighbour report element body, lowercase hex.
    pub nr_hex: String,
}

impl Neighbour {
    /// This node's gossip entry for one of its own client APs.
    pub fn to_entry(&self, node_id: &str, expiry: u64, hlc: HLC) -> NeighbourReportEntry {
        NeighbourReportEntry {
            bssid: self.bssid,
            ssid: self.ssid.clone(),
            nr_hex: self.nr_hex.clone(),
            node_id: node_id.to_string(),
            expiry,
            hlc,
        }
    }
}

/// ubus object for one AP interface: `hostapd.phy0-ap0`.
pub fn hostapd_object(ap_iface: &str) -> String {
    format!("hostapd.{ap_iface}")
}

/// One `[bssid, ssid, nr_hex]` array. Hex is lowercased so two spellings of
/// one report compare equal.
fn parse_triple(value: &Value) -> Option<Neighbour> {
    let [bssid, ssid, nr_hex] = value.as_array()?.as_slice() else {
        return None;
    };
    let bssid = parse_bssid(bssid.as_str()?)?;
    let ssid = ssid.as_str()?;
    let nr_hex = nr_hex.as_str()?;
    if !valid_ssid(ssid) || !valid_nr_hex(nr_hex) {
        return None;
    }
    Some(Neighbour {
        bssid,
        ssid: ssid.to_string(),
        nr_hex: nr_hex.to_ascii_lowercase(),
    })
}

/// Parse `ubus call hostapd.<ap> rrm_nr_get_own`:
/// `{"value": [bssid, ssid, nr_hex]}`. None on anything else.
pub fn parse_nr_own(json: &str) -> Option<Neighbour> {
    let root: Value = serde_json::from_str(json).ok()?;
    parse_triple(root.get("value")?)
}

/// Parse `ubus call hostapd.<ap> rrm_nr_list`:
/// `{"list": [[bssid, ssid, nr_hex], ...]}`.
///
/// None when the reply is not that shape: the installed list is then unknown
/// and the caller must not act on it. A malformed item inside a well-formed
/// list is dropped, which makes the list differ from the desired one and so
/// gets it replaced.
pub fn parse_nr_list(json: &str) -> Option<Vec<Neighbour>> {
    let root: Value = serde_json::from_str(json).ok()?;
    let list = root.get("list")?.as_array()?;
    Some(list.iter().filter_map(parse_triple).collect())
}

/// The neighbours one local AP should advertise: every unexpired report from
/// another node whose SSID equals this AP's. Sorted by BSSID.
///
/// `own_bssids` are this node's own client APs (lowercase [`bssid_key`]
/// form). They are left out even if the book holds them under another node
/// id, so an AP never lists itself.
pub fn desired_neighbours(
    book: &NeighbourReportBook,
    self_id: &str,
    own_bssids: &BTreeSet<String>,
    ap_ssid: &str,
    now_unix: u64,
) -> Vec<Neighbour> {
    // The book is a BTreeMap keyed by BSSID, so iteration is already sorted.
    book.iter()
        .filter(|(key, e)| {
            e.node_id != self_id
                && !own_bssids.contains(*key)
                && e.ssid == ap_ssid
                && e.expiry > now_unix
                && e.is_well_formed()
        })
        .map(|(_, e)| Neighbour {
            bssid: e.bssid,
            ssid: e.ssid.clone(),
            nr_hex: e.nr_hex.clone(),
        })
        .collect()
}

/// True when hostapd already holds exactly the desired neighbours. Order and
/// duplicates do not matter. False after a hostapd restart (it forgets the
/// list), which is what makes the reconcile loop repair it.
pub fn neighbours_match(installed: &[Neighbour], desired: &[Neighbour]) -> bool {
    let installed: BTreeSet<&Neighbour> = installed.iter().collect();
    let desired: BTreeSet<&Neighbour> = desired.iter().collect();
    installed == desired
}

/// `rrm_nr_set` argument: `{"list": [[bssid, ssid, nr_hex], ...]}`. Replaces
/// the AP's whole neighbour list; an empty list clears it.
pub fn rrm_nr_set_payload(neighbours: &[Neighbour]) -> String {
    let list: Vec<Value> = neighbours
        .iter()
        .map(|n| json!([bssid_key(&n.bssid), n.ssid, n.nr_hex]))
        .collect();
    json!({ "list": list }).to_string()
}

/// `bss_mgmt_enable` argument: 802.11k neighbour and beacon reports plus
/// 802.11v BSS transition management. `link_measurement` is left as it is.
pub fn bss_mgmt_enable_payload() -> String {
    json!({
        "neighbor_report": true,
        "beacon_report": true,
        "bss_transition": true,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::neighbour_report::apply_neighbour_report;

    const OWN: &str = r#"{
	"value": [
		"d4:0d:ab:81:b2:a8",
		"Lightning Mesh",
		"d40dab81b2a8ef4900005106070603000600"
	]
}"#;

    fn hlc(wall_clock: u64, node_id: &str) -> HLC {
        HLC {
            wall_clock,
            counter: 0,
            node_id: node_id.to_string(),
        }
    }

    fn neighbour(last: u8, ssid: &str) -> Neighbour {
        Neighbour {
            bssid: [0xd4, 0x0d, 0xab, 0x81, 0xb2, last],
            ssid: ssid.to_string(),
            nr_hex: format!("d40dab81b2{last:02x}ef4900005106070603000600"),
        }
    }

    fn book(entries: &[(u8, &str, &str, u64)]) -> NeighbourReportBook {
        let mut book = NeighbourReportBook::new();
        for (last, ssid, node, expiry) in entries {
            apply_neighbour_report(
                &mut book,
                neighbour(*last, ssid).to_entry(node, *expiry, hlc(1_000, node)),
            );
        }
        book
    }

    #[test]
    fn parses_own_report_from_a_live_node() {
        assert_eq!(parse_nr_own(OWN), Some(neighbour(0xa8, "Lightning Mesh")));
    }

    #[test]
    fn own_report_hex_is_lowercased() {
        let json = r#"{"value":["D4:0D:AB:81:B2:A8","Lightning Mesh","D40DAB81B2A8EF4900005106070603000600"]}"#;
        assert_eq!(parse_nr_own(json), Some(neighbour(0xa8, "Lightning Mesh")));
    }

    #[test]
    fn rejects_malformed_own_report() {
        // Not JSON, wrong key, wrong arity, wrong types.
        assert_eq!(parse_nr_own(""), None);
        assert_eq!(parse_nr_own("{}"), None);
        assert_eq!(parse_nr_own(r#"{"value":[]}"#), None);
        assert_eq!(
            parse_nr_own(r#"{"value":["d4:0d:ab:81:b2:a8","Lightning Mesh"]}"#),
            None
        );
        assert_eq!(
            parse_nr_own(
                r#"{"value":["d4:0d:ab:81:b2:a8",7,"d40dab81b2a8ef4900005106070603000600"]}"#
            ),
            None
        );
        // BSSID that is not six octets.
        assert_eq!(
            parse_nr_own(
                r#"{"value":["d4:0d:ab:81:b2","Lightning Mesh","d40dab81b2a8ef4900005106070603000600"]}"#
            ),
            None
        );
        // Odd-length hex, then non-hex.
        assert_eq!(
            parse_nr_own(
                r#"{"value":["d4:0d:ab:81:b2:a8","Lightning Mesh","d40dab81b2a8ef490000510607060300060"]}"#
            ),
            None
        );
        assert_eq!(
            parse_nr_own(
                r#"{"value":["d4:0d:ab:81:b2:a8","Lightning Mesh","d40dab81b2a8ef49000051060706030006zz"]}"#
            ),
            None
        );
    }

    #[test]
    fn parses_installed_list() {
        assert_eq!(parse_nr_list(r#"{"list":[]}"#), Some(Vec::new()));
        assert_eq!(parse_nr_list("{ \"list\": [ ] }\n"), Some(Vec::new()));
        let json = r#"{"list":[
            ["d4:0d:ab:81:b2:a9","Lightning Mesh","d40dab81b2a9ef4900005106070603000600"],
            ["nonsense","Lightning Mesh","00"],
            ["d4:0d:ab:81:b2:aa","Lightning Mesh","d40dab81b2aaef4900005106070603000600"]
        ]}"#;
        assert_eq!(
            parse_nr_list(json),
            Some(vec![
                neighbour(0xa9, "Lightning Mesh"),
                neighbour(0xaa, "Lightning Mesh")
            ])
        );
    }

    #[test]
    fn unknown_installed_list_is_none_not_empty() {
        // An error reply must not read as "hostapd has no neighbours".
        assert_eq!(parse_nr_list(""), None);
        assert_eq!(parse_nr_list("{}"), None);
        assert_eq!(parse_nr_list(r#"{"list":"x"}"#), None);
    }

    #[test]
    fn desired_is_same_ssid_peers_sorted() {
        let book = book(&[
            (0xcc, "Lightning Mesh", "node-c", 500),
            (0xa8, "Lightning Mesh", "node-a", 500),
            (0xbb, "Lightning Mesh", "node-b", 500),
            (0xdd, "Other", "node-d", 500),
        ]);
        let own = BTreeSet::from(["d4:0d:ab:81:b2:a8".to_string()]);
        assert_eq!(
            desired_neighbours(&book, "node-a", &own, "Lightning Mesh", 100),
            vec![
                neighbour(0xbb, "Lightning Mesh"),
                neighbour(0xcc, "Lightning Mesh")
            ]
        );
        // SSID match is exact, case included.
        assert!(desired_neighbours(&book, "node-a", &own, "lightning mesh", 100).is_empty());
    }

    #[test]
    fn desired_skips_own_and_expired() {
        let book = book(&[
            // Own second radio, published under our id.
            (0xa9, "Lightning Mesh", "node-a", 500),
            // Our BSSID, stale in the book under an old node id.
            (0xa8, "Lightning Mesh", "node-old", 500),
            // A dead peer: lifetime has passed.
            (0xbb, "Lightning Mesh", "node-b", 100),
            (0xcc, "Lightning Mesh", "node-c", 101),
        ]);
        let own = BTreeSet::from(["d4:0d:ab:81:b2:a8".to_string()]);
        assert_eq!(
            desired_neighbours(&book, "node-a", &own, "Lightning Mesh", 100),
            vec![neighbour(0xcc, "Lightning Mesh")]
        );
    }

    #[test]
    fn match_ignores_order_and_detects_any_difference() {
        let a = neighbour(0xbb, "Lightning Mesh");
        let b = neighbour(0xcc, "Lightning Mesh");
        assert!(neighbours_match(&[], &[]));
        assert!(neighbours_match(
            &[b.clone(), a.clone()],
            &[a.clone(), b.clone()]
        ));
        // hostapd restarted and forgot the list.
        assert!(!neighbours_match(&[], std::slice::from_ref(&a)));
        // A dead node must leave the list.
        assert!(!neighbours_match(
            &[a.clone(), b.clone()],
            std::slice::from_ref(&a)
        ));
        // Same BSSID, new channel in the report body.
        let mut moved = a.clone();
        moved.nr_hex = "d40dab81b2bbef4900005101070603000100".to_string();
        assert!(!neighbours_match(&[a], &[moved]));
    }

    #[test]
    fn set_payload_roundtrips_through_the_list_parser() {
        let want = vec![
            neighbour(0xbb, "Lightning Mesh"),
            neighbour(0xcc, "Lightning Mesh"),
        ];
        let payload = rrm_nr_set_payload(&want);
        assert_eq!(
            payload,
            r#"{"list":[["d4:0d:ab:81:b2:bb","Lightning Mesh","d40dab81b2bbef4900005106070603000600"],["d4:0d:ab:81:b2:cc","Lightning Mesh","d40dab81b2ccef4900005106070603000600"]]}"#
        );
        assert_eq!(parse_nr_list(&payload), Some(want));
        assert_eq!(rrm_nr_set_payload(&[]), r#"{"list":[]}"#);
    }

    #[test]
    fn set_payload_escapes_the_ssid() {
        let odd = neighbour(0xbb, r#"Bob's "Wi-Fi" \ ⚡"#);
        let payload = rrm_nr_set_payload(std::slice::from_ref(&odd));
        assert_eq!(parse_nr_list(&payload), Some(vec![odd]));
    }

    #[test]
    fn enable_payload_turns_on_11k_and_11v_only() {
        let v: Value = serde_json::from_str(&bss_mgmt_enable_payload()).unwrap();
        assert_eq!(v["neighbor_report"], true);
        assert_eq!(v["beacon_report"], true);
        assert_eq!(v["bss_transition"], true);
        assert!(v.get("link_measurement").is_none());
        assert_eq!(v.as_object().unwrap().len(), 3);
    }

    #[test]
    fn own_report_becomes_a_well_formed_entry() {
        let entry = parse_nr_own(OWN).unwrap().to_entry(
            "node-a",
            1_700_000_120,
            hlc(1_700_000_000_000, "node-a"),
        );
        assert!(entry.is_well_formed());
        assert_eq!(bssid_key(&entry.bssid), "d4:0d:ab:81:b2:a8");
        assert_eq!(entry.node_id, "node-a");
        assert_eq!(entry.expiry, 1_700_000_120);
    }
}
