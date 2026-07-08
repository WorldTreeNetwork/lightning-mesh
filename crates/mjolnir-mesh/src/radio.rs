// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Identikey Inc. and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Per-node 802.11s radio telemetry (bead mjolnir-mesh-ng9).
//!
//! `mjolnir-meshd` collects a small `radio.json` snapshot on a ~10s cadence —
//! the local mesh interface's channel/frequency, its peer stations (signal,
//! expected throughput, inactivity) and the kernel mesh path table — so a
//! browser aggregating every node's `GET /api/radio` can draw a live
//! mesh-topology view. The radio is plumbing; this is a read-only projection of
//! what the kernel already knows via `iw`.
//!
//! The parsers ([`parse_mesh_iface`], [`parse_iface_info`],
//! [`parse_station_dump`], [`parse_mpath_dump`]) are pure functions over the
//! textual output of `iw dev ...`, unit-tested against real fixtures captured
//! from a deployed mt7986 node. [`collect_radio`] is the only impure entry
//! point: it shells out to `iw` and, on any failure (no `iw` binary, no mesh
//! interface — e.g. a linux-dev/test box), returns `None` so the caller can
//! degrade silently.

use std::process::Command;

use serde::Serialize;

/// Schema version of the `radio.json` / `GET /api/radio` contract. This is a
/// FIXED wire contract shared with `mjolnir-hello` and the browser topology
/// view — bump only on a breaking field change.
pub const RADIO_SCHEMA_VERSION: u32 = 1;

/// The `radio.json` document: one node's radio telemetry snapshot. Field names
/// are the fixed wire contract — do not rename.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RadioSnapshot {
    pub version: u32,
    /// This node's derived `10.254.x.y` overlay/backhaul address, so the
    /// browser can join radio telemetry to the directory projection.
    pub backhaul_addr: String,
    /// The 802.11s mesh-point interface name, e.g. `phy1-mesh0`.
    pub mesh_if: String,
    /// The mesh interface's own MAC (its peer id in `mpaths`/other nodes'
    /// `stations`).
    pub mesh_mac: String,
    pub channel: u32,
    pub freq_mhz: u32,
    pub collected_at_unix: u64,
    pub stations: Vec<Station>,
    pub mpaths: Vec<Mpath>,
}

/// A single 802.11s peer station (one row of `iw dev <if> station dump`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Station {
    pub mac: String,
    /// First value of the `signal avg:` line, in dBm (the per-chain average
    /// combined RSSI).
    pub signal_dbm: i32,
    /// Kernel `expected throughput` estimate, in Mbps.
    pub expected_throughput_mbps: f64,
    /// Milliseconds since the last frame from this peer (`inactive time`).
    pub inactive_ms: u64,
}

/// A single mesh path-table entry (one row of `iw dev <if> mpath dump`): the
/// `DEST`, `NEXT_HOP` and airtime `METRIC` columns.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mpath {
    pub dst: String,
    pub next_hop: String,
    pub metric: u32,
}

/// Interface-level facts parsed from `iw dev <if> info`.
#[derive(Debug, Clone, PartialEq)]
pub struct IfaceInfo {
    pub mesh_mac: String,
    pub channel: u32,
    pub freq_mhz: u32,
}

/// Find the first 802.11s mesh-point interface name in `iw dev` output.
///
/// The output groups `Interface <name>` blocks under `phy#N` headers; a mesh
/// interface carries a `type mesh point` line. Returns `None` when there is no
/// mesh interface (a plain AP/STA box, or a dev machine with no wireless).
pub fn parse_mesh_iface(iw_dev_output: &str) -> Option<String> {
    let mut current: Option<String> = None;
    for line in iw_dev_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Interface ") {
            current = Some(rest.trim().to_string());
        } else if trimmed == "type mesh point"
            && let Some(name) = &current
        {
            return Some(name.clone());
        }
    }
    None
}

/// Parse `iw dev <if> info` into the interface's MAC, channel and center
/// frequency. Returns `None` if the channel line (the one datum we cannot
/// default) is absent.
pub fn parse_iface_info(info_output: &str) -> Option<IfaceInfo> {
    let mut mesh_mac = String::new();
    let mut channel: Option<u32> = None;
    let mut freq_mhz: Option<u32> = None;

    for line in info_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("addr ") {
            mesh_mac = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("channel ") {
            // `channel 36 (5180 MHz), width: 80 MHz, center1: 5210 MHz`
            let mut it = rest.split_whitespace();
            channel = it.next().and_then(|t| t.parse().ok());
            // The frequency is the first `(NNNN` token.
            freq_mhz = it
                .next()
                .map(|t| t.trim_start_matches('('))
                .and_then(|t| t.parse().ok());
        }
    }

    Some(IfaceInfo {
        mesh_mac,
        channel: channel?,
        freq_mhz: freq_mhz.unwrap_or(0),
    })
}

/// Parse `iw dev <if> station dump` into per-peer telemetry. Each peer begins
/// with a `Station <mac> (on <if>)` line; the following indented lines carry
/// its fields. Missing numeric fields default to 0 rather than dropping the
/// station, so a peer is always represented.
pub fn parse_station_dump(dump: &str) -> Vec<Station> {
    let mut stations: Vec<Station> = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Station ") {
            let mac = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string();
            stations.push(Station {
                mac,
                signal_dbm: 0,
                expected_throughput_mbps: 0.0,
                inactive_ms: 0,
            });
        } else if let Some(station) = stations.last_mut() {
            if let Some(val) = trimmed.strip_prefix("signal avg:") {
                // `signal avg:	-93 [-95, -96] dBm` -> first value.
                if let Some(v) = val.split_whitespace().next().and_then(|t| t.parse().ok()) {
                    station.signal_dbm = v;
                }
            } else if let Some(val) = trimmed.strip_prefix("inactive time:") {
                // `inactive time:	50 ms`
                if let Some(v) = val.split_whitespace().next().and_then(|t| t.parse().ok()) {
                    station.inactive_ms = v;
                }
            } else if let Some(val) = trimmed.strip_prefix("expected throughput:") {
                // `expected throughput:	63.15Mbps` -> strip the unit suffix.
                let num: String = val
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                if let Ok(v) = num.parse() {
                    station.expected_throughput_mbps = v;
                }
            }
        }
    }
    stations
}

/// Parse `iw dev <if> mpath dump` into path-table rows. The first line is the
/// column header (`DEST ADDR NEXT HOP IFACE SN METRIC ...`); each data row is
/// whitespace-separated with columns `dst next_hop iface sn metric ...`, so
/// the metric is the 5th field.
pub fn parse_mpath_dump(dump: &str) -> Vec<Mpath> {
    let mut mpaths = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("DEST") {
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }
        let Ok(metric) = fields[4].parse() else {
            continue;
        };
        mpaths.push(Mpath {
            dst: fields[0].to_string(),
            next_hop: fields[1].to_string(),
            metric,
        });
    }
    mpaths
}

/// Run `iw <args...>` and return its stdout as a `String`, or `None` if the
/// binary is missing, the command fails, or the output is not UTF-8. This is
/// the single seam that makes [`collect_radio`] degrade silently off-router.
fn run_iw(args: &[&str]) -> Option<String> {
    let output = Command::new("iw").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Collect this node's radio telemetry by shelling out to `iw`. Returns `None`
/// when there is no 802.11s mesh interface or `iw` is unavailable (dev/test
/// boxes) so the caller writes no `radio.json` and logs at debug.
///
/// `backhaul_addr` is this node's derived `10.254.x.y` address and
/// `now_unix` is the collection timestamp (injected so the assembly stays
/// testable), stamped verbatim into the snapshot.
pub fn collect_radio(backhaul_addr: &str, now_unix: u64) -> Option<RadioSnapshot> {
    let dev = run_iw(&["dev"])?;
    let mesh_if = parse_mesh_iface(&dev)?;
    let info = run_iw(&["dev", &mesh_if, "info"])?;
    let iface_info = parse_iface_info(&info)?;

    // Station/mpath dumps are best-effort: an empty table is a valid snapshot
    // (a freshly-booted node with no peers yet), so a failure here degrades to
    // empty rather than suppressing the whole document.
    let stations = run_iw(&["dev", &mesh_if, "station", "dump"])
        .map(|s| parse_station_dump(&s))
        .unwrap_or_default();
    let mpaths = run_iw(&["dev", &mesh_if, "mpath", "dump"])
        .map(|s| parse_mpath_dump(&s))
        .unwrap_or_default();

    Some(RadioSnapshot {
        version: RADIO_SCHEMA_VERSION,
        backhaul_addr: backhaul_addr.to_string(),
        mesh_if,
        mesh_mac: iface_info.mesh_mac,
        channel: iface_info.channel,
        freq_mhz: iface_info.freq_mhz,
        collected_at_unix: now_unix,
        stations,
        mpaths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real output captured from a deployed mt7986 node
    // (`ssh root@10.254.12.214 'iw dev ...'`, bead ng9).

    const IW_DEV: &str = "phy#1\n\tInterface phy1-mesh0\n\t\tifindex 7\n\t\twdev 0x100000001\n\t\taddr 82:af:ca:e7:ba:9d\n\t\ttype mesh point\n\t\tchannel 36 (5180 MHz), width: 80 MHz, center1: 5210 MHz\n\t\ttxpower 23.00 dBm\nphy#0\n\tInterface phy0-ap0\n\t\tifindex 6\n\t\twdev 0x1\n\t\taddr 80:af:ca:e7:ba:9c\n\t\tssid Lightning Mesh\n\t\ttype AP\n\t\tchannel 6 (2437 MHz), width: 20 MHz, center1: 2437 MHz\n\t\ttxpower 20.00 dBm\n";

    const IW_INFO: &str = "Interface phy1-mesh0\n\tifindex 7\n\twdev 0x100000001\n\taddr 82:af:ca:e7:ba:9d\n\ttype mesh point\n\twiphy 1\n\tchannel 36 (5180 MHz), width: 80 MHz, center1: 5210 MHz\n\ttxpower 23.00 dBm\n";

    const IW_STATION: &str = "Station 82:af:ca:e7:bd:01 (on phy1-mesh0)\n\tmesh llid:\t0\n\tmesh plid:\t0\n\tmesh plink:\tESTAB\n\tmesh airtime link metric: 170\n\tinactive time:\t50 ms\n\trx bytes:\t14240992\n\trx packets:\t111974\n\ttx bytes:\t1020377\n\tsignal:  \t-93 [-96, -96] dBm\n\tsignal avg:\t-93 [-95, -96] dBm\n\ttx bitrate:\t72.0 MBit/s 80MHz HE-MCS 0 HE-NSS 2 HE-GI 0 HE-DCM 0\n\trx bitrate:\t144.1 MBit/s 80MHz HE-MCS 1 HE-NSS 2 HE-GI 0 HE-DCM 0\n\texpected throughput:\t63.15Mbps\n\tDTIM period:\t2\n\tbeacon interval:100\n\tconnected time:\t4204 seconds\n\nStation 82:af:ca:d9:85:af (on phy1-mesh0)\n\tmesh plink:\tESTAB\n\tmesh airtime link metric: 14\n\tinactive time:\t100 ms\n\tsignal:  \t-61 [-63, -65] dBm\n\tsignal avg:\t-59 [-61, -64] dBm\n\ttx bitrate:\t1080.6 MBit/s 80MHz HE-MCS 10 HE-NSS 2 HE-GI 0 HE-DCM 0\n\texpected throughput:\t887.703Mbps\n\tconnected time:\t386 seconds\n";

    const IW_MPATH: &str = "DEST ADDR         NEXT HOP          IFACE\tSN\tMETRIC\tQLEN\tEXPTIME\tDTIM\tDRET\tFLAGS\tHOP_COUNT\tPATH_CHANGE\n82:af:ca:d9:85:af 82:af:ca:d9:85:af phy1-mesh0\t94\t14\t0\t4210\t100\t0\t0x15\t1\t1\n82:af:ca:e7:bd:01 82:af:ca:d9:85:af phy1-mesh0\t1268\t52\t0\t4210\t100\t0\t0x5\t2\t945\n";

    #[test]
    fn discovers_mesh_interface_not_ap() {
        assert_eq!(parse_mesh_iface(IW_DEV).as_deref(), Some("phy1-mesh0"));
    }

    #[test]
    fn no_mesh_interface_yields_none() {
        // AP-only box: same output with the mesh block removed.
        let ap_only = "phy#0\n\tInterface phy0-ap0\n\t\taddr 80:af:ca:e7:ba:9c\n\t\ttype AP\n";
        assert_eq!(parse_mesh_iface(ap_only), None);
        assert_eq!(parse_mesh_iface(""), None);
    }

    #[test]
    fn parses_iface_info() {
        let info = parse_iface_info(IW_INFO).unwrap();
        assert_eq!(info.mesh_mac, "82:af:ca:e7:ba:9d");
        assert_eq!(info.channel, 36);
        assert_eq!(info.freq_mhz, 5180);
    }

    #[test]
    fn parses_two_stations_with_avg_signal_and_throughput() {
        let stations = parse_station_dump(IW_STATION);
        assert_eq!(stations.len(), 2);

        assert_eq!(stations[0].mac, "82:af:ca:e7:bd:01");
        // First value of `signal avg:` (not the momentary `signal:`).
        assert_eq!(stations[0].signal_dbm, -93);
        assert_eq!(stations[0].expected_throughput_mbps, 63.15);
        assert_eq!(stations[0].inactive_ms, 50);

        assert_eq!(stations[1].mac, "82:af:ca:d9:85:af");
        assert_eq!(stations[1].signal_dbm, -59);
        assert_eq!(stations[1].expected_throughput_mbps, 887.703);
        assert_eq!(stations[1].inactive_ms, 100);
    }

    #[test]
    fn empty_station_dump_is_empty() {
        assert!(parse_station_dump("").is_empty());
    }

    #[test]
    fn parses_mpath_rows_skipping_header() {
        let mpaths = parse_mpath_dump(IW_MPATH);
        assert_eq!(mpaths.len(), 2);
        assert_eq!(mpaths[0].dst, "82:af:ca:d9:85:af");
        assert_eq!(mpaths[0].next_hop, "82:af:ca:d9:85:af");
        assert_eq!(mpaths[0].metric, 14);
        // Two-hop path: dst differs from next hop, metric is the 5th column.
        assert_eq!(mpaths[1].dst, "82:af:ca:e7:bd:01");
        assert_eq!(mpaths[1].next_hop, "82:af:ca:d9:85:af");
        assert_eq!(mpaths[1].metric, 52);
    }

    #[test]
    fn empty_mpath_dump_is_empty() {
        assert!(parse_mpath_dump("").is_empty());
        // Header only, no rows.
        assert!(parse_mpath_dump("DEST ADDR NEXT HOP IFACE SN METRIC\n").is_empty());
    }

    #[test]
    fn snapshot_serializes_to_contract_shape() {
        let snapshot = RadioSnapshot {
            version: RADIO_SCHEMA_VERSION,
            backhaul_addr: "10.254.12.214".to_string(),
            mesh_if: parse_mesh_iface(IW_DEV).unwrap(),
            mesh_mac: parse_iface_info(IW_INFO).unwrap().mesh_mac,
            channel: 36,
            freq_mhz: 5180,
            collected_at_unix: 1_751_234_567,
            stations: parse_station_dump(IW_STATION),
            mpaths: parse_mpath_dump(IW_MPATH),
        };
        let value: serde_json::Value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["mesh_if"], "phy1-mesh0");
        assert_eq!(value["mesh_mac"], "82:af:ca:e7:ba:9d");
        assert_eq!(value["channel"], 36);
        assert_eq!(value["freq_mhz"], 5180);
        assert_eq!(value["backhaul_addr"], "10.254.12.214");
        assert_eq!(value["collected_at_unix"], 1_751_234_567u64);
        assert_eq!(value["stations"][0]["mac"], "82:af:ca:e7:bd:01");
        assert_eq!(value["stations"][0]["signal_dbm"], -93);
        assert_eq!(value["stations"][0]["expected_throughput_mbps"], 63.15);
        assert_eq!(value["stations"][0]["inactive_ms"], 50);
        assert_eq!(value["mpaths"][1]["dst"], "82:af:ca:e7:bd:01");
        assert_eq!(value["mpaths"][1]["next_hop"], "82:af:ca:d9:85:af");
        assert_eq!(value["mpaths"][1]["metric"], 52);
    }
}
