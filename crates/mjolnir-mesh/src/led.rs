// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Connectivity LED language (mjolnir-mesh-849.5 AP3000, 849.6 M3000).
//!
//! Pure classifier: facts in, tone out. The daemon maps tone onto SKU lamps
//! and writes sysfs. Probe `amber:status` first (AP3000); else M3000
//! `red:wan-online` + `white:wan-online`. Other SKUs no-op.

use crate::crdt::egress::{DefaultRoute, EXCLUDED_EGRESS_IFACES, classify_egress};

/// Sysfs directory names on Cudy AP3000 v1 (indoor). Mix by brightness.
pub const LED_AMBER: &str = "amber:status";
pub const LED_RED: &str = "red:wlan-2ghz";
pub const LED_BLUE: &str = "blue:wlan-5ghz";
pub const LED_PHY: &[&str] = &["mt76-phy0", "mt76-phy1"];

/// Cudy M3000 front bicolor. Do not write `green:wan` / `green:lan` (port lamps).
pub const LED_M3000_RED: &str = "red:wan-online";
pub const LED_M3000_WHITE: &str = "white:wan-online";

/// Admin identify pulse: touch this file; meshd blinks while mtime is fresh.
pub const IDENTIFY_PATH: &str = "/tmp/mjolnir-identify";
/// `mjolnir-apply` lock (do not fight OpenWrt status blink).
pub const APPLY_LOCK: &str = "/root/mjolnir-stage/.lock";

pub const IDENTIFY_FLASH_MS: u64 = 200;
pub const OVERLAY_SICK_FLASH_MS: u64 = 500;
pub const IDENTIFY_FRESH_SECS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedSku {
    Ap3000,
    M3000,
}

/// Highest-wins topology. SKU maps this onto lamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedTone {
    Off,
    Alone,
    MeshNoNet,
    ViaMesh,
    Egress,
    OverlaySick,
}

/// AP3000 lamps. Solid blue is not a topology color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedMix {
    pub amber: bool,
    pub red: bool,
    pub blue: bool,
}

impl LedMix {
    pub const OFF: Self = Self {
        amber: false,
        red: false,
        blue: false,
    };
    pub const ALONE: Self = Self {
        amber: false,
        red: true,
        blue: false,
    };
    /// Mesh, no internet (reads orange).
    pub const MESH_NO_NET: Self = Self {
        amber: true,
        red: true,
        blue: false,
    };
    /// Internet via mesh (pale purple).
    pub const VIA_MESH: Self = Self {
        amber: true,
        red: false,
        blue: true,
    };
    /// Local egress / gateway / STA (dark purple).
    pub const EGRESS: Self = Self {
        amber: false,
        red: true,
        blue: true,
    };
    pub const OVERLAY_SICK: Self = Self {
        amber: false,
        red: true,
        blue: false,
    };
}

/// M3000 front red/white. Via-mesh and local egress are both white.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedWhiteMix {
    pub red: bool,
    pub white: bool,
}

impl RedWhiteMix {
    pub const OFF: Self = Self {
        red: false,
        white: false,
    };
    pub const ALONE: Self = Self {
        red: true,
        white: false,
    };
    /// Mesh, no internet (pink).
    pub const MESH_NO_NET: Self = Self {
        red: true,
        white: true,
    };
    pub const INTERNET: Self = Self {
        red: false,
        white: true,
    };
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

/// `brightness` exists under `/sys/class/leds/<name>/`. AP3000 wins if both match.
pub fn detect_sku(brightness_exists: impl Fn(&str) -> bool) -> Option<LedSku> {
    if brightness_exists(LED_AMBER) {
        Some(LedSku::Ap3000)
    } else if brightness_exists(LED_M3000_RED) && brightness_exists(LED_M3000_WHITE) {
        Some(LedSku::M3000)
    } else {
        None
    }
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

pub fn mix_ap3000(tone: LedTone) -> LedMix {
    match tone {
        LedTone::Off => LedMix::OFF,
        LedTone::Alone | LedTone::OverlaySick => LedMix::ALONE,
        LedTone::MeshNoNet => LedMix::MESH_NO_NET,
        LedTone::ViaMesh => LedMix::VIA_MESH,
        LedTone::Egress => LedMix::EGRESS,
    }
}

pub fn mix_m3000(tone: LedTone) -> RedWhiteMix {
    match tone {
        LedTone::Off => RedWhiteMix::OFF,
        LedTone::Alone | LedTone::OverlaySick => RedWhiteMix::ALONE,
        LedTone::MeshNoNet => RedWhiteMix::MESH_NO_NET,
        LedTone::ViaMesh | LedTone::Egress => RedWhiteMix::INTERNET,
    }
}

/// Brightness for one flash phase. `on` is the first half of the period.
pub fn mix_for_phase(mix: LedMix, flash_ms: Option<u64>, on: bool) -> LedMix {
    if flash_ms.is_some() && !on {
        LedMix::OFF
    } else {
        mix
    }
}

pub fn mix_red_white_for_phase(mix: RedWhiteMix, flash_ms: Option<u64>, on: bool) -> RedWhiteMix {
    if flash_ms.is_some() && !on {
        RedWhiteMix::OFF
    } else {
        mix
    }
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

    fn mix(f: LedFacts) -> LedMix {
        match classify(f) {
            LedAction::Drive(r) => mix_ap3000(r.tone),
            LedAction::Hold => panic!("hold"),
        }
    }

    fn tone(f: LedFacts) -> LedTone {
        match classify(f) {
            LedAction::Drive(r) => r.tone,
            LedAction::Hold => panic!("hold"),
        }
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
                assert_eq!(mix_ap3000(r.tone), LedMix::OVERLAY_SICK);
                assert_eq!(mix_m3000(r.tone), RedWhiteMix::ALONE);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn egress_beats_via_mesh() {
        assert_eq!(
            mix(facts(|f| {
                f.local_egress = true;
                f.default_via_mesh = true;
                f.mesh_estab = true;
            })),
            LedMix::EGRESS
        );
    }

    #[test]
    fn satellite_internet_is_amber_blue() {
        assert_eq!(
            mix(facts(|f| {
                f.default_via_mesh = true;
                f.mesh_estab = true;
            })),
            LedMix::VIA_MESH
        );
        assert!(!LedMix::VIA_MESH.red);
        assert!(LedMix::VIA_MESH.amber && LedMix::VIA_MESH.blue);
    }

    #[test]
    fn mesh_no_internet_is_amber_red() {
        assert_eq!(mix(facts(|f| f.mesh_estab = true)), LedMix::MESH_NO_NET);
        assert!(!LedMix::MESH_NO_NET.blue);
    }

    #[test]
    fn alone_is_solid_red() {
        let r = match classify(facts(|_| {})) {
            LedAction::Drive(r) => r,
            LedAction::Hold => panic!(),
        };
        assert_eq!(r.tone, LedTone::Alone);
        assert_eq!(mix_ap3000(r.tone), LedMix::ALONE);
        assert_eq!(r.flash_ms, None);
    }

    #[test]
    fn radios_idle_is_off() {
        assert_eq!(mix(facts(|f| f.radios_up = false)), LedMix::OFF);
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
        for f in [
            facts(|_| {}),
            facts(|f| f.mesh_estab = true),
            facts(|f| {
                f.mesh_estab = true;
                f.default_via_mesh = true;
            }),
            facts(|f| f.local_egress = true),
            facts(|f| {
                f.overlay_addr_present = false;
                f.mesh_l2_up = true;
            }),
        ] {
            if let LedAction::Drive(r) = classify(f) {
                let m = mix_ap3000(r.tone);
                let blue_only = m.blue && !m.red && !m.amber;
                assert!(!blue_only, "solid blue leaked: {r:?}");
            }
        }
    }

    #[test]
    fn m3000_internet_is_white_for_via_mesh_and_egress() {
        assert_eq!(
            mix_m3000(tone(facts(|f| {
                f.default_via_mesh = true;
                f.mesh_estab = true;
            }))),
            RedWhiteMix::INTERNET
        );
        assert_eq!(
            mix_m3000(tone(facts(|f| {
                f.local_egress = true;
                f.mesh_estab = true;
            }))),
            RedWhiteMix::INTERNET
        );
        assert!(!RedWhiteMix::INTERNET.red);
        assert!(RedWhiteMix::INTERNET.white);
    }

    #[test]
    fn m3000_mesh_no_internet_is_pink() {
        assert_eq!(
            mix_m3000(tone(facts(|f| f.mesh_estab = true))),
            RedWhiteMix::MESH_NO_NET
        );
        assert!(RedWhiteMix::MESH_NO_NET.red && RedWhiteMix::MESH_NO_NET.white);
    }

    #[test]
    fn m3000_alone_is_red_only() {
        assert_eq!(mix_m3000(tone(facts(|_| {}))), RedWhiteMix::ALONE);
        assert!(!RedWhiteMix::ALONE.white);
    }

    #[test]
    fn detect_sku_prefers_ap3000() {
        assert_eq!(detect_sku(|n| n == LED_AMBER), Some(LedSku::Ap3000));
        assert_eq!(
            detect_sku(|n| n == LED_AMBER || n == LED_M3000_RED || n == LED_M3000_WHITE),
            Some(LedSku::Ap3000)
        );
        assert_eq!(
            detect_sku(|n| n == LED_M3000_RED || n == LED_M3000_WHITE),
            Some(LedSku::M3000)
        );
        assert_eq!(detect_sku(|n| n == LED_M3000_RED), None);
        assert_eq!(detect_sku(|_| false), None);
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
        assert_eq!(
            mix_for_phase(LedMix::EGRESS, Some(200), true),
            LedMix::EGRESS
        );
        assert_eq!(mix_for_phase(LedMix::EGRESS, Some(200), false), LedMix::OFF);
        assert_eq!(
            mix_for_phase(LedMix::VIA_MESH, None, false),
            LedMix::VIA_MESH
        );
        assert_eq!(
            mix_red_white_for_phase(RedWhiteMix::INTERNET, Some(200), false),
            RedWhiteMix::OFF
        );
        assert_eq!(
            mix_red_white_for_phase(RedWhiteMix::MESH_NO_NET, None, false),
            RedWhiteMix::MESH_NO_NET
        );
    }
}
