// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! AP3000 connectivity LED language (mjolnir-mesh-849.5).
//!
//! Pure classifier: facts in, mix out. The daemon writes sysfs. Other SKUs
//! have different LED names — the writer no-ops when `amber:status` is absent.

use crate::crdt::egress::{DefaultRoute, EXCLUDED_EGRESS_IFACES, classify_egress};

/// Sysfs directory names on Cudy AP3000 v1 (indoor). Mix by brightness.
pub const LED_AMBER: &str = "amber:status";
pub const LED_RED: &str = "red:wlan-2ghz";
pub const LED_BLUE: &str = "blue:wlan-5ghz";
pub const LED_PHY: &[&str] = &["mt76-phy0", "mt76-phy1"];

/// Admin identify pulse: touch this file; meshd blinks while mtime is fresh.
pub const IDENTIFY_PATH: &str = "/tmp/mjolnir-identify";
/// `mjolnir-apply` lock (do not fight OpenWrt status blink).
pub const APPLY_LOCK: &str = "/root/mjolnir-stage/.lock";

pub const IDENTIFY_FLASH_MS: u64 = 200;
pub const OVERLAY_SICK_FLASH_MS: u64 = 500;
pub const IDENTIFY_FRESH_SECS: u64 = 15;

/// Which lamps are on. Solid blue is not a topology color.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedRender {
    pub mix: LedMix,
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

/// Highest-wins ladder from 849.5.
pub fn classify(f: LedFacts) -> LedAction {
    if f.apply_in_progress {
        return LedAction::Hold;
    }
    let base = classify_topology(f);
    if f.identify {
        return LedAction::Drive(LedRender {
            mix: match base {
                LedAction::Drive(r) => r.mix,
                LedAction::Hold => LedMix::OFF,
            },
            flash_ms: Some(IDENTIFY_FLASH_MS),
        });
    }
    base
}

fn classify_topology(f: LedFacts) -> LedAction {
    let drive = |mix: LedMix, flash_ms: Option<u64>| {
        LedAction::Drive(LedRender { mix, flash_ms })
    };
    if !f.overlay_addr_present && (f.mesh_estab || f.mesh_l2_up) {
        return drive(LedMix::OVERLAY_SICK, Some(OVERLAY_SICK_FLASH_MS));
    }
    if f.local_egress {
        return drive(LedMix::EGRESS, None);
    }
    if f.default_via_mesh {
        return drive(LedMix::VIA_MESH, None);
    }
    if f.mesh_estab {
        return drive(LedMix::MESH_NO_NET, None);
    }
    if f.radios_up {
        return drive(LedMix::ALONE, None);
    }
    drive(LedMix::OFF, None)
}

/// Babel-learned or backhaul-dev default — satellite internet, not local WAN.
pub fn default_via_mesh(routes: &[DefaultRoute]) -> bool {
    routes.iter().any(|r| {
        r.proto_babel || EXCLUDED_EGRESS_IFACES.contains(&r.oif.as_str())
    })
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

/// Brightness for one flash phase. `on` is the first half of the period.
pub fn mix_for_phase(render: LedRender, on: bool) -> LedMix {
    match render.flash_ms {
        None => render.mix,
        Some(_) if on => render.mix,
        Some(_) => LedMix::OFF,
    }
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
            LedAction::Drive(r) => r.mix,
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
                assert_eq!(r.mix, LedMix::OVERLAY_SICK);
                assert_eq!(r.flash_ms, Some(OVERLAY_SICK_FLASH_MS));
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
        assert_eq!(
            mix(facts(|f| f.mesh_estab = true)),
            LedMix::MESH_NO_NET
        );
        assert!(!LedMix::MESH_NO_NET.blue);
    }

    #[test]
    fn alone_is_solid_red() {
        let r = match classify(facts(|_| {})) {
            LedAction::Drive(r) => r,
            LedAction::Hold => panic!(),
        };
        assert_eq!(r.mix, LedMix::ALONE);
        assert_eq!(r.flash_ms, None);
    }

    #[test]
    fn radios_idle_is_off() {
        assert_eq!(
            mix(facts(|f| f.radios_up = false)),
            LedMix::OFF
        );
    }

    #[test]
    fn identify_flashes_current_color() {
        match classify(facts(|f| {
            f.identify = true;
            f.mesh_estab = true;
        })) {
            LedAction::Drive(r) => {
                assert_eq!(r.mix, LedMix::MESH_NO_NET);
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
                let blue_only = r.mix.blue && !r.mix.red && !r.mix.amber;
                assert!(!blue_only, "solid blue leaked: {r:?}");
            }
        }
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
        let r = LedRender {
            mix: LedMix::EGRESS,
            flash_ms: Some(200),
        };
        assert_eq!(mix_for_phase(r, true), LedMix::EGRESS);
        assert_eq!(mix_for_phase(r, false), LedMix::OFF);
        let solid = LedRender {
            mix: LedMix::VIA_MESH,
            flash_ms: None,
        };
        assert_eq!(mix_for_phase(solid, false), LedMix::VIA_MESH);
    }
}
