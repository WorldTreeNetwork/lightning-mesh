use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::crdt::hlc::HLC;
use crate::crdt::merge::MergeResult;

/// Longest SSID 802.11 allows, in bytes.
pub const MAX_SSID_LEN: usize = 32;
/// Shortest neighbour report element body: BSSID (6) + BSSID info (4) +
/// operating class + channel + PHY type.
pub const MIN_NR_LEN: usize = 13;
/// An information element body cannot exceed one length octet.
pub const MAX_NR_LEN: usize = 255;

/// One client AP's 802.11k neighbour report, as its owning node read it from
/// hostapd (`rrm_nr_get_own`).
///
/// Keyed by BSSID ([`bssid_key`]). Only the node that runs the AP announces
/// it, so merge is last-writer-wins on `hlc`. The owner refreshes the entry
/// on a loop; one it stops refreshing ages out through `expiry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighbourReportEntry {
    pub bssid: [u8; 6],
    pub ssid: String,
    /// Neighbour report element body, lowercase hex, as hostapd printed it.
    pub nr_hex: String,
    /// Node that runs this AP (subject == announcer).
    pub node_id: String,
    /// Unix timestamp in seconds; daemon reaps expired entries.
    pub expiry: u64,
    pub hlc: HLC,
}

impl NeighbourReportEntry {
    /// True when the fields are safe to hand to hostapd: a real SSID, and a
    /// report body that is well-formed hex and describes this very BSSID.
    pub fn is_well_formed(&self) -> bool {
        valid_ssid(&self.ssid)
            && valid_nr_hex(&self.nr_hex)
            && self.nr_hex == self.nr_hex.to_ascii_lowercase()
            && nr_names_bssid(&self.nr_hex, &self.bssid)
    }
}

/// BSSID → neighbour report. Key is [`bssid_key`].
pub type NeighbourReportBook = BTreeMap<String, NeighbourReportEntry>;

/// Lowercase `aa:bb:cc:dd:ee:ff`.
pub fn bssid_key(bssid: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        bssid[0], bssid[1], bssid[2], bssid[3], bssid[4], bssid[5]
    )
}

/// Parse exactly six colon-separated hex octets. Stricter than
/// [`parse_mac`](crate::crdt::lease::parse_mac): the value is fed to hostapd.
pub fn parse_bssid(s: &str) -> Option<[u8; 6]> {
    let mut bssid = [0u8; 6];
    let mut parts = s.split(':');
    for octet in &mut bssid {
        let part = parts.next()?;
        if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        *octet = u8::from_str_radix(part, 16).ok()?;
    }
    parts.next().is_none().then_some(bssid)
}

/// 1 to 32 bytes.
pub fn valid_ssid(ssid: &str) -> bool {
    !ssid.is_empty() && ssid.len() <= MAX_SSID_LEN
}

/// Even-length hex, within the bounds of a neighbour report element body.
pub fn valid_nr_hex(nr_hex: &str) -> bool {
    nr_hex.len().is_multiple_of(2)
        && (MIN_NR_LEN * 2..=MAX_NR_LEN * 2).contains(&nr_hex.len())
        && nr_hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A neighbour report body starts with the BSSID it describes.
fn nr_names_bssid(nr_hex: &str, bssid: &[u8; 6]) -> bool {
    let want: String = bssid.iter().map(|b| format!("{b:02x}")).collect();
    nr_hex
        .get(..12)
        .is_some_and(|head| head.eq_ignore_ascii_case(&want))
}

/// Merge one BSSID's report. Last-writer-wins on HLC: the owner's latest
/// refresh carries the current SSID, channel and expiry.
pub fn merge_neighbour_report(
    local: Option<&NeighbourReportEntry>,
    incoming: &NeighbourReportEntry,
) -> MergeResult<NeighbourReportEntry> {
    match local {
        None => MergeResult::Inserted,
        Some(existing) => match incoming.hlc.cmp(&existing.hlc) {
            std::cmp::Ordering::Greater => MergeResult::Updated,
            _ => MergeResult::Unchanged,
        },
    }
}

/// Apply `incoming` into `book`. A malformed entry is never stored.
pub fn apply_neighbour_report(
    book: &mut NeighbourReportBook,
    incoming: NeighbourReportEntry,
) -> MergeResult<NeighbourReportEntry> {
    if !incoming.is_well_formed() {
        return MergeResult::Unchanged;
    }
    let key = bssid_key(&incoming.bssid);
    let result = merge_neighbour_report(book.get(&key), &incoming);
    match &result {
        MergeResult::Inserted | MergeResult::Updated => {
            book.insert(key, incoming);
        }
        MergeResult::Unchanged | MergeResult::Conflict { .. } => {}
    }
    result
}

/// Drop reports whose `expiry` is at or before `now_unix`. Returns the removed
/// entries. There is no release message: every node reaps on its own clock.
pub fn reap_expired_neighbour_reports(
    book: &mut NeighbourReportBook,
    now_unix: u64,
) -> Vec<NeighbourReportEntry> {
    let mut gone = Vec::new();
    book.retain(|_, e| {
        if e.expiry <= now_unix {
            gone.push(e.clone());
            false
        } else {
            true
        }
    });
    gone
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hlc(wall_clock: u64, counter: u32, node_id: &str) -> HLC {
        HLC {
            wall_clock,
            counter,
            node_id: node_id.to_string(),
        }
    }

    fn entry(last: u8, node: &str, expiry: u64, wall_clock: u64) -> NeighbourReportEntry {
        let bssid = [0xd4, 0x0d, 0xab, 0x81, 0xb2, last];
        NeighbourReportEntry {
            bssid,
            ssid: "Lightning Mesh".to_string(),
            nr_hex: format!("d40dab81b2{last:02x}ef4900005106070603000600"),
            node_id: node.to_string(),
            expiry,
            hlc: hlc(wall_clock, 0, node),
        }
    }

    #[test]
    fn postcard_roundtrip() {
        let original = entry(0xa8, "node-a", 1_700_000_120, 1_700_000_000_000);
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: NeighbourReportEntry = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn bssid_parses_strictly_and_keys_lowercase() {
        let bssid = parse_bssid("D4:0D:AB:81:B2:A8").unwrap();
        assert_eq!(bssid_key(&bssid), "d4:0d:ab:81:b2:a8");
        assert!(parse_bssid("d4:0d:ab:81:b2").is_none());
        assert!(parse_bssid("d4:0d:ab:81:b2:a8:00").is_none());
        assert!(parse_bssid("d40dab81b2a8").is_none());
        assert!(parse_bssid("d4:0d:ab:81:b2:zz").is_none());
        assert!(parse_bssid("d4:0d:ab:81:b2:+8").is_none());
        assert!(parse_bssid("").is_none());
    }

    #[test]
    fn nr_hex_must_be_even_bounded_hex() {
        assert!(valid_nr_hex("d40dab81b2a8ef4900005106070603000600"));
        assert!(!valid_nr_hex("d40dab81b2a8ef490000510607060300060"));
        assert!(!valid_nr_hex("d40dab81b2a8ef49000051060706030006zz"));
        assert!(!valid_nr_hex("d40dab81b2a8"));
        assert!(!valid_nr_hex(""));
        assert!(!valid_nr_hex(&"ab".repeat(MAX_NR_LEN + 1)));
    }

    #[test]
    fn malformed_entry_is_not_stored() {
        let mut book = NeighbourReportBook::new();

        let mut wrong_bssid = entry(0xa8, "node-a", 200, 1_000);
        wrong_bssid.bssid[5] = 0xa9;
        assert_eq!(
            apply_neighbour_report(&mut book, wrong_bssid),
            MergeResult::Unchanged
        );

        let mut no_ssid = entry(0xa8, "node-a", 200, 1_000);
        no_ssid.ssid.clear();
        assert_eq!(
            apply_neighbour_report(&mut book, no_ssid),
            MergeResult::Unchanged
        );

        let mut long_ssid = entry(0xa8, "node-a", 200, 1_000);
        long_ssid.ssid = "x".repeat(MAX_SSID_LEN + 1);
        assert_eq!(
            apply_neighbour_report(&mut book, long_ssid),
            MergeResult::Unchanged
        );

        let mut upper = entry(0xa8, "node-a", 200, 1_000);
        upper.nr_hex = upper.nr_hex.to_ascii_uppercase();
        assert_eq!(
            apply_neighbour_report(&mut book, upper),
            MergeResult::Unchanged
        );

        assert!(book.is_empty());
    }

    #[test]
    fn merge_is_last_writer_wins_on_hlc() {
        let mut book = NeighbourReportBook::new();
        let first = entry(0xa8, "node-a", 200, 1_000);
        assert_eq!(
            apply_neighbour_report(&mut book, first.clone()),
            MergeResult::Inserted
        );
        // Same stamp again: idempotent.
        assert_eq!(
            apply_neighbour_report(&mut book, first.clone()),
            MergeResult::Unchanged
        );
        // An older stamp never replaces a newer one.
        let older = entry(0xa8, "node-a", 900, 500);
        assert_eq!(
            apply_neighbour_report(&mut book, older),
            MergeResult::Unchanged
        );
        assert_eq!(book["d4:0d:ab:81:b2:a8"].expiry, 200);
        // A refresh moves the expiry forward.
        let refresh = entry(0xa8, "node-a", 230, 31_000);
        assert_eq!(
            apply_neighbour_report(&mut book, refresh),
            MergeResult::Updated
        );
        assert_eq!(book["d4:0d:ab:81:b2:a8"].expiry, 230);
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn reap_drops_at_or_before_now() {
        let mut book = NeighbourReportBook::new();
        apply_neighbour_report(&mut book, entry(0xa8, "node-a", 100, 1_000));
        apply_neighbour_report(&mut book, entry(0xa9, "node-b", 101, 1_000));
        assert!(reap_expired_neighbour_reports(&mut book, 99).is_empty());
        let gone = reap_expired_neighbour_reports(&mut book, 100);
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].node_id, "node-a");
        assert_eq!(book.len(), 1);
        assert!(book.contains_key("d4:0d:ab:81:b2:a9"));
    }
}
