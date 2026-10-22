//! Wired node-connection profiles (`add-wired-node-profiles`, bead
//! `mjolnir-mesh-ai0.6`).
//!
//! A pure decider and plan builder. It reads the UCI text it is handed, gives
//! every port and radio exactly one role, renders the desired allowlisted UCI
//! files, and returns one revision-checked [`Plan`]. It never reads the node,
//! writes a file, or talks to a radio. Every refusal happens here, before a
//! journal reaches `prepared`.
//!
//! A plan that changes port roles or a mesh radio requires management and mesh
//! reachability. The OpenWrt adapter reports those probes as `Unknown` until
//! bead `mjolnir-mesh-2mgv` lands, so [`submit_profile_plan`] accepts only an
//! adapter that implements [`ProvesReachability`]. The OpenWrt adapter does not.

use crate::engine::{ApplyAdapter, Engine, EngineError};
use crate::model::{
    DEFAULT_TIMEOUT_SECS, ObservationKind, Plan, Receipt, SCHEMA_VERSION, UciConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use thiserror::Error;

/// The client bridge. Each node owns its routed client `/24` here.
pub const CLIENT_BRIDGE: &str = "br-lan";
/// The bridge carrying the 802.11s backhaul (`mesh_fwding=1`). A cable in it
/// with a mesh radio up is an L2 loop through the house switch.
pub const MESH_BRIDGE: &str = "br-mesh";
/// Image directory holding per-model port maps, one `<model>.json` each.
pub const PORT_MAP_DIR: &str = "/usr/share/mjolnir/port-maps";
/// Ethernet recovery address of last resort. Never copied onto another node.
const RECOVERY_ADDRESS: &str = "192.168.1.1";
/// netifd interface-name prefix for node-link L3 interfaces.
const NODE_LINK_PREFIX: &str = "nl_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileId {
    EthernetGatewaySatellites,
    WiredCore,
    WiredCoreWithFallback,
}

impl ProfileId {
    /// Today's fleet split: 5 GHz mesh, 2.4 GHz client AP, Ethernet WAN on the
    /// gateway. A node with no `mjolnir.profile.id` is on this profile.
    pub const DEFAULT: Self = Self::EthernetGatewaySatellites;

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EthernetGatewaySatellites => "ethernet-gateway-satellites",
            Self::WiredCore => "wired-core",
            Self::WiredCoreWithFallback => "wired-core-with-fallback",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::EthernetGatewaySatellites,
            Self::WiredCore,
            Self::WiredCoreWithFallback,
        ]
        .into_iter()
        .find(|profile| profile.as_str() == value)
    }

    /// What losing the node-link cable does. Satellites have no node-link.
    pub fn cable_loss(self) -> Option<CableLoss> {
        match self {
            Self::EthernetGatewaySatellites => None,
            Self::WiredCore => Some(CableLoss::Isolate),
            Self::WiredCoreWithFallback => Some(CableLoss::ReserveMesh),
        }
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `isolate` buys both client bands and becomes a silent island on unplug;
/// `reserve-mesh` keeps one radio on mesh as fallback at the cost of a band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CableLoss {
    Isolate,
    ReserveMesh,
}

impl CableLoss {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Isolate => "isolate",
            Self::ReserveMesh => "reserve-mesh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortRole {
    Wan,
    NodeLink,
    Client,
    Disabled,
}

impl fmt::Display for PortRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wan => "wan",
            Self::NodeLink => "node-link",
            Self::Client => "client",
            Self::Disabled => "disabled",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadioRole {
    Mesh,
    ClientAp,
    Station,
    Disabled,
}

impl fmt::Display for RadioRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mesh => "mesh",
            Self::ClientAp => "client-ap",
            Self::Station => "station",
            Self::Disabled => "disabled",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Band {
    #[serde(rename = "2g")]
    TwoG,
    #[serde(rename = "5g")]
    FiveG,
    #[serde(rename = "6g")]
    SixG,
}

impl Band {
    fn from_uci(band: Option<&str>, hwmode: Option<&str>) -> Option<Self> {
        match (band, hwmode) {
            (Some("2g"), _) => Some(Self::TwoG),
            (Some("5g"), _) => Some(Self::FiveG),
            (Some("6g"), _) => Some(Self::SixG),
            (None, Some("11a")) => Some(Self::FiveG),
            (None, Some("11g" | "11b")) => Some(Self::TwoG),
            _ => None,
        }
    }

    fn is_client_band(self) -> bool {
        matches!(self, Self::TwoG | Self::FiveG)
    }
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TwoG => "2g",
            Self::FiveG => "5g",
            Self::SixG => "6g",
        })
    }
}

/// A per-model port map shipped as image data (`PORT_MAP_DIR/<model>.json`).
/// `qualified: true` is set only by hardware trials (`mjolnir-mesh-lpv`);
/// nothing here guesses a map from `board.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortMap {
    pub model: String,
    #[serde(default)]
    pub qualified: bool,
    /// Every physical Ethernet port on the model, by kernel name.
    pub ports: Vec<String>,
}

impl PortMap {
    pub fn parse(text: &str) -> Result<Self, ProfileRefusal> {
        let map: Self = serde_json::from_str(text)
            .map_err(|error| ProfileRefusal::InvalidPortMap(error.to_string()))?;
        if map.model.is_empty() {
            return Err(ProfileRefusal::InvalidPortMap("model is empty".into()));
        }
        if map.ports.is_empty() {
            return Err(ProfileRefusal::InvalidPortMap(format!(
                "{} lists no ports",
                map.model
            )));
        }
        let mut seen = BTreeSet::new();
        for port in &map.ports {
            if !is_ifname(port) {
                return Err(ProfileRefusal::InvalidPortMap(format!(
                    "{port:?} is not an interface name"
                )));
            }
            if !seen.insert(port) {
                return Err(ProfileRefusal::InvalidPortMap(format!(
                    "{port} is listed twice"
                )));
            }
        }
        Ok(map)
    }

    /// Image path of the map for `model`, or `None` for an unsafe model name.
    pub fn path_for(model: &str) -> Option<String> {
        let safe = !model.is_empty()
            && model != "."
            && model != ".."
            && model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b',' | b'-' | b'_' | b'.'));
        safe.then(|| format!("{PORT_MAP_DIR}/{model}.json"))
    }
}

fn is_ifname(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 16
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'@'))
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileRefusal {
    #[error("cannot parse {file}: {detail}")]
    Parse { file: &'static str, detail: String },
    #[error("invalid port map: {0}")]
    InvalidPortMap(String),
    #[error("{kind} {name} has more than one role: {roles}")]
    RoleConflict {
        kind: &'static str,
        name: String,
        roles: String,
    },
    #[error("{profile} needs a qualified port map for model {model}; {reason}")]
    UnqualifiedPortMap {
        profile: ProfileId,
        model: String,
        reason: &'static str,
    },
    #[error("port {port} is not in the port map for {model}")]
    UnknownPort { port: String, model: String },
    #[error("radio {0} does not exist on this node")]
    UnknownRadio(String),
    #[error("{profile} needs a {role} port")]
    MissingDomain { profile: ProfileId, role: PortRole },
    #[error("{profile} allows one WAN port, got {ports}")]
    MultipleWan { profile: ProfileId, ports: String },
    #[error("{profile} does not give {name} the {role} role")]
    RoleNotInProfile {
        profile: ProfileId,
        name: String,
        role: String,
    },
    #[error("no {0} radio carries the mesh backhaul")]
    NoRadioForBand(Band),
    #[error(
        "node mesh radio {radio} is {node}, the island is {island}; a backhaul band change is fleet-wide"
    )]
    IslandBandMismatch {
        radio: String,
        node: Band,
        island: Band,
    },
    #[error("radio {0} may be a station only when add-wifi-internet-uplink assigns it")]
    StationNotOwned(String),
    #[error("reserve-mesh keeps radio {0} on mesh; it cannot be a client AP")]
    ReservedRadioOnClient(String),
    #[error("isolate would strand mesh peers with no other path ({0}) without an acknowledgement")]
    WouldStrandPeers(String),
    #[error("radio {radio} has no wifi-iface for the {role} role")]
    NoIfaceForRole { radio: String, role: RadioRole },
    #[error("client port {0} would run babel")]
    ClientPortRunsBabel(String),
    #[error("node-link {0} would join {MESH_BRIDGE} with the mesh radio")]
    NodeLinkInMeshBridge(String),
    #[error("node-link {0} would serve or take DHCP")]
    NodeLinkDhcp(String),
    #[error("WAN {0} would be bridged into {CLIENT_BRIDGE}")]
    WanBridgedIntoClient(String),
    #[error("port {0} has no role but would stay in a bridge or interface")]
    UnassignedPortBridged(String),
    #[error("client bridge would reach other nodes through {0}")]
    ClientBridgeCrossesNodes(String),
    #[error("plan would add the {RECOVERY_ADDRESS} recovery address to {0}")]
    RecoveryAddressAdded(String),
    #[error("plan would remove the house-gateway default export ({0}) without saying so")]
    DefaultExportRemoved(String),
}

/// The four allowlisted UCI files as text. `None` is a missing file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeConfig {
    pub wireless: Option<String>,
    pub network: Option<String>,
    pub firewall: Option<String>,
    pub mjolnir: Option<String>,
}

impl NodeConfig {
    pub fn get(&self, config: UciConfig) -> Option<&str> {
        match config {
            UciConfig::Wireless => self.wireless.as_deref(),
            UciConfig::Network => self.network.as_deref(),
            UciConfig::Firewall => self.firewall.as_deref(),
            UciConfig::Mjolnir => self.mjolnir.as_deref(),
        }
    }

    fn slot(&mut self, config: UciConfig) -> &mut Option<String> {
        match config {
            UciConfig::Wireless => &mut self.wireless,
            UciConfig::Network => &mut self.network,
            UciConfig::Firewall => &mut self.firewall,
            UciConfig::Mjolnir => &mut self.mjolnir,
        }
    }

    /// The revision these files would have on a node. Byte-for-byte the same
    /// digest the OpenWrt adapter computes over the allowlisted files.
    pub fn revision(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        for config in UciConfig::ALL {
            hasher.update(config.file_name().as_bytes());
            match self.get(config) {
                Some(text) => {
                    hasher.update(b"\0present\0");
                    hasher.update(&(text.len() as u64).to_le_bytes());
                    hasher.update(text.as_bytes());
                }
                None => {
                    hasher.update(b"\0missing\0");
                }
            }
        }
        hasher.finalize().to_hex().to_string()
    }
}

/// What the decider knows about one node. All of it is handed in; nothing is
/// read from the node here.
#[derive(Debug, Clone)]
pub struct NodeFacts {
    pub model: String,
    pub port_map: Option<PortMap>,
    pub config: NodeConfig,
    /// Current mesh peers whose only path is this node's mesh radio.
    pub wireless_only_peers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProfileRequest {
    pub profile: ProfileId,
    /// Explicit port roles. Satellites keep today's ports unless overridden;
    /// wired profiles disable every mapped port not named here.
    pub ports: Vec<(String, PortRole)>,
    /// Explicit radio roles. Unnamed radios take the profile's default role.
    pub radios: Vec<(String, RadioRole)>,
    /// The island's backhaul band. Fleet-wide (`setup-wireless.sh BACKHAUL_BAND`).
    pub island_band: Band,
    /// The radio `add-wifi-internet-uplink` assigns as a station, if any.
    pub uplink_station_radio: Option<String>,
    /// Owner acknowledged that `isolate` strands wireless-only peers.
    pub acknowledge_stranded_peers: bool,
    /// The default-route export change is an explicit part of this plan.
    pub explicit_default_export_change: bool,
}

impl ProfileRequest {
    pub fn new(profile: ProfileId) -> Self {
        Self {
            profile,
            ports: Vec::new(),
            radios: Vec::new(),
            island_band: Band::FiveG,
            uplink_station_radio: None,
            acknowledge_stranded_peers: false,
            explicit_default_export_change: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDecision {
    pub profile: ProfileId,
    pub cable_loss: Option<CableLoss>,
    pub ports: BTreeMap<String, PortRole>,
    pub radios: BTreeMap<String, RadioRole>,
}

impl ProfileDecision {
    pub fn node_links(&self) -> Vec<String> {
        self.ports_with(PortRole::NodeLink)
    }

    fn ports_with(&self, role: PortRole) -> Vec<String> {
        self.ports
            .iter()
            .filter(|(_, r)| **r == role)
            .map(|(port, _)| port.clone())
            .collect()
    }

    /// The radio carrying mesh once the node-link cable is gone. Cable loss
    /// never re-roles a radio: under `isolate` the node does not retake a
    /// client radio for mesh, and under `reserve-mesh` the reserved radio
    /// already is mesh.
    pub fn mesh_radio_after_node_link_loss(&self) -> Option<&str> {
        self.radios
            .iter()
            .find(|(_, role)| **role == RadioRole::Mesh)
            .map(|(radio, _)| radio.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanIds {
    pub transaction_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePlan {
    pub plan: Plan,
    pub decision: ProfileDecision,
    /// The files the plan writes. Unchanged files keep their exact bytes.
    pub desired: NodeConfig,
}

impl ProfilePlan {
    pub fn is_noop(&self) -> bool {
        self.plan.expected_revision == self.plan.proposed_revision
    }
}

/// An adapter that can prove management and mesh reachability from a real
/// vantage. Under `isolate`, mesh reachability means a babel neighbour over the
/// node-link, not an 802.11s peer (bead `mjolnir-mesh-2mgv`). The OpenWrt
/// adapter does not implement this, so a profile plan cannot reach it.
pub trait ProvesReachability: ApplyAdapter {}

/// Submit a profile plan as one network-change transaction.
pub fn submit_profile_plan<A: ProvesReachability>(
    engine: &Engine,
    profile_plan: &ProfilePlan,
    adapter: &mut A,
) -> Result<Receipt, EngineError> {
    engine.apply(&profile_plan.plan, adapter)
}

/// Decide one role for every port and radio. Pure.
pub fn decide(
    request: &ProfileRequest,
    node: &NodeFacts,
) -> Result<ProfileDecision, ProfileRefusal> {
    let docs = Docs::parse(&node.config)?;
    let requested_ports = single_roles("port", &request.ports)?;
    let requested_radios = single_roles("radio", &request.radios)?;

    let bands = radio_bands(&docs.wireless);
    for radio in requested_radios.keys() {
        if !bands.contains_key(radio) {
            return Err(ProfileRefusal::UnknownRadio(radio.clone()));
        }
    }
    let owned_station = request.uplink_station_radio.as_deref();
    for (radio, role) in &requested_radios {
        if *role == RadioRole::Station && owned_station != Some(radio.as_str()) {
            return Err(ProfileRefusal::StationNotOwned(radio.clone()));
        }
    }
    let current_radios = radio_roles(&docs.wireless);
    for (radio, roles) in &current_radios {
        // z3th: refuse rather than clobber a live uplink this plan does not own.
        if roles.contains(&RadioRole::Station) && owned_station != Some(radio.as_str()) {
            return Err(ProfileRefusal::StationNotOwned(radio.clone()));
        }
        // Advise N5: the backhaul band is island-wide, not a per-node choice.
        if roles.contains(&RadioRole::Mesh)
            && let Some(Some(band)) = bands.get(radio)
            && *band != request.island_band
        {
            return Err(ProfileRefusal::IslandBandMismatch {
                radio: radio.clone(),
                node: *band,
                island: request.island_band,
            });
        }
    }
    let island_radio = bands
        .iter()
        .find(|(_, band)| **band == Some(request.island_band))
        .map(|(radio, _)| radio.clone());

    let profile = request.profile;
    let ports = match profile {
        ProfileId::EthernetGatewaySatellites => {
            let mut ports = single_roles_of("port", port_roles(&docs.network, &docs.mjolnir))?;
            for (port, role) in requested_ports {
                if role == PortRole::NodeLink {
                    return Err(ProfileRefusal::RoleNotInProfile {
                        profile,
                        name: port,
                        role: role.to_string(),
                    });
                }
                ports.insert(port, role);
            }
            ports
        }
        ProfileId::WiredCore | ProfileId::WiredCoreWithFallback => {
            let map = qualified_map(profile, node)?;
            let mut ports: BTreeMap<String, PortRole> = map
                .ports
                .iter()
                .map(|port| (port.clone(), PortRole::Disabled))
                .collect();
            for (port, role) in requested_ports {
                if !ports.contains_key(&port) {
                    return Err(ProfileRefusal::UnknownPort {
                        port,
                        model: map.model.clone(),
                    });
                }
                ports.insert(port, role);
            }
            for role in [PortRole::Wan, PortRole::NodeLink, PortRole::Client] {
                if !ports.values().any(|r| *r == role) {
                    return Err(ProfileRefusal::MissingDomain { profile, role });
                }
            }
            ports
        }
    };
    let wans: Vec<&str> = ports
        .iter()
        .filter(|(_, role)| **role == PortRole::Wan)
        .map(|(port, _)| port.as_str())
        .collect();
    if wans.len() > 1 {
        return Err(ProfileRefusal::MultipleWan {
            profile,
            ports: wans.join(", "),
        });
    }

    // The radio that stays on mesh: always for satellites, the reserved band
    // for `reserve-mesh`, none for `isolate`.
    let mesh_radio = match profile.cable_loss() {
        Some(CableLoss::Isolate) => None,
        None | Some(CableLoss::ReserveMesh) => Some(
            island_radio
                .clone()
                .ok_or(ProfileRefusal::NoRadioForBand(request.island_band))?,
        ),
    };
    let mut radios = BTreeMap::new();
    for (radio, band) in &bands {
        let role = if mesh_radio.as_deref() == Some(radio.as_str()) {
            RadioRole::Mesh
        } else if owned_station == Some(radio.as_str()) {
            RadioRole::Station
        } else if band.is_some_and(Band::is_client_band) {
            RadioRole::ClientAp
        } else {
            RadioRole::Disabled
        };
        radios.insert(radio.clone(), role);
    }
    for (radio, role) in requested_radios {
        let is_mesh_radio = mesh_radio.as_deref() == Some(radio.as_str());
        if is_mesh_radio && role != RadioRole::Mesh {
            if profile.cable_loss() == Some(CableLoss::ReserveMesh) {
                return Err(ProfileRefusal::ReservedRadioOnClient(radio));
            }
            return Err(ProfileRefusal::RoleNotInProfile {
                profile,
                name: radio,
                role: role.to_string(),
            });
        }
        if !is_mesh_radio && role == RadioRole::Mesh {
            return Err(ProfileRefusal::RoleNotInProfile {
                profile,
                name: radio,
                role: role.to_string(),
            });
        }
        radios.insert(radio, role);
    }
    if mesh_radio.is_none()
        && !node.wireless_only_peers.is_empty()
        && !request.acknowledge_stranded_peers
    {
        return Err(ProfileRefusal::WouldStrandPeers(
            node.wireless_only_peers.join(", "),
        ));
    }

    Ok(ProfileDecision {
        profile,
        cable_loss: profile.cable_loss(),
        ports,
        radios,
    })
}

fn qualified_map(profile: ProfileId, node: &NodeFacts) -> Result<&PortMap, ProfileRefusal> {
    let refuse = |reason| ProfileRefusal::UnqualifiedPortMap {
        profile,
        model: node.model.clone(),
        reason,
    };
    let map = node
        .port_map
        .as_ref()
        .ok_or_else(|| refuse("no port map ships for it"))?;
    if map.model != node.model {
        return Err(refuse("the port map names another model"));
    }
    if !map.qualified {
        return Err(refuse("its port map is not qualified (mjolnir-mesh-lpv)"));
    }
    Ok(map)
}

/// Decide, render, check, and wrap one revision-checked plan. Pure.
pub fn build_plan(
    request: &ProfileRequest,
    node: &NodeFacts,
    ids: &PlanIds,
) -> Result<ProfilePlan, ProfileRefusal> {
    let decision = decide(request, node)?;
    let current = Docs::parse(&node.config)?;
    let mut desired = current.clone();
    render_ports(&mut desired, &decision);
    render_radios(&mut desired.wireless, &decision)?;
    render_mjolnir(&mut desired.mjolnir, &current.mjolnir, &decision);
    check_docs(
        &current,
        &desired,
        &decision,
        request.explicit_default_export_change,
    )?;

    let mut desired_config = node.config.clone();
    for config in UciConfig::ALL {
        if current.get(config) != desired.get(config) {
            *desired_config.slot(config) = Some(desired.get(config).render());
        }
    }
    let plan = Plan {
        schema_version: SCHEMA_VERSION,
        transaction_id: ids.transaction_id.clone(),
        node_id: ids.node_id.clone(),
        expected_revision: node.config.revision(),
        proposed_revision: desired_config.revision(),
        resources: UciConfig::ALL.to_vec(),
        timeout_secs: DEFAULT_TIMEOUT_SECS,
        required_health: required_health(&current, &desired),
    };
    Ok(ProfilePlan {
        plan,
        decision,
        desired: desired_config,
    })
}

/// Refuse a desired configuration that breaks a loop or gateway rule. Run by
/// [`build_plan`]; public so any producer of profile files is held to it.
pub fn check_profile_safety(
    current: &NodeConfig,
    desired: &NodeConfig,
    decision: &ProfileDecision,
    explicit_default_export_change: bool,
) -> Result<(), ProfileRefusal> {
    check_docs(
        &Docs::parse(current)?,
        &Docs::parse(desired)?,
        decision,
        explicit_default_export_change,
    )
}

fn check_docs(
    current: &Docs,
    desired: &Docs,
    decision: &ProfileDecision,
    explicit_default_export_change: bool,
) -> Result<(), ProfileRefusal> {
    let net = &desired.network;
    let lan_ports = bridge_ports(net, CLIENT_BRIDGE);
    let mesh_ports = bridge_ports(net, MESH_BRIDGE);
    let node_links = node_link_names(&desired.mjolnir);
    let is_client = |name: &str| {
        name == CLIENT_BRIDGE
            || lan_ports.iter().any(|p| p == name)
            || decision.ports.get(name) == Some(&PortRole::Client)
    };

    // Client LANs stay per-node routed prefixes.
    for port in &lan_ports {
        if port == MESH_BRIDGE || mesh_ports.contains(port) {
            return Err(ProfileRefusal::ClientBridgeCrossesNodes(port.clone()));
        }
    }
    if net.named("lan").and_then(|s| s.get("device")) == Some(MESH_BRIDGE) {
        return Err(ProfileRefusal::ClientBridgeCrossesNodes(MESH_BRIDGE.into()));
    }
    for iface in desired.wireless.of_kind("wifi-iface") {
        if iface_enabled(iface)
            && iface.get("mode") == Some("mesh")
            && iface.words("network").any(|n| n == "lan")
        {
            return Err(ProfileRefusal::ClientBridgeCrossesNodes(format!(
                "wireless.{}",
                iface.name.as_deref().unwrap_or("@wifi-iface")
            )));
        }
    }

    // Client ports do not run babel.
    for link in &node_links {
        if is_client(link) {
            return Err(ProfileRefusal::ClientPortRunsBabel(link.clone()));
        }
    }
    if let Some(backhaul) = desired
        .mjolnir
        .named("meshd")
        .and_then(|s| s.get("backhaul_iface"))
        && is_client(backhaul)
    {
        return Err(ProfileRefusal::ClientPortRunsBabel(backhaul.to_owned()));
    }

    // A node-link is its own routed L3 interface: never in br-mesh, no DHCP.
    for link in &node_links {
        if link == MESH_BRIDGE || mesh_ports.contains(link) {
            return Err(ProfileRefusal::NodeLinkInMeshBridge(link.clone()));
        }
        for iface in net.of_kind("interface") {
            if iface.get("device") == Some(link.as_str())
                && (iface.get("proto") != Some("none")
                    || matches!(iface.name.as_deref(), Some("lan" | "wan" | "wan6")))
            {
                return Err(ProfileRefusal::NodeLinkDhcp(link.clone()));
            }
        }
    }

    // WAN is not bridged into the client bridge.
    for wan in wan_devices(net) {
        if wan == CLIENT_BRIDGE || lan_ports.contains(&wan) {
            return Err(ProfileRefusal::WanBridgedIntoClient(wan));
        }
    }

    // A port the profile leaves without a role is disabled, not inherited.
    for port in decision.ports_with(PortRole::Disabled) {
        let in_interface = net
            .of_kind("interface")
            .any(|iface| iface.get("device") == Some(port.as_str()));
        if lan_ports.contains(&port) || mesh_ports.contains(&port) || in_interface {
            return Err(ProfileRefusal::UnassignedPortBridged(port));
        }
    }

    // The recovery address is never copied onto a node that lacks it.
    if !carries_recovery_address(&current.network) {
        for iface in net.of_kind("interface") {
            if section_addresses(iface).any(is_recovery_address) {
                return Err(ProfileRefusal::RecoveryAddressAdded(format!(
                    "network.{}",
                    iface.name.as_deref().unwrap_or("@interface")
                )));
            }
        }
    }

    // A house-gateway default export does not vanish as a side effect.
    if !explicit_default_export_change {
        let gateway = |docs: &Docs| {
            docs.mjolnir
                .named("meshd")
                .and_then(|s| s.get("gateway"))
                .unwrap_or("auto")
                .to_owned()
        };
        if gateway(current) != "never" && gateway(desired) == "never" {
            return Err(ProfileRefusal::DefaultExportRemoved(
                "mjolnir.meshd.gateway".into(),
            ));
        }
        let live_wan = |docs: &Docs| {
            docs.network.named("wan").is_some_and(|wan| {
                wan.get("device").is_some()
                    && wan.get("disabled") != Some("1")
                    && wan.get("auto") != Some("0")
            })
        };
        if live_wan(current) && !live_wan(desired) {
            return Err(ProfileRefusal::DefaultExportRemoved("network.wan".into()));
        }
    }

    // One role per port and radio in what would be written.
    single_roles_of("port", port_roles(net, &desired.mjolnir))?;
    single_roles_of("radio", radio_roles(&desired.wireless))?;
    Ok(())
}

/// `AppliedRevision` always. A plan that changes port roles or a mesh radio
/// also needs management and mesh reachability before commit.
fn required_health(current: &Docs, desired: &Docs) -> Vec<ObservationKind> {
    let ports_changed = port_roles(&current.network, &current.mjolnir)
        != port_roles(&desired.network, &desired.mjolnir)
        || bridge_ports(&current.network, MESH_BRIDGE)
            != bridge_ports(&desired.network, MESH_BRIDGE);
    let before = radio_roles(&current.wireless);
    let after = radio_roles(&desired.wireless);
    let radios: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let mesh_changed = radios.into_iter().any(|radio| {
        let (b, a) = (before.get(radio), after.get(radio));
        let touches_mesh = [b, a]
            .into_iter()
            .flatten()
            .any(|roles| roles.contains(&RadioRole::Mesh));
        touches_mesh && b != a
    });
    let mut kinds = vec![ObservationKind::AppliedRevision];
    if ports_changed || mesh_changed {
        kinds.push(ObservationKind::ManagementReachability);
        kinds.push(ObservationKind::MeshReachability);
    }
    kinds
}

fn single_roles<R: Copy + Ord + fmt::Display>(
    kind: &'static str,
    assignments: &[(String, R)],
) -> Result<BTreeMap<String, R>, ProfileRefusal> {
    let mut grouped: BTreeMap<String, BTreeSet<R>> = BTreeMap::new();
    for (name, role) in assignments {
        grouped.entry(name.clone()).or_default().insert(*role);
    }
    single_roles_of(kind, grouped)
}

fn single_roles_of<R: Copy + Ord + fmt::Display>(
    kind: &'static str,
    grouped: BTreeMap<String, BTreeSet<R>>,
) -> Result<BTreeMap<String, R>, ProfileRefusal> {
    grouped
        .into_iter()
        .map(|(name, roles)| {
            let mut iter = roles.iter();
            match (iter.next(), iter.next()) {
                (Some(role), None) => Ok((name, *role)),
                _ => Err(ProfileRefusal::RoleConflict {
                    kind,
                    name,
                    roles: roles
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" + "),
                }),
            }
        })
        .collect()
}

/// Roles the network and mjolnir files give each port today.
fn port_roles(network: &UciDoc, mjolnir: &UciDoc) -> BTreeMap<String, BTreeSet<PortRole>> {
    let mut roles: BTreeMap<String, BTreeSet<PortRole>> = BTreeMap::new();
    for port in bridge_ports(network, CLIENT_BRIDGE) {
        roles.entry(port).or_default().insert(PortRole::Client);
    }
    for port in wan_devices(network) {
        roles.entry(port).or_default().insert(PortRole::Wan);
    }
    for port in node_link_names(mjolnir) {
        roles.entry(port).or_default().insert(PortRole::NodeLink);
    }
    roles
}

fn radio_bands(wireless: &UciDoc) -> BTreeMap<String, Option<Band>> {
    wireless
        .of_kind("wifi-device")
        .filter_map(|dev| {
            let band = Band::from_uci(dev.get("band"), dev.get("hwmode"));
            dev.name.clone().map(|name| (name, band))
        })
        .collect()
}

/// Roles the enabled wifi-ifaces give each radio today.
fn radio_roles(wireless: &UciDoc) -> BTreeMap<String, BTreeSet<RadioRole>> {
    let mut roles: BTreeMap<String, BTreeSet<RadioRole>> = BTreeMap::new();
    let mut off = BTreeSet::new();
    for dev in wireless.of_kind("wifi-device") {
        if let Some(name) = &dev.name {
            roles.entry(name.clone()).or_default();
            if dev.get("disabled") == Some("1") {
                off.insert(name.clone());
            }
        }
    }
    for iface in wireless.of_kind("wifi-iface").filter(|i| iface_enabled(i)) {
        let Some(radio) = iface.get("device") else {
            continue;
        };
        if off.contains(radio) {
            continue;
        }
        let role = match iface.get("mode") {
            Some("mesh") => RadioRole::Mesh,
            Some("ap") => RadioRole::ClientAp,
            Some("sta") => RadioRole::Station,
            _ => continue,
        };
        roles.entry(radio.to_owned()).or_default().insert(role);
    }
    for set in roles.values_mut() {
        if set.is_empty() {
            set.insert(RadioRole::Disabled);
        }
    }
    roles
}

fn iface_enabled(iface: &UciSection) -> bool {
    iface.get("disabled") != Some("1")
}

fn bridge_ports(network: &UciDoc, bridge: &str) -> Vec<String> {
    network
        .of_kind("device")
        .filter(|dev| dev.get("name") == Some(bridge))
        .flat_map(|dev| dev.list("ports"))
        .map(str::to_owned)
        .collect()
}

fn wan_devices(network: &UciDoc) -> Vec<String> {
    let mut devices: Vec<String> = ["wan", "wan6"]
        .into_iter()
        .filter_map(|name| network.named(name)?.get("device"))
        .map(str::to_owned)
        .collect();
    devices.dedup();
    devices
}

fn node_link_names(mjolnir: &UciDoc) -> Vec<String> {
    mjolnir
        .named("node_link")
        .map(|s| s.list("ifname").map(str::to_owned).collect())
        .unwrap_or_default()
}

fn node_link_interface(port: &str) -> String {
    let safe: String = port
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{NODE_LINK_PREFIX}{safe}")
}

fn section_addresses(iface: &UciSection) -> impl Iterator<Item = &str> {
    iface
        .entries
        .iter()
        .filter(|e| e.key == "ipaddr")
        .flat_map(|e| e.value.split_whitespace())
}

fn is_recovery_address(address: &str) -> bool {
    address
        .strip_prefix(RECOVERY_ADDRESS)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn carries_recovery_address(network: &UciDoc) -> bool {
    network
        .of_kind("interface")
        .any(|iface| section_addresses(iface).any(is_recovery_address))
}

fn render_ports(docs: &mut Docs, decision: &ProfileDecision) {
    let role = |port: &str| decision.ports.get(port).copied();
    let net = &mut docs.network;

    // Client bridge: keep unmapped members and client ports, in order.
    let mut lan: Vec<String> = bridge_ports(net, CLIENT_BRIDGE)
        .into_iter()
        .filter(|p| matches!(role(p), None | Some(PortRole::Client)))
        .collect();
    for port in decision.ports_with(PortRole::Client) {
        if !lan.contains(&port) {
            lan.push(port);
        }
    }
    let lan_section = match net.bridge_mut(CLIENT_BRIDGE) {
        Some(section) => Some(section),
        None if !lan.is_empty() => Some(net.create_bridge("br_lan", CLIENT_BRIDGE)),
        None => None,
    };
    if let Some(section) = lan_section {
        section.set_list("ports", &lan);
    }

    // Mesh bridge: no port with a role rides the 802.11s L2.
    if let Some(section) = net.bridge_mut(MESH_BRIDGE) {
        let kept: Vec<String> = section
            .list("ports")
            .filter(|p| role(p).is_none())
            .map(str::to_owned)
            .collect();
        section.set_list("ports", &kept);
    }

    // WAN: the one WAN port, else drop a device that now has another role.
    let wan = decision.ports_with(PortRole::Wan).into_iter().next();
    for name in ["wan", "wan6"] {
        let Some(section) = net.named_mut(name) else {
            continue;
        };
        match &wan {
            Some(port) => section.set("device", port),
            None => {
                if section.get("device").is_some_and(|d| role(d).is_some()) {
                    section.remove("device");
                }
            }
        }
    }
    if let Some(port) = &wan
        && net.named("wan").is_none()
    {
        let section = net.section_mut("interface", "wan");
        section.set("device", port);
        section.set("proto", "dhcp");
    }

    // Node-links: one unmanaged L3 interface each; meshd addresses it.
    let links = decision.node_links();
    let wanted: BTreeSet<String> = links.iter().map(|p| node_link_interface(p)).collect();
    net.sections.retain(|s| {
        s.kind != "interface"
            || !s
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with(NODE_LINK_PREFIX) && !wanted.contains(n))
    });
    for port in &links {
        let section = net.section_mut("interface", &node_link_interface(port));
        section.set("device", port);
        section.set("proto", "none");
    }

    // Any other interface still bound to a port that lost its role.
    for iface in net.sections.iter_mut().filter(|s| s.kind == "interface") {
        let is_wan = matches!(iface.name.as_deref(), Some("wan" | "wan6"));
        let is_link = iface
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with(NODE_LINK_PREFIX));
        if !is_wan
            && !is_link
            && iface
                .get("device")
                .is_some_and(|d| role(d).is_some_and(|r| r != PortRole::Client))
        {
            iface.remove("device");
        }
    }

    // Node-link interfaces sit in the lan firewall zone, like `mesh`, so babel
    // hellos and transit are not rejected.
    let fw = &mut docs.firewall;
    if let Some(zone) = fw
        .sections
        .iter_mut()
        .find(|s| s.kind == "zone" && s.get("name") == Some("lan"))
    {
        let mut networks: Vec<String> = zone
            .list("network")
            .filter(|n| !n.starts_with(NODE_LINK_PREFIX) || wanted.contains(*n))
            .map(str::to_owned)
            .collect();
        for name in &wanted {
            if !networks.contains(name) {
                networks.push(name.clone());
            }
        }
        zone.set_list("network", &networks);
    }
}

fn render_radios(wireless: &mut UciDoc, decision: &ProfileDecision) -> Result<(), ProfileRefusal> {
    for (radio, role) in &decision.radios {
        let wanted_mode = match role {
            RadioRole::Mesh => Some("mesh"),
            RadioRole::ClientAp => Some("ap"),
            RadioRole::Station => Some("sta"),
            RadioRole::Disabled => None,
        };
        let on_radio = |s: &UciSection| s.kind == "wifi-iface" && s.get("device") == Some(radio);
        let is_wanted = |s: &UciSection| {
            wanted_mode.is_some_and(|mode| s.get("mode") == Some(mode))
                && (*role != RadioRole::ClientAp || s.words("network").any(|n| n == "lan"))
        };
        let candidates = wireless
            .sections
            .iter()
            .filter(|s| on_radio(s) && is_wanted(s))
            .count();
        if wanted_mode.is_some() && candidates == 0 {
            return Err(ProfileRefusal::NoIfaceForRole {
                radio: radio.clone(),
                role: *role,
            });
        }
        let already_on = wireless
            .sections
            .iter()
            .any(|s| on_radio(s) && is_wanted(s) && iface_enabled(s));
        for section in wireless.sections.iter_mut().filter(|s| on_radio(s)) {
            if is_wanted(section) {
                // Station ifaces belong to the uplink plan; never enabled here.
                if !already_on && *role != RadioRole::Station {
                    section.set_enabled(true);
                }
            } else {
                section.set_enabled(false);
            }
        }
        if *role != RadioRole::Disabled
            && let Some(dev) = wireless.named_mut(radio)
            && dev.get("disabled") == Some("1")
        {
            dev.set("disabled", "0");
        }
    }
    Ok(())
}

/// Record the profile id and the node-link names in the same plan. A node with
/// no `profile` section is on the default profile, so planning the default
/// onto today's config writes nothing.
fn render_mjolnir(mjolnir: &mut UciDoc, current: &UciDoc, decision: &ProfileDecision) {
    let current_profile = current
        .named("profile")
        .and_then(|s| s.get("id"))
        .and_then(ProfileId::parse)
        .unwrap_or(ProfileId::DEFAULT);
    if decision.profile != current_profile || current.named("profile").is_some() {
        let section = mjolnir.section_mut("profile", "profile");
        section.set("id", decision.profile.as_str());
        match decision.cable_loss {
            Some(mode) => section.set("cable_loss", mode.as_str()),
            None => section.remove("cable_loss"),
        }
    }
    let links = decision.node_links();
    if links.is_empty() {
        mjolnir
            .sections
            .retain(|s| s.name.as_deref() != Some("node_link"));
    } else {
        mjolnir
            .section_mut("node_link", "node_link")
            .set_list("ifname", &links);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Docs {
    wireless: UciDoc,
    network: UciDoc,
    firewall: UciDoc,
    mjolnir: UciDoc,
}

impl Docs {
    fn parse(config: &NodeConfig) -> Result<Self, ProfileRefusal> {
        let parse = |c: UciConfig| UciDoc::parse(c.file_name(), config.get(c).unwrap_or(""));
        Ok(Self {
            wireless: parse(UciConfig::Wireless)?,
            network: parse(UciConfig::Network)?,
            firewall: parse(UciConfig::Firewall)?,
            mjolnir: parse(UciConfig::Mjolnir)?,
        })
    }

    fn get(&self, config: UciConfig) -> &UciDoc {
        match config {
            UciConfig::Wireless => &self.wireless,
            UciConfig::Network => &self.network,
            UciConfig::Firewall => &self.firewall,
            UciConfig::Mjolnir => &self.mjolnir,
        }
    }
}

/// Minimal UCI document: enough to read and rewrite the allowlisted files. A
/// rewritten file is canonical, comment-free text, as `uci commit` writes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct UciDoc {
    sections: Vec<UciSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UciSection {
    kind: String,
    name: Option<String>,
    entries: Vec<UciEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UciEntry {
    key: String,
    value: String,
    list: bool,
}

impl UciDoc {
    fn parse(file: &'static str, text: &str) -> Result<Self, ProfileRefusal> {
        let mut sections: Vec<UciSection> = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let error = |detail: String| ProfileRefusal::Parse {
                file,
                detail: format!("line {}: {detail}", index + 1),
            };
            let words = split_words(line).map_err(error)?;
            let Some((head, rest)) = words.split_first() else {
                continue;
            };
            match (head.as_str(), rest) {
                ("package", [_]) => {}
                ("config", [kind]) => sections.push(UciSection::new(kind, None)),
                ("config", [kind, name]) => {
                    sections.push(UciSection::new(kind, Some(name.as_str())))
                }
                ("option" | "list", [key, value]) => {
                    let section = sections
                        .last_mut()
                        .ok_or_else(|| error(format!("{head} before any config section")))?;
                    section.entries.push(UciEntry {
                        key: key.clone(),
                        value: value.clone(),
                        list: head == "list",
                    });
                }
                _ => return Err(error(format!("unexpected `{}`", line.trim()))),
            }
        }
        Ok(Self { sections })
    }

    fn render(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            out.push_str("\nconfig ");
            out.push_str(&section.kind);
            if let Some(name) = &section.name {
                out.push(' ');
                out.push_str(&quote(name));
            }
            out.push('\n');
            for entry in &section.entries {
                out.push('\t');
                out.push_str(if entry.list { "list " } else { "option " });
                out.push_str(&entry.key);
                out.push(' ');
                out.push_str(&quote(&entry.value));
                out.push('\n');
            }
        }
        out
    }

    fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a UciSection> {
        self.sections.iter().filter(move |s| s.kind == kind)
    }

    fn named(&self, name: &str) -> Option<&UciSection> {
        self.sections
            .iter()
            .find(|s| s.name.as_deref() == Some(name))
    }

    fn named_mut(&mut self, name: &str) -> Option<&mut UciSection> {
        self.sections
            .iter_mut()
            .find(|s| s.name.as_deref() == Some(name))
    }

    fn section_mut(&mut self, kind: &str, name: &str) -> &mut UciSection {
        let index = match self
            .sections
            .iter()
            .position(|s| s.name.as_deref() == Some(name))
        {
            Some(index) => index,
            None => {
                self.sections.push(UciSection::new(kind, Some(name)));
                self.sections.len() - 1
            }
        };
        &mut self.sections[index]
    }

    fn bridge_mut(&mut self, bridge: &str) -> Option<&mut UciSection> {
        self.sections
            .iter_mut()
            .find(|s| s.kind == "device" && s.get("name") == Some(bridge))
    }

    fn create_bridge(&mut self, section: &str, bridge: &str) -> &mut UciSection {
        let created = self.section_mut("device", section);
        created.set("name", bridge);
        created.set("type", "bridge");
        created
    }
}

impl UciSection {
    fn new(kind: &str, name: Option<&str>) -> Self {
        Self {
            kind: kind.to_owned(),
            name: name.map(str::to_owned),
            entries: Vec::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|e| !e.list && e.key == key)
            .map(|e| e.value.as_str())
    }

    fn list<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.entries
            .iter()
            .filter(move |e| e.list && e.key == key)
            .map(|e| e.value.as_str())
    }

    /// Space-separated option value or list entries (`option network 'lan wwan'`).
    fn words<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.entries
            .iter()
            .filter(move |e| e.key == key)
            .flat_map(|e| e.value.split_whitespace())
    }

    fn set(&mut self, key: &str, value: &str) {
        let mut found = false;
        self.entries.retain_mut(|e| {
            if e.list || e.key != key {
                return true;
            }
            if found {
                return false;
            }
            found = true;
            e.value = value.to_owned();
            true
        });
        if !found {
            self.entries.push(UciEntry {
                key: key.to_owned(),
                value: value.to_owned(),
                list: false,
            });
        }
    }

    fn remove(&mut self, key: &str) {
        self.entries.retain(|e| e.list || e.key != key);
    }

    /// Replace a list in place, at the position of its first entry.
    fn set_list(&mut self, key: &str, values: &[String]) {
        let at = self
            .entries
            .iter()
            .position(|e| e.list && e.key == key)
            .unwrap_or(self.entries.len());
        let before = self.entries[..at]
            .iter()
            .filter(|e| !(e.list && e.key == key))
            .count();
        self.entries.retain(|e| !(e.list && e.key == key));
        let fresh = values.iter().map(|value| UciEntry {
            key: key.to_owned(),
            value: value.clone(),
            list: true,
        });
        self.entries.splice(before..before, fresh);
    }

    fn set_enabled(&mut self, enabled: bool) {
        match (enabled, self.get("disabled") == Some("1")) {
            (true, true) => self.set("disabled", "0"),
            (false, false) => self.set("disabled", "1"),
            _ => {}
        }
    }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

fn split_words(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        match chars.peek() {
            None | Some('#') => break,
            Some(_) => {}
        }
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                break;
            }
            chars.next();
            match c {
                '\'' | '"' => loop {
                    match chars.next() {
                        Some(q) if q == c => break,
                        Some('\\') if c == '"' => word.extend(chars.next()),
                        Some(other) => word.push(other),
                        None => return Err("unterminated quote".into()),
                    }
                },
                '\\' => word.extend(chars.next()),
                _ => word.push(c),
            }
        }
        words.push(word);
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Runtime;
    use crate::model::{Observation, ObservationStatus, Outcome, Vantage};
    use crate::storage::TxnPaths;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// The shipped `/etc/config/mjolnir` (comments and all).
    const MJOLNIR: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/openwrt/files/etc/config/mjolnir"
    ));

    /// `setup-wireless.sh` output with the default `BACKHAUL_BAND=5g`.
    const WIRELESS: &str = "
config wifi-device 'radio0'
	option type 'mac80211'
	option path 'platform/soc/18000000.wifi'
	option band '2g'
	option htmode 'HE20'
	option channel '6'
	option country 'US'
	option disabled '0'

config wifi-device 'radio1'
	option type 'mac80211'
	option path 'platform/soc/18000000.wifi+1'
	option band '5g'
	option htmode 'HE80'
	option channel '36'
	option country 'US'
	option disabled '0'

config wifi-iface 'meshbh'
	option device 'radio1'
	option mode 'mesh'
	option mesh_id 'mjolnir-mesh'
	option network 'mesh'
	option mesh_fwding '1'
	option encryption 'none'

config wifi-iface 'clientap'
	option device 'radio0'
	option mode 'ap'
	option ssid '⚡'
	option network 'lan'
	option encryption 'none'

config wifi-iface 'clientap2g'
	option disabled '1'
	option device 'radio1'
	option mode 'ap'
	option ssid '⚡'
	option network 'lan'
	option encryption 'none'
";

    /// A gateway with switch ports and the Ethernet recovery address.
    const GATEWAY_NETWORK: &str = "
config interface 'loopback'
	option device 'lo'
	option proto 'static'
	option ipaddr '127.0.0.1'
	option netmask '255.0.0.0'

config globals 'globals'
	option ula_prefix 'fd12:3456:789a::/48'

config device
	option name 'br-lan'
	option type 'bridge'
	list ports 'lan1'
	list ports 'lan2'
	list ports 'lan3'
	list ports 'lan4'

config interface 'lan'
	option device 'br-lan'
	option proto 'static'
	option ipaddr '192.168.1.1'
	option netmask '255.255.255.0'

config interface 'wan'
	option device 'wan'
	option proto 'dhcp'

config interface 'wan6'
	option device 'wan'
	option proto 'dhcpv6'

config interface 'mesh'
	option proto 'none'
	option device 'br-mesh'

config device 'br_mesh'
	option name 'br-mesh'
	option type 'bridge'
";

    /// Cudy AP3000 Outdoor v1 as observed: `eth0` WAN, no client Ethernet on
    /// `br-lan`, no recovery address.
    const AP3000_NETWORK: &str = "
config interface 'loopback'
	option device 'lo'
	option proto 'static'
	option ipaddr '127.0.0.1'
	option netmask '255.0.0.0'

config device
	option name 'br-lan'
	option type 'bridge'

config interface 'lan'
	option device 'br-lan'
	option proto 'static'
	option ipaddr '10.42.7.1'
	option netmask '255.255.255.0'

config interface 'wan'
	option device 'eth0'
	option proto 'dhcp'

config interface 'mesh'
	option proto 'none'
	option device 'br-mesh'

config device 'br_mesh'
	option name 'br-mesh'
	option type 'bridge'
";

    const FIREWALL: &str = "
config defaults
	option input 'REJECT'
	option output 'ACCEPT'
	option forward 'REJECT'

config zone
	option name 'lan'
	list network 'lan'
	list network 'mesh'
	option input 'ACCEPT'
	option output 'ACCEPT'
	option forward 'ACCEPT'

config zone
	option name 'wan'
	list network 'wan'
	list network 'wan6'
	option input 'REJECT'
	option output 'ACCEPT'
	option forward 'REJECT'
	option masq '1'
";

    /// A test-only map. Image fixtures ship `qualified: false`.
    const TEST_MAP: &str = r#"{"model": "test,wired-board", "qualified": true,
        "ports": ["wan", "lan1", "lan2", "lan3", "lan4"]}"#;
    const AP3000_MAP: &str =
        r#"{"model": "cudy,ap3000outdoor-v1", "qualified": false, "ports": ["eth0"]}"#;

    fn config(network: &str) -> NodeConfig {
        NodeConfig {
            wireless: Some(WIRELESS.into()),
            network: Some(network.into()),
            firewall: Some(FIREWALL.into()),
            mjolnir: Some(MJOLNIR.into()),
        }
    }

    fn gateway() -> NodeFacts {
        NodeFacts {
            model: "test,wired-board".into(),
            port_map: Some(PortMap::parse(TEST_MAP).unwrap()),
            config: config(GATEWAY_NETWORK),
            wireless_only_peers: Vec::new(),
        }
    }

    fn ap3000() -> NodeFacts {
        NodeFacts {
            model: "cudy,ap3000outdoor-v1".into(),
            port_map: Some(PortMap::parse(AP3000_MAP).unwrap()),
            config: config(AP3000_NETWORK),
            wireless_only_peers: Vec::new(),
        }
    }

    fn ids() -> PlanIds {
        PlanIds {
            transaction_id: "profile-1".into(),
            node_id: "node-a".into(),
        }
    }

    fn wired_core() -> ProfileRequest {
        let mut request = ProfileRequest::new(ProfileId::WiredCore);
        request.ports = vec![
            ("wan".into(), PortRole::Wan),
            ("lan1".into(), PortRole::Client),
            ("lan3".into(), PortRole::NodeLink),
        ];
        request
    }

    fn doc(text: &str) -> UciDoc {
        UciDoc::parse("test", text).unwrap()
    }

    fn wired_plan() -> ProfilePlan {
        build_plan(&wired_core(), &gateway(), &ids()).unwrap()
    }

    /// Re-check a hand-edited desired config against the wired-core decision.
    fn check_tampered(edit: impl FnOnce(&mut NodeConfig)) -> Result<(), ProfileRefusal> {
        let plan = wired_plan();
        let mut desired = plan.desired.clone();
        edit(&mut desired);
        check_profile_safety(&gateway().config, &desired, &plan.decision, false)
    }

    fn edit(slot: &mut Option<String>, from: &str, to: &str) {
        let text = slot.as_mut().unwrap();
        assert!(text.contains(from), "fixture lacks {from:?}:\n{text}");
        *text = text.replacen(from, to, 1);
    }

    // ---- default profile ------------------------------------------------

    #[test]
    fn default_satellite_profile_is_a_noop_on_current_fixtures() {
        for node in [gateway(), ap3000()] {
            let plan = build_plan(&ProfileRequest::new(ProfileId::DEFAULT), &node, &ids()).unwrap();
            assert!(plan.is_noop(), "{}: not a no-op", node.model);
            assert_eq!(plan.plan.proposed_revision, plan.plan.expected_revision);
            assert_eq!(plan.desired, node.config, "{}: bytes changed", node.model);
            assert_eq!(
                plan.plan.required_health,
                vec![ObservationKind::AppliedRevision]
            );
            assert_eq!(plan.decision.radios["radio1"], RadioRole::Mesh);
            assert_eq!(plan.decision.radios["radio0"], RadioRole::ClientAp);
            assert!(plan.decision.node_links().is_empty());
        }
    }

    #[test]
    fn satellite_default_keeps_todays_port_roles() {
        let decision = decide(&ProfileRequest::new(ProfileId::DEFAULT), &gateway()).unwrap();
        assert_eq!(decision.ports["wan"], PortRole::Wan);
        assert_eq!(decision.ports["lan1"], PortRole::Client);
        let ap = decide(&ProfileRequest::new(ProfileId::DEFAULT), &ap3000()).unwrap();
        assert_eq!(ap.ports.len(), 1);
        assert_eq!(ap.ports["eth0"], PortRole::Wan);
    }

    #[test]
    fn default_onto_a_2g_backhaul_node_is_refused_as_a_fleet_change() {
        // Advise N5: BACKHAUL_BAND is island-wide.
        let mut node = gateway();
        let w = node.config.wireless.as_mut().unwrap();
        *w = w
            .replace(
                "option device 'radio1'\n\toption mode 'mesh'",
                "option device 'radio0'\n\toption mode 'mesh'",
            )
            .replace(
                "option device 'radio0'\n\toption mode 'ap'",
                "option device 'radio1'\n\toption mode 'ap'",
            );
        let error = decide(&ProfileRequest::new(ProfileId::DEFAULT), &node).unwrap_err();
        assert!(matches!(
            error,
            ProfileRefusal::IslandBandMismatch {
                node: Band::TwoG,
                island: Band::FiveG,
                ..
            }
        ));
    }

    // ---- one role per port and radio ------------------------------------

    #[test]
    fn shared_port_is_refused_before_mutation() {
        for profile in [ProfileId::DEFAULT, ProfileId::WiredCore] {
            let mut request = ProfileRequest::new(profile);
            request.ports = vec![
                ("wan".into(), PortRole::Wan),
                ("wan".into(), PortRole::Client),
            ];
            let error = build_plan(&request, &gateway(), &ids()).unwrap_err();
            assert_eq!(
                error,
                ProfileRefusal::RoleConflict {
                    kind: "port",
                    name: "wan".into(),
                    roles: "wan + client".into()
                }
            );
        }
    }

    #[test]
    fn shared_radio_is_refused() {
        let mut request = ProfileRequest::new(ProfileId::DEFAULT);
        request.radios = vec![
            ("radio0".into(), RadioRole::ClientAp),
            ("radio0".into(), RadioRole::Disabled),
        ];
        assert!(matches!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::RoleConflict { kind: "radio", .. }
        ));
    }

    #[test]
    fn station_needs_the_uplink_plan() {
        let mut request = ProfileRequest::new(ProfileId::DEFAULT);
        request.radios = vec![("radio0".into(), RadioRole::Station)];
        assert_eq!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::StationNotOwned("radio0".into())
        );
        request.uplink_station_radio = Some("radio0".into());
        assert_eq!(
            decide(&request, &gateway()).unwrap().radios["radio0"],
            RadioRole::Station
        );
    }

    #[test]
    fn a_live_station_the_plan_does_not_own_is_refused() {
        let mut node = gateway();
        let w = node.config.wireless.as_mut().unwrap();
        w.push_str("\nconfig wifi-iface 'uplink'\n\toption device 'radio0'\n\toption mode 'sta'\n\toption network 'wwan'\n");
        assert_eq!(
            decide(&ProfileRequest::new(ProfileId::DEFAULT), &node).unwrap_err(),
            ProfileRefusal::StationNotOwned("radio0".into())
        );
    }

    // ---- port maps ------------------------------------------------------

    #[test]
    fn unqualified_wired_core_is_refused_and_names_the_map() {
        let request = wired_core();
        let error = build_plan(&request, &ap3000(), &ids()).unwrap_err();
        assert!(matches!(error, ProfileRefusal::UnqualifiedPortMap { .. }));
        let text = error.to_string();
        assert!(text.contains("cudy,ap3000outdoor-v1"), "{text}");
        assert!(text.contains("not qualified"), "{text}");

        let mut no_map = gateway();
        no_map.port_map = None;
        let error = decide(&request, &no_map).unwrap_err();
        assert!(error.to_string().contains("no port map ships"), "{error}");

        let mut other = gateway();
        other.model = "test,other-board".into();
        assert!(matches!(
            decide(&request, &other).unwrap_err(),
            ProfileRefusal::UnqualifiedPortMap { .. }
        ));
    }

    #[test]
    fn port_map_parser_validates_and_fixtures_are_unqualified() {
        let ap = PortMap::parse(AP3000_MAP).unwrap();
        assert!(!ap.qualified);
        assert_eq!(
            PortMap::path_for(&ap.model).as_deref(),
            Some("/usr/share/mjolnir/port-maps/cudy,ap3000outdoor-v1.json")
        );
        assert!(
            !PortMap::parse(r#"{"model":"m","ports":["eth0"]}"#)
                .unwrap()
                .qualified
        );
        for bad in [
            r#"{"model":"m","ports":[]}"#,
            r#"{"model":"","ports":["eth0"]}"#,
            r#"{"model":"m","ports":["eth0","eth0"]}"#,
            r#"{"model":"m","ports":["eth 0"]}"#,
            r#"{"model":"m","ports":["eth0"],"guess":"board.json"}"#,
        ] {
            assert!(PortMap::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(PortMap::path_for("../etc"), None);
    }

    #[test]
    fn wired_core_needs_all_three_domains_and_mapped_ports() {
        let mut request = wired_core();
        request
            .ports
            .retain(|(_, role)| *role != PortRole::NodeLink);
        assert_eq!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::MissingDomain {
                profile: ProfileId::WiredCore,
                role: PortRole::NodeLink
            }
        );
        let mut request = wired_core();
        request.ports.push(("eth9".into(), PortRole::Client));
        assert!(matches!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::UnknownPort { .. }
        ));
        let mut request = wired_core();
        request.ports.push(("lan2".into(), PortRole::Wan));
        assert!(matches!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::MultipleWan { .. }
        ));
    }

    // ---- wired-core plan ------------------------------------------------

    #[test]
    fn wired_core_node_link_is_its_own_l3_interface_not_in_br_mesh() {
        let plan = wired_plan();
        let net = doc(plan.desired.network.as_deref().unwrap());
        assert!(!bridge_ports(&net, MESH_BRIDGE).contains(&"lan3".to_string()));
        assert!(!bridge_ports(&net, CLIENT_BRIDGE).contains(&"lan3".to_string()));
        let link = net.named("nl_lan3").expect("node-link interface");
        assert_eq!(link.get("device"), Some("lan3"));
        assert_eq!(link.get("proto"), Some("none"));

        let mj = doc(plan.desired.mjolnir.as_deref().unwrap());
        assert_eq!(node_link_names(&mj), vec!["lan3".to_string()]);
        let profile = mj.named("profile").unwrap();
        assert_eq!(profile.get("id"), Some("wired-core"));
        assert_eq!(profile.get("cable_loss"), Some("isolate"));

        let fw = doc(plan.desired.firewall.as_deref().unwrap());
        let lan_zone = fw
            .of_kind("zone")
            .find(|z| z.get("name") == Some("lan"))
            .unwrap();
        assert!(lan_zone.list("network").any(|n| n == "nl_lan3"));
    }

    #[test]
    fn omitted_mapped_ports_are_disabled_not_left_in_br_lan() {
        let plan = wired_plan();
        assert_eq!(plan.decision.ports["lan2"], PortRole::Disabled);
        assert_eq!(plan.decision.ports["lan4"], PortRole::Disabled);
        let net = doc(plan.desired.network.as_deref().unwrap());
        assert_eq!(bridge_ports(&net, CLIENT_BRIDGE), vec!["lan1".to_string()]);
    }

    #[test]
    fn port_or_mesh_radio_changes_require_management_and_mesh_reachability() {
        let plan = wired_plan();
        assert!(!plan.is_noop());
        for kind in [
            ObservationKind::AppliedRevision,
            ObservationKind::ManagementReachability,
            ObservationKind::MeshReachability,
        ] {
            assert!(plan.plan.required_health.contains(&kind), "{kind:?}");
        }

        // Mesh radio only: reserve-mesh with ports untouched still needs both.
        let current = Docs::parse(&gateway().config).unwrap();
        let mut desired = current.clone();
        desired
            .wireless
            .named_mut("meshbh")
            .unwrap()
            .set_enabled(false);
        let kinds = required_health(&current, &desired);
        assert!(kinds.contains(&ObservationKind::MeshReachability));
        assert!(kinds.contains(&ObservationKind::ManagementReachability));

        // A client-radio-only change does not.
        let mut desired = current.clone();
        desired
            .wireless
            .named_mut("clientap")
            .unwrap()
            .set_enabled(false);
        assert_eq!(
            required_health(&current, &desired),
            vec![ObservationKind::AppliedRevision]
        );
    }

    #[test]
    fn isolate_releases_the_mesh_radio_to_clients() {
        let plan = wired_plan();
        assert_eq!(plan.decision.radios["radio1"], RadioRole::ClientAp);
        let w = doc(plan.desired.wireless.as_deref().unwrap());
        assert_eq!(w.named("meshbh").unwrap().get("disabled"), Some("1"));
        assert_eq!(w.named("clientap2g").unwrap().get("disabled"), Some("0"));
        assert!(w.named("clientap").unwrap().get("disabled").is_none());
    }

    #[test]
    fn cable_loss_isolate_does_not_retake_a_client_radio() {
        let decision = wired_plan().decision;
        assert_eq!(decision.cable_loss, Some(CableLoss::Isolate));
        assert_eq!(decision.mesh_radio_after_node_link_loss(), None);
        assert!(decision.radios.values().all(|r| *r == RadioRole::ClientAp));
    }

    #[test]
    fn isolate_mesh_override_is_refused() {
        let mut request = wired_core();
        request.radios = vec![("radio1".into(), RadioRole::Mesh)];
        assert!(matches!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::RoleNotInProfile { .. }
        ));
    }

    #[test]
    fn isolate_that_strands_a_peer_needs_an_acknowledgement() {
        let mut node = gateway();
        node.wireless_only_peers = vec!["shed".into()];
        let mut request = wired_core();
        assert_eq!(
            decide(&request, &node).unwrap_err(),
            ProfileRefusal::WouldStrandPeers("shed".into())
        );
        request.acknowledge_stranded_peers = true;
        assert!(decide(&request, &node).is_ok());
    }

    #[test]
    fn reserve_mesh_keeps_the_reserved_radio_off_client_ap() {
        let mut request = wired_core();
        request.profile = ProfileId::WiredCoreWithFallback;
        let mut node = gateway();
        node.wireless_only_peers = vec!["shed".into()];
        let decision = decide(&request, &node).unwrap();
        assert_eq!(decision.cable_loss, Some(CableLoss::ReserveMesh));
        assert_eq!(decision.radios["radio1"], RadioRole::Mesh);
        assert_eq!(decision.radios["radio0"], RadioRole::ClientAp);
        assert_eq!(decision.mesh_radio_after_node_link_loss(), Some("radio1"));

        request.radios = vec![("radio1".into(), RadioRole::ClientAp)];
        assert_eq!(
            decide(&request, &node).unwrap_err(),
            ProfileRefusal::ReservedRadioOnClient("radio1".into())
        );
    }

    #[test]
    fn satellites_have_no_node_link() {
        let mut request = ProfileRequest::new(ProfileId::DEFAULT);
        request.ports = vec![("lan3".into(), PortRole::NodeLink)];
        assert!(matches!(
            decide(&request, &gateway()).unwrap_err(),
            ProfileRefusal::RoleNotInProfile { .. }
        ));
    }

    // ---- loop and gateway refusals --------------------------------------

    #[test]
    fn wired_plan_passes_its_own_safety_check() {
        assert_eq!(check_tampered(|_| {}), Ok(()));
    }

    #[test]
    fn refuses_client_port_running_babel() {
        let error = check_tampered(|d| {
            edit(
                &mut d.mjolnir,
                "list ifname 'lan3'",
                "list ifname 'lan3'\n\tlist ifname 'lan1'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::ClientPortRunsBabel("lan1".into()))
        );
        let error = check_tampered(|d| {
            edit(
                &mut d.mjolnir,
                "option backhaul_iface 'br-mesh'",
                "option backhaul_iface 'br-lan'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::ClientPortRunsBabel("br-lan".into()))
        );
    }

    #[test]
    fn refuses_node_link_in_br_mesh() {
        let error = check_tampered(|d| {
            edit(
                &mut d.network,
                "option name 'br-mesh'",
                "option name 'br-mesh'\n\tlist ports 'lan3'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::NodeLinkInMeshBridge("lan3".into()))
        );
    }

    #[test]
    fn refuses_node_link_dhcp() {
        for proto in ["dhcp", "static"] {
            let error = check_tampered(|d| {
                edit(
                    &mut d.network,
                    "option device 'lan3'\n\toption proto 'none'",
                    &format!("option device 'lan3'\n\toption proto '{proto}'"),
                )
            });
            assert_eq!(error, Err(ProfileRefusal::NodeLinkDhcp("lan3".into())));
        }
    }

    #[test]
    fn refuses_wan_bridged_into_br_lan() {
        let error = check_tampered(|d| {
            edit(
                &mut d.network,
                "list ports 'lan1'",
                "list ports 'lan1'\n\tlist ports 'wan'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::WanBridgedIntoClient("wan".into()))
        );
    }

    #[test]
    fn refuses_unassigned_mapped_port_left_in_br_lan() {
        let error = check_tampered(|d| {
            edit(
                &mut d.network,
                "list ports 'lan1'",
                "list ports 'lan1'\n\tlist ports 'lan4'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::UnassignedPortBridged("lan4".into()))
        );
    }

    #[test]
    fn refuses_cross_node_client_bridge() {
        let error = check_tampered(|d| {
            edit(
                &mut d.network,
                "list ports 'lan1'",
                "list ports 'lan1'\n\tlist ports 'br-mesh'",
            )
        });
        assert_eq!(
            error,
            Err(ProfileRefusal::ClientBridgeCrossesNodes("br-mesh".into()))
        );
        // A mesh radio joined to the client network bridges every node's LAN.
        let node = gateway();
        let decision = decide(&ProfileRequest::new(ProfileId::DEFAULT), &node).unwrap();
        let mut desired = node.config.clone();
        edit(
            &mut desired.wireless,
            "option network 'mesh'",
            "option network 'lan'",
        );
        assert_eq!(
            check_profile_safety(&node.config, &desired, &decision, false),
            Err(ProfileRefusal::ClientBridgeCrossesNodes(
                "wireless.meshbh".into()
            ))
        );
    }

    #[test]
    fn refuses_adding_the_recovery_address_to_ap3000() {
        let node = ap3000();
        let decision = decide(&ProfileRequest::new(ProfileId::DEFAULT), &node).unwrap();
        let mut desired = node.config.clone();
        edit(
            &mut desired.network,
            "option ipaddr '10.42.7.1'",
            "option ipaddr '10.42.7.1'\n\tlist ipaddr '192.168.1.1/24'",
        );
        assert_eq!(
            check_profile_safety(&node.config, &desired, &decision, false),
            Err(ProfileRefusal::RecoveryAddressAdded("network.lan".into()))
        );
        // The gateway already uses it for recovery; keeping it is fine.
        let gw = gateway();
        let gw_decision = decide(&ProfileRequest::new(ProfileId::DEFAULT), &gw).unwrap();
        assert_eq!(
            check_profile_safety(&gw.config, &gw.config, &gw_decision, false),
            Ok(())
        );
    }

    #[test]
    fn refuses_silent_deletion_of_house_gateway_export() {
        let node = gateway();
        let decision = decide(&ProfileRequest::new(ProfileId::DEFAULT), &node).unwrap();
        let mut never = node.config.clone();
        edit(
            &mut never.mjolnir,
            "option gateway 'auto'",
            "option gateway 'never'",
        );
        assert_eq!(
            check_profile_safety(&node.config, &never, &decision, false),
            Err(ProfileRefusal::DefaultExportRemoved(
                "mjolnir.meshd.gateway".into()
            ))
        );
        assert_eq!(
            check_profile_safety(&node.config, &never, &decision, true),
            Ok(())
        );

        let mut no_wan = node.config.clone();
        edit(
            &mut no_wan.network,
            "config interface 'wan'\n\toption device 'wan'",
            "config interface 'wan'",
        );
        let mut wan_off = decision.clone();
        wan_off.ports.remove("wan");
        assert_eq!(
            check_profile_safety(&node.config, &no_wan, &wan_off, false),
            Err(ProfileRefusal::DefaultExportRemoved("network.wan".into()))
        );

        // Re-roling the WAN port to client through the builder hits the same rule.
        let mut request = ProfileRequest::new(ProfileId::DEFAULT);
        request.ports = vec![("wan".into(), PortRole::Client)];
        assert!(matches!(
            build_plan(&request, &node, &ids()).unwrap_err(),
            ProfileRefusal::DefaultExportRemoved(_)
        ));
    }

    // ---- UCI round trip --------------------------------------------------

    #[test]
    fn uci_parse_render_round_trips_and_quotes() {
        let original = doc(WIRELESS);
        assert_eq!(doc(&original.render()), original);
        let mut tricky = UciDoc::default();
        tricky.section_mut("x", "y").set("ssid", "it's ⚡");
        let rendered = tricky.render();
        assert!(rendered.contains(r"'it'\''s ⚡'"), "{rendered}");
        assert_eq!(doc(&rendered), tricky);
        assert!(UciDoc::parse("t", "option x 'y'").is_err());
        assert!(UciDoc::parse("t", "config a 'b'\n\toption x 'y").is_err());
        doc(MJOLNIR);
    }

    // ---- transaction on a fake adapter ----------------------------------

    struct FakeRuntime;

    impl Runtime for FakeRuntime {
        fn boot_id(&self) -> Result<String, EngineError> {
            Ok("boot-a".into())
        }
        fn monotonic_ms(&self) -> Result<u64, EngineError> {
            Ok(1_000)
        }
        fn wall_time_ms(&self) -> i64 {
            10_000
        }
    }

    /// Writes desired files into a temp config root and reports scripted
    /// reachability. Implements [`ProvesReachability`]; the OpenWrt adapter
    /// does not.
    struct FakeProbeAdapter {
        root: PathBuf,
        desired: NodeConfig,
        reachability: ObservationStatus,
    }

    impl FakeProbeAdapter {
        fn on_disk(&self) -> NodeConfig {
            let read = |c: UciConfig| fs::read_to_string(self.root.join(c.file_name())).ok();
            NodeConfig {
                wireless: read(UciConfig::Wireless),
                network: read(UciConfig::Network),
                firewall: read(UciConfig::Firewall),
                mjolnir: read(UciConfig::Mjolnir),
            }
        }

        fn observe(kind: ObservationKind, status: ObservationStatus) -> Observation {
            Observation {
                kind,
                status,
                vantage: Vantage::ManagementPeer,
                target: "10.254.1.1".into(),
                interface: "nl_lan3".into(),
                boot_id: "boot-a".into(),
                monotonic_ms: 1_001,
                detail: None,
            }
        }
    }

    impl ApplyAdapter for FakeProbeAdapter {
        fn current_revision(&mut self) -> Result<String, String> {
            Ok(self.on_disk().revision())
        }

        fn apply(&mut self, plan: &Plan) -> Result<(), String> {
            for config in &plan.resources {
                if let Some(text) = self.desired.get(*config) {
                    fs::write(self.root.join(config.file_name()), text)
                        .map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }

        fn check_health(&mut self, plan: &Plan) -> Result<Vec<Observation>, String> {
            let applied = if self.on_disk().revision() == plan.proposed_revision {
                ObservationStatus::Passed
            } else {
                ObservationStatus::Failed
            };
            Ok(vec![
                Self::observe(ObservationKind::AppliedRevision, applied),
                Self::observe(ObservationKind::ManagementReachability, self.reachability),
                Self::observe(ObservationKind::MeshReachability, self.reachability),
            ])
        }

        fn verify_restoration(&mut self, previous: &str) -> Result<Vec<Observation>, String> {
            if self.on_disk().revision() != previous {
                return Err("restored files do not match the previous revision".into());
            }
            Ok(vec![Self::observe(
                ObservationKind::AppliedRevision,
                ObservationStatus::Passed,
            )])
        }
    }

    impl ProvesReachability for FakeProbeAdapter {}

    fn run_on_fake(reachability: ObservationStatus) -> (Receipt, ProfilePlan, NodeConfig) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("config");
        fs::create_dir_all(&root).unwrap();
        let node = gateway();
        for config in UciConfig::ALL {
            fs::write(
                root.join(config.file_name()),
                node.config.get(config).unwrap(),
            )
            .unwrap();
        }
        let plan = wired_plan();
        let engine = Engine::new(TxnPaths::new(temp.path().join("txn"), &root))
            .with_runtime(Arc::new(FakeRuntime));
        let mut adapter = FakeProbeAdapter {
            root,
            desired: plan.desired.clone(),
            reachability,
        };
        let receipt = submit_profile_plan(&engine, &plan, &mut adapter).unwrap();
        (receipt, plan, adapter.on_disk())
    }

    #[test]
    fn profile_apply_commits_with_reachability() {
        let (receipt, plan, on_disk) = run_on_fake(ObservationStatus::Passed);
        assert_eq!(receipt.outcome, Outcome::Committed);
        assert_eq!(receipt.resulting_revision, plan.plan.proposed_revision);
        assert_eq!(on_disk, plan.desired);
    }

    #[test]
    fn unproven_reachability_restores_the_previous_revision() {
        let (receipt, plan, on_disk) = run_on_fake(ObservationStatus::Unknown);
        assert_eq!(receipt.outcome, Outcome::Restored);
        assert_eq!(receipt.resulting_revision, plan.plan.expected_revision);
        assert_eq!(on_disk, gateway().config);
    }

    #[test]
    fn decider_does_not_reference_the_openwrt_adapter() {
        // The trait bound on `submit_profile_plan` is the structural guard;
        // this keeps the code path from growing a direct call.
        let needle = concat!("OpenWrt", "Adapter");
        let source = include_str!("node_profile.rs");
        for line in source.lines() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            assert!(!code.contains(needle), "decider references it: {line}");
        }
    }
}
