// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Connectivity LED language (849.5–849.8).
//!
//! Facts → [`LedTone`]. Hardware is a row in [`HARDWARE`]: OpenWrt
//! `board_name` first, else sysfs probe. Each row lists which
//! `/sys/class/leds` names light for each tone. Adding a box is a row.

use crate::crdt::egress::{DefaultRoute, EXCLUDED_EGRESS_IFACES, classify_egress};

/// Admin identify pulse: touch this file; meshd blinks while mtime is fresh.
pub const IDENTIFY_PATH: &str = "/tmp/mjolnir-identify";
/// `mjolnir-apply` lock (do not fight OpenWrt status blink).
pub const APPLY_LOCK: &str = "/root/mjolnir-stage/.lock";

pub const IDENTIFY_FLASH_MS: u64 = 200;
pub const OVERLAY_SICK_FLASH_MS: u64 = 500;
pub const IDENTIFY_FRESH_SECS: u64 = 15;

/// Highest-wins topology. Hardware maps this onto lamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedTone {
    Off,
    Alone,
    MeshNoNet,
    ViaMesh,
    Egress,
    OverlaySick,
}

/// One hardware row. `drive` is every lamp we write; `silence` is forced off
/// (phy-tpt). Tones pick a subset of `drive`. Lamps not listed are never written
/// (port LEDs, WPS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareMap {
    pub id: &'static str,
    pub label: &'static str,
    /// OpenWrt `/tmp/sysinfo/board_name` values.
    pub boards: &'static [&'static str],
    /// All must exist under `/sys/class/leds` when board_name is missing.
    pub probe: &'static [&'static str],
    pub drive: &'static [&'static str],
    pub silence: &'static [&'static str],
    pub off: &'static [&'static str],
    pub alone: &'static [&'static str],
    pub mesh_no_net: &'static [&'static str],
    pub via_mesh: &'static [&'static str],
    pub egress: &'static [&'static str],
}

impl HardwareMap {
    pub fn lit(&self, tone: LedTone) -> &'static [&'static str] {
        match tone {
            LedTone::Off => self.off,
            LedTone::Alone | LedTone::OverlaySick => self.alone,
            LedTone::MeshNoNet => self.mesh_no_net,
            LedTone::ViaMesh => self.via_mesh,
            LedTone::Egress => self.egress,
        }
    }

    /// Brightness for one drive lamp. `phase_on` is false on the dark half of a flash.
    pub fn lamp_on(&self, tone: LedTone, name: &str, phase_on: bool) -> bool {
        phase_on && self.lit(tone).iter().any(|n| *n == name)
    }
}

const PHY: &[&str] = &["mt76-phy0", "mt76-phy1"];

/// Probe order matters for sysfs fallback (outdoor before TR3000: both have
/// `red:power`). Board_name skips that.
pub static HARDWARE: &[HardwareMap] = &[
    HardwareMap {
        id: "ap3000-indoor",
        label: "AP3000 indoor status lamps (849.5)",
        boards: &["cudy,ap3000-v1"],
        probe: &["amber:status"],
        drive: &["amber:status", "red:wlan-2ghz", "blue:wlan-5ghz"],
        silence: PHY,
        off: &[],
        alone: &["red:wlan-2ghz"],
        mesh_no_net: &["amber:status", "red:wlan-2ghz"],
        via_mesh: &["amber:status", "blue:wlan-5ghz"],
        egress: &["red:wlan-2ghz", "blue:wlan-5ghz"],
    },
    HardwareMap {
        id: "m3000",
        label: "M3000 status lamps (849.6)",
        boards: &["cudy,m3000-v1", "cudy,m3000-v2"],
        probe: &["red:wan-online", "white:wan-online"],
        drive: &["red:wan-online", "white:wan-online"],
        silence: PHY,
        off: &[],
        alone: &["red:wan-online"],
        mesh_no_net: &["red:wan-online", "white:wan-online"],
        via_mesh: &["white:wan-online"],
        egress: &["white:wan-online"],
    },
    HardwareMap {
        id: "ap3000-outdoor",
        label: "AP3000 Outdoor status lamps (849.7)",
        boards: &["cudy,ap3000outdoor-v1"],
        probe: &["red:power", "green:status"],
        drive: &["red:power", "green:status"],
        silence: PHY,
        off: &[],
        alone: &["red:power"],
        mesh_no_net: &["red:power", "green:status"],
        via_mesh: &["green:status"],
        egress: &["green:status"],
    },
    HardwareMap {
        id: "tr3000",
        label: "TR3000 status lamps (849.7)",
        boards: &["cudy,tr3000-v1"],
        probe: &["red:power", "white:status"],
        drive: &["red:power", "white:status"],
        silence: PHY,
        off: &[],
        alone: &["red:power"],
        mesh_no_net: &["red:power", "white:status"],
        via_mesh: &["white:status"],
        egress: &["white:status"],
    },
    HardwareMap {
        id: "wr3000s",
        label: "WR3000S status lamps (849.7)",
        boards: &["cudy,wr3000s-v1"],
        probe: &["white:status", "white:wan-online", "white:wlan-5ghz"],
        drive: &["white:status", "white:wan-online", "white:wlan-5ghz"],
        silence: &["mt76-phy0", "mt76-phy1", "white:wlan-2ghz"],
        off: &[],
        alone: &["white:status"],
        mesh_no_net: &["white:status", "white:wlan-5ghz"],
        via_mesh: &["white:wan-online"],
        egress: &["white:wan-online"],
    },
];

pub fn hardware(id: &str) -> Option<&'static HardwareMap> {
    HARDWARE.iter().find(|m| m.id == id)
}

/// Board_name wins. Else first row whose `probe` lamps all exist.
pub fn detect_hardware(
    board: Option<&str>,
    brightness_exists: impl Fn(&str) -> bool,
) -> Option<&'static HardwareMap> {
    if let Some(b) = board.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(m) = HARDWARE.iter().find(|m| m.boards.iter().any(|x| *x == b)) {
            return Some(m);
        }
    }
    HARDWARE
        .iter()
        .find(|m| m.probe.iter().copied().all(|n| brightness_exists(n)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedRender {
    pub tone: LedTone,
    /// `None` = solid. Identify 200ms, overlay-sick 500ms.
    pub flash_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedAction {
    /// Apply/sysupgrade owns the status LED — do not write sysfs.
    Hold,
    Drive(LedRender),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LedFacts {
    pub identify: bool,
    pub apply_in_progress: bool,
    pub overlay_addr_present: bool,
    /// 802.11s plink ESTAB to at least one station.
    pub mesh_estab: bool,
    /// Mesh wireless iface or `br-mesh` is administratively up.
    pub mesh_l2_up: bool,
    pub local_egress: bool,
    pub default_via_mesh: bool,
    pub radios_up: bool,
}

/// Highest-wins ladder from 849.5 (SKU-agnostic tone).
pub fn classify(f: LedFacts) -> LedAction {
    if f.apply_in_progress {
        return LedAction::Hold;
    }
    let (tone, flash_ms) = classify_tone(f);
    if f.identify {
        return LedAction::Drive(LedRender {
            tone,
            flash_ms: Some(IDENTIFY_FLASH_MS),
        });
    }
    LedAction::Drive(LedRender { tone, flash_ms })
}

fn classify_tone(f: LedFacts) -> (LedTone, Option<u64>) {
    if !f.overlay_addr_present && (f.mesh_estab || f.mesh_l2_up) {
        return (LedTone::OverlaySick, Some(OVERLAY_SICK_FLASH_MS));
    }
    if f.local_egress {
        return (LedTone::Egress, None);
    }
    if f.default_via_mesh {
        return (LedTone::ViaMesh, None);
    }
    if f.mesh_estab {
        return (LedTone::MeshNoNet, None);
    }
    if f.radios_up {
        return (LedTone::Alone, None);
    }
    (LedTone::Off, None)
}

pub fn phase_on(flash_ms: Option<u64>, on: bool) -> bool {
    flash_ms.is_none() || on
}

/// Babel-learned or backhaul-dev default — satellite internet, not local WAN.
pub fn default_via_mesh(routes: &[DefaultRoute]) -> bool {
    routes
        .iter()
        .any(|r| r.proto_babel || EXCLUDED_EGRESS_IFACES.contains(&r.oif.as_str()))
}

pub fn is_local_egress(routes: &[DefaultRoute]) -> bool {
    classify_egress(routes, EXCLUDED_EGRESS_IFACES, true).is_some()
}

/// `iw dev` interfaces of `type mesh point`.
pub fn parse_mesh_ifaces(iw_dev_output: &str) -> Vec<String> {
    let mut current: Option<String> = None;
    let mut out = Vec::new();
    for line in iw_dev_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Interface ") {
            current = Some(rest.trim().to_string());
        } else if trimmed == "type mesh point"
            && let Some(name) = current.take()
        {
            out.push(name);
        }
    }
    out
}

/// True when `iw dev <mesh> station dump` has a `mesh plink: ESTAB` line.
pub fn mesh_has_estab(station_dump: &str) -> bool {
    station_dump.lines().any(|line| {
        line.split_whitespace()
            .collect::<Vec<_>>()
            .windows(3)
            .any(|w| w == ["mesh", "plink:", "ESTAB"])
            || line.trim() == "mesh plink: ESTAB"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(patch: impl Fn(&mut LedFacts)) -> LedFacts {
        let mut f = LedFacts {
            radios_up: true,
            overlay_addr_present: true,
            ..LedFacts::default()
        };
        patch(&mut f);
        f
    }

    fn tone(f: LedFacts) -> LedTone {
        match classify(f) {
            LedAction::Drive(r) => r.tone,
            LedAction::Hold => panic!("hold"),
        }
    }

    fn lit(id: &str, f: LedFacts) -> &'static [&'static str] {
        hardware(id).unwrap().lit(tone(f))
    }

    fn exists(have: &[&str]) -> impl Fn(&str) -> bool {
        let have: Vec<String> = have.iter().map(|s| (*s).to_string()).collect();
        move |n| have.iter().any(|h| h == n)
    }

    #[test]
    fn apply_holds() {
        assert_eq!(
            classify(facts(|f| f.apply_in_progress = true)),
            LedAction::Hold
        );
    }

    #[test]
    fn overlay_sick_beats_mesh() {
        let f = facts(|f| {
            f.overlay_addr_present = false;
            f.mesh_estab = true;
            f.mesh_l2_up = true;
            f.default_via_mesh = true;
        });
        match classify(f) {
            LedAction::Drive(r) => {
                assert_eq!(r.tone, LedTone::OverlaySick);
                assert_eq!(r.flash_ms, Some(OVERLAY_SICK_FLASH_MS));
                assert_eq!(
                    hardware("ap3000-indoor").unwrap().lit(r.tone),
                    &["red:wlan-2ghz"]
                );
                assert_eq!(hardware("m3000").unwrap().lit(r.tone), &["red:wan-online"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn indoor_egress_vs_via_mesh() {
        assert_eq!(
            lit(
                "ap3000-indoor",
                facts(|f| {
                    f.local_egress = true;
                    f.default_via_mesh = true;
                    f.mesh_estab = true;
                })
            ),
            ["red:wlan-2ghz", "blue:wlan-5ghz"].as_slice()
        );
        assert_eq!(
            lit(
                "ap3000-indoor",
                facts(|f| {
                    f.default_via_mesh = true;
                    f.mesh_estab = true;
                })
            ),
            ["amber:status", "blue:wlan-5ghz"].as_slice()
        );
        assert_eq!(
            lit("ap3000-indoor", facts(|f| f.mesh_estab = true)),
            ["amber:status", "red:wlan-2ghz"].as_slice()
        );
        assert_eq!(
            lit("ap3000-indoor", facts(|_| {})),
            ["red:wlan-2ghz"].as_slice()
        );
        assert!(lit("ap3000-indoor", facts(|f| f.radios_up = false)).is_empty());
    }

    #[test]
    fn identify_flashes_current_color() {
        match classify(facts(|f| {
            f.identify = true;
            f.mesh_estab = true;
        })) {
            LedAction::Drive(r) => {
                assert_eq!(r.tone, LedTone::MeshNoNet);
                assert_eq!(r.flash_ms, Some(IDENTIFY_FLASH_MS));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn solid_blue_is_not_a_topology_mix() {
        let m = hardware("ap3000-indoor").unwrap();
        for tone in [
            LedTone::Off,
            LedTone::Alone,
            LedTone::MeshNoNet,
            LedTone::ViaMesh,
            LedTone::Egress,
            LedTone::OverlaySick,
        ] {
            assert_ne!(m.lit(tone), &["blue:wlan-5ghz"]);
        }
    }

    #[test]
    fn m3000_internet_is_white_for_via_mesh_and_egress() {
        assert_eq!(
            lit(
                "m3000",
                facts(|f| {
                    f.default_via_mesh = true;
                    f.mesh_estab = true;
                })
            ),
            ["white:wan-online"].as_slice()
        );
        assert_eq!(
            lit(
                "m3000",
                facts(|f| {
                    f.local_egress = true;
                    f.mesh_estab = true;
                })
            ),
            ["white:wan-online"].as_slice()
        );
        assert_eq!(
            lit("m3000", facts(|f| f.mesh_estab = true)),
            ["red:wan-online", "white:wan-online"].as_slice()
        );
        assert_eq!(lit("m3000", facts(|_| {})), ["red:wan-online"].as_slice());
    }

    #[test]
    fn board_name_wins_over_sysfs() {
        let m = detect_hardware(Some("cudy,m3000-v1"), exists(&["amber:status"])).unwrap();
        assert_eq!(m.id, "m3000");
    }

    #[test]
    fn probe_fallback_order() {
        assert_eq!(
            detect_hardware(None, exists(&["amber:status"])).unwrap().id,
            "ap3000-indoor"
        );
        assert_eq!(
            detect_hardware(None, exists(&["red:wan-online", "white:wan-online"]))
                .unwrap()
                .id,
            "m3000"
        );
        assert_eq!(
            detect_hardware(None, exists(&["red:power", "green:status"]))
                .unwrap()
                .id,
            "ap3000-outdoor"
        );
        assert_eq!(
            detect_hardware(None, exists(&["red:power", "white:status"]))
                .unwrap()
                .id,
            "tr3000"
        );
        assert_eq!(
            detect_hardware(None, exists(&["red:power", "green:status", "white:status"]))
                .unwrap()
                .id,
            "ap3000-outdoor"
        );
        assert_eq!(
            detect_hardware(
                None,
                exists(&["white:status", "white:wan-online", "white:wlan-5ghz"])
            )
            .unwrap()
            .id,
            "wr3000s"
        );
        assert!(detect_hardware(None, exists(&["white:status", "white:wan-online"])).is_none());
        assert!(detect_hardware(None, exists(&[])).is_none());
    }

    #[test]
    fn outdoor_internet_is_green() {
        assert_eq!(
            lit("ap3000-outdoor", facts(|f| f.default_via_mesh = true)),
            ["green:status"].as_slice()
        );
        assert_eq!(
            lit("ap3000-outdoor", facts(|f| f.local_egress = true)),
            ["green:status"].as_slice()
        );
        assert_eq!(
            lit("ap3000-outdoor", facts(|f| f.mesh_estab = true)),
            ["red:power", "green:status"].as_slice()
        );
    }

    #[test]
    fn wr3000s_which_lamp_and_not_wps() {
        let m = hardware("wr3000s").unwrap();
        assert_eq!(m.lit(LedTone::Alone), &["white:status"]);
        assert_eq!(
            m.lit(LedTone::MeshNoNet),
            &["white:status", "white:wlan-5ghz"]
        );
        assert_eq!(m.lit(LedTone::ViaMesh), &["white:wan-online"]);
        assert_eq!(m.lit(LedTone::Egress), &["white:wan-online"]);
        assert!(!m.drive.iter().any(|n| *n == "white:wps"));
        assert!(!m.silence.iter().any(|n| *n == "white:wps"));
    }

    #[test]
    fn tr3000_shares_m3000_language_on_its_lamps() {
        assert_eq!(
            lit("tr3000", facts(|f| f.mesh_estab = true)),
            ["red:power", "white:status"].as_slice()
        );
        assert_eq!(
            lit("tr3000", facts(|f| f.local_egress = true)),
            ["white:status"].as_slice()
        );
    }

    #[test]
    fn parse_mesh_and_estab() {
        let iw = "\
Interface phy0-ap0
	type AP
Interface phy1-mesh0
	type mesh point
	channel 36 (5180 MHz), width: 80 MHz
";
        assert_eq!(parse_mesh_ifaces(iw), vec!["phy1-mesh0".to_string()]);
        let dump = "\
Station 82:af:ca:e7:bd:01 (on phy1-mesh0)
	mesh plink: ESTAB
	signal: -33 dBm
";
        assert!(mesh_has_estab(dump));
        assert!(!mesh_has_estab("Station aa:bb\n\tmesh plink: LISTEN\n"));
    }

    #[test]
    fn default_via_mesh_excludes_wan() {
        let wan = DefaultRoute {
            oif: "eth0".into(),
            proto_babel: false,
        };
        let babel = DefaultRoute {
            oif: "br-mesh".into(),
            proto_babel: true,
        };
        assert!(!default_via_mesh(&[wan.clone()]));
        assert!(is_local_egress(&[wan.clone()]));
        assert!(default_via_mesh(&[babel.clone()]));
        assert!(!is_local_egress(&[babel.clone()]));
        assert!(is_local_egress(&[wan, babel]));
    }

    #[test]
    fn flash_phase_goes_dark() {
        let m = hardware("m3000").unwrap();
        assert!(m.lamp_on(LedTone::Egress, "white:wan-online", true));
        assert!(!m.lamp_on(LedTone::Egress, "white:wan-online", false));
        assert!(phase_on(None, false));
        assert!(!phase_on(Some(200), false));
    }

    #[test]
    fn adding_a_box_is_a_row() {
        assert_eq!(HARDWARE.len(), 5);
        for m in HARDWARE {
            for lamp in m
                .alone
                .iter()
                .chain(m.mesh_no_net)
                .chain(m.via_mesh)
                .chain(m.egress)
            {
                assert!(
                    m.drive.iter().any(|d| d == lamp),
                    "{} tone lamp {lamp} not in drive",
                    m.id
                );
            }
        }
    }
}
