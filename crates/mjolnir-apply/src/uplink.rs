//! Wi-Fi station uplink plans (`add-wifi-internet-uplink`, bead
//! `mjolnir-mesh-bf7.2`).
//!
//! A pure plan builder. It binds credentials to one candidate (node, radio,
//! SSID and, when the scan saw one, BSSID), checks every refusal rule against
//! the committed UCI text it is handed, and returns one revision-checked
//! [`Plan`]. It never scans, associates, reads the node, or writes a file.
//!
//! The passphrase is never placed in the four revisioned UCI files, in the
//! [`Plan`], or in anything derived from them. The plan names a root-only
//! secret file ([`SECRET_DIR`]`/<candidate-id>`) and a monotonic secret
//! generation; the generation sits in `mjolnir.uplink`, so a passphrase-only
//! change still moves the revision without the revision depending on the
//! bytes (advise r2 N2, N3).
//!
//! The mesh default-route export is gated: the station interface comes up with
//! `defaultroute '0'`, and [`default_export`] announces `0.0.0.0/0` only after
//! upstream-bound DNS and HTTPS evidence passes. A captive lease exports
//! nothing.
//!
//! The OpenWrt adapter cannot observe the upstream health kinds or reload
//! inside apply and restore, so [`submit_uplink_plan`] accepts only an adapter
//! that implements [`ProvesUpstream`]. The OpenWrt adapter does not.

use crate::engine::{ApplyAdapter, Engine, EngineError};
use crate::model::{
    DEFAULT_TIMEOUT_SECS, Observation, ObservationKind, Plan, Receipt, SCHEMA_VERSION, UciConfig,
};
use crate::node_profile::NodeConfig;
use std::collections::BTreeSet;
use std::fmt;
use std::net::Ipv4Addr;
use thiserror::Error;

/// Root-only directory holding one passphrase file per candidate.
pub const SECRET_DIR: &str = "/etc/mjolnir/secrets/uplink";
/// Mode of a secret file. Its parent directory is root-only too.
pub const SECRET_MODE: u32 = 0o600;
/// The house client SSID on this fleet. The configured one is refused as well.
pub const HOUSE_CLIENT_SSID: &str = "Lightning Mesh";

/// Health a station plan must prove before commit.
pub const REQUIRED_HEALTH: [ObservationKind; 6] = [
    ObservationKind::UpstreamAssociation,
    ObservationKind::UpstreamAddress,
    ObservationKind::UpstreamRoute,
    ObservationKind::UpstreamBoundDnsHttps,
    ObservationKind::MeshReachability,
    ObservationKind::ManagementReachability,
];

/// Health restoration proves. Never the upstream kinds: a wrong passphrase must
/// return to the previous revision, not stick at recovery-required for lack of
/// upstream evidence.
pub const RESTORATION_HEALTH: [ObservationKind; 2] = [
    ObservationKind::ManagementReachability,
    ObservationKind::AppliedRevision,
];

/// Evidence that must pass before the mesh default route is announced.
pub const EXPORT_GATE: [ObservationKind; 4] = [
    ObservationKind::UpstreamAssociation,
    ObservationKind::UpstreamAddress,
    ObservationKind::UpstreamRoute,
    ObservationKind::UpstreamBoundDnsHttps,
];

/// UCI section names this builder owns.
const STATION_IFACE: &str = "mjolnir_uplink";
const STATION_NETWORK: &str = "wwan";
const UPLINK_ZONE: &str = "mjolnir_uplink";
const UPLINK_FORWARDING: &str = "mjolnir_uplink_fwd";
const UPLINK_RECORD: &str = "uplink";

/// One upstream network as bound to one node and radio. A different BSSID for
/// the same SSID is a different candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub node_id: String,
    pub radio: String,
    pub ssid: String,
    /// Recorded when the scan saw one. Lowercase `aa:bb:cc:dd:ee:ff`.
    pub bssid: Option<String>,
}

impl Candidate {
    /// Opaque, path-safe ID. Derived from the binding only, never the secret.
    pub fn id(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"mjolnir-uplink-candidate\0");
        for field in [
            self.node_id.as_str(),
            self.radio.as_str(),
            self.ssid.as_str(),
            self.bssid.as_deref().unwrap_or(""),
        ] {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field.as_bytes());
        }
        let hex = hasher.finalize().to_hex();
        format!("uplink-{}", &hex[..24])
    }

    pub fn secret_path(&self) -> String {
        format!("{SECRET_DIR}/{}", self.id())
    }
}

/// A WPA passphrase. It has no `Serialize`, and `Debug` is redacted, so it
/// cannot ride into a plan, receipt, journal, or log by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct Passphrase(String);

impl Passphrase {
    /// 8..=63 printable ASCII, or 64 hex digits (a raw PSK).
    pub fn new(value: impl Into<String>) -> Result<Self, UplinkRefusal> {
        let value = value.into();
        let printable =
            (8..=63).contains(&value.len()) && value.bytes().all(|b| (0x20..=0x7e).contains(&b));
        let raw_psk = value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        if printable || raw_psk {
            Ok(Self(value))
        } else {
            Err(UplinkRefusal::InvalidPassphrase)
        }
    }

    /// The bytes for the root-only secret file. Only an adapter writing that
    /// file should call this.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Passphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Passphrase(<redacted>)")
    }
}

/// What the owner authorized: exactly one candidate. `open` is set only when
/// the owner selected an open network on purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    pub candidate: Candidate,
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct UplinkRequest {
    pub authorization: Authorization,
    /// The candidate the plan would associate to.
    pub target: Candidate,
    /// The scan showed the target network without encryption.
    pub target_open: bool,
    /// Required unless the authorization selected an open network.
    pub passphrase: Option<Passphrase>,
}

/// SSIDs and BSSIDs every mesh node advertises (house client, guest, captive,
/// dongle AP, per-node SSID), from the radio inventory / address book.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeshInventory {
    pub ssids: BTreeSet<String>,
    pub bssids: BTreeSet<String>,
}

/// What the builder knows about the node. All of it is handed in.
#[derive(Debug, Clone)]
pub struct UplinkFacts {
    pub node_id: String,
    /// The committed four UCI files.
    pub config: NodeConfig,
    /// `None` when the inventory is not current. The builder then refuses
    /// rather than guess which BSSIDs are ours (advise r2 N4).
    pub mesh_inventory: Option<MeshInventory>,
    /// The configured house client SSID. [`HOUSE_CLIENT_SSID`] is refused too.
    pub client_ssid: Option<String>,
    /// Nodes announcing `0.0.0.0/0` into the mesh now (e.g. AP3000).
    pub mesh_gateway_exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UplinkRefusal {
    #[error("cannot parse {file}: {detail}")]
    Parse { file: &'static str, detail: String },
    #[error("invalid {field}: {value}")]
    InvalidField { field: &'static str, value: String },
    #[error("passphrase must be 8..=63 printable ASCII or 64 hex digits")]
    InvalidPassphrase,
    #[error("credentials are authorized for node {authorized}, not {target}")]
    WrongNode { authorized: String, target: String },
    #[error("credentials are authorized for candidate {authorized}, not {target}")]
    WrongCandidate { authorized: String, target: String },
    #[error("open network {0} was not selected")]
    UnselectedOpen(String),
    #[error("a protected network needs a passphrase")]
    MissingPassphrase,
    #[error("an open network takes no passphrase")]
    PassphraseForOpen,
    #[error("the mesh BSSID inventory is not current")]
    MeshInventoryNotCurrent,
    #[error("BSSID {0} is advertised by a mesh node")]
    MeshBssid(String),
    #[error("SSID {0} belongs to this mesh")]
    MeshSsid(String),
    #[error("radio {0} does not exist on this node")]
    UnknownRadio(String),
    #[error("radio {0} is disabled")]
    RadioDisabled(String),
    #[error("radio {radio} carries mesh-point wifi-iface {iface}")]
    MeshRadio { radio: String, iface: String },
    #[error("station wifi-iface {0} already exists; the uplink names exactly one network")]
    ExistingStation(String),
    #[error("{0} already exists and is not owned by the uplink")]
    SectionTaken(String),
    #[error("a default route is already exported by {0}; a second one is not installed beside it")]
    SecondDefaultRoute(String),
    #[error("mjolnir.meshd.gateway is never; the uplink could not be exported")]
    ExportDisabled,
}

/// The root-only secret file the adapter writes before apply and snapshots in
/// a journal sidecar. It holds no bytes; the caller hands the [`Passphrase`]
/// to the adapter separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretFile {
    pub path: String,
    pub mode: u32,
    /// Monotonic per node. Recorded in `mjolnir.uplink.secret_generation`, so a
    /// restored config cannot point at a newer secret under a reused name.
    pub generation: u64,
}

/// Known side effects that are not failures (advise r2 N5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UplinkEffect {
    /// The upstream AP sets the radio channel; these APs follow it and their
    /// clients re-associate.
    CoLocatedApFollowsUpstream { radio: String, ifaces: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UplinkPlan {
    pub plan: Plan,
    pub candidate_id: String,
    pub secret: Option<SecretFile>,
    /// The four files after apply. The passphrase is in none of them.
    pub desired: NodeConfig,
    /// What restoration proves. See [`RESTORATION_HEALTH`].
    pub restoration_health: Vec<ObservationKind>,
    /// What must pass before the mesh default is announced. See [`EXPORT_GATE`].
    pub export_gate: Vec<ObservationKind>,
    pub effects: Vec<UplinkEffect>,
}

/// An adapter that observes the upstream health kinds from a real vantage,
/// reloads inside apply and restore, snapshots and restores the secret file,
/// and verifies restoration with [`check_restoration`] only. The OpenWrt
/// adapter does not implement this, so an uplink plan cannot reach it.
pub trait ProvesUpstream: ApplyAdapter {}

/// Submit an uplink plan as one network-change transaction.
pub fn submit_uplink_plan<A: ProvesUpstream>(
    engine: &Engine,
    uplink: &UplinkPlan,
    adapter: &mut A,
) -> Result<Receipt, EngineError> {
    engine.apply(&uplink.plan, adapter)
}

/// Whether the mesh default route may be announced now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultExport {
    Withheld,
    Announce,
}

/// The export gate. A lease, a route, even association alone, announce
/// nothing; only qualifying upstream-bound DNS and HTTPS evidence does.
pub fn default_export(observations: &[Observation]) -> DefaultExport {
    let passed = |kind: ObservationKind| {
        observations
            .iter()
            .any(|o| o.kind == kind && o.is_qualifying_pass())
    };
    if EXPORT_GATE.into_iter().all(passed) {
        DefaultExport::Announce
    } else {
        DefaultExport::Withheld
    }
}

/// The restoration rule every [`ProvesUpstream`] adapter applies: management
/// reachability and the applied revision, nothing upstream.
pub fn check_restoration(observations: &[Observation]) -> Result<(), ObservationKind> {
    match RESTORATION_HEALTH.into_iter().find(|kind| {
        !observations
            .iter()
            .any(|o| o.kind == *kind && o.is_qualifying_pass())
    }) {
        Some(missing) => Err(missing),
        None => Ok(()),
    }
}

/// A lease that overlaps a mesh prefix or the recovery address is a restore
/// trigger, not a NAT exception.
pub fn lease_conflicts(address: Ipv4Addr, prefix_len: u8) -> bool {
    let reserved = [
        (Ipv4Addr::new(10, 42, 0, 0), 16),
        (Ipv4Addr::new(10, 254, 0, 0), 16),
        (Ipv4Addr::new(192, 168, 1, 1), 32),
    ];
    let mask = |len: u8| match len {
        0 => 0,
        len => u32::MAX << (32 - u32::from(len.min(32))),
    };
    reserved.into_iter().any(|(net, len)| {
        let shared = mask(prefix_len.min(len));
        u32::from(address) & shared == u32::from(net) & shared
    })
}

/// Check every refusal and build one revision-checked station plan. Pure.
pub fn build_uplink_plan(
    request: &UplinkRequest,
    facts: &UplinkFacts,
    transaction_id: &str,
) -> Result<UplinkPlan, UplinkRefusal> {
    let target = normalize(&request.target)?;
    let authorized = normalize(&request.authorization.candidate)?;
    let open = request.authorization.open;

    // Credentials are not tried on a node, radio, SSID, or BSSID they were not
    // authorized for.
    if authorized.node_id != facts.node_id || target.node_id != facts.node_id {
        return Err(UplinkRefusal::WrongNode {
            authorized: authorized.node_id.clone(),
            target: if target.node_id != authorized.node_id {
                target.node_id.clone()
            } else {
                facts.node_id.clone()
            },
        });
    }
    if target != authorized {
        return Err(UplinkRefusal::WrongCandidate {
            authorized: authorized.id(),
            target: target.id(),
        });
    }
    if request.target_open && !open {
        return Err(UplinkRefusal::UnselectedOpen(target.ssid.clone()));
    }
    match (&request.passphrase, open) {
        (None, false) => return Err(UplinkRefusal::MissingPassphrase),
        (Some(_), true) => return Err(UplinkRefusal::PassphraseForOpen),
        _ => {}
    }

    let wireless = UciText::parse("wireless", facts.config.get(UciConfig::Wireless))?;
    let network = UciText::parse("network", facts.config.get(UciConfig::Network))?;
    let firewall = UciText::parse("firewall", facts.config.get(UciConfig::Firewall))?;
    let mjolnir = UciText::parse("mjolnir", facts.config.get(UciConfig::Mjolnir))?;
    let record = mjolnir.named(UPLINK_RECORD);

    // Our own networks are refused by identity, not by SSID string alone.
    let inventory = facts
        .mesh_inventory
        .as_ref()
        .ok_or(UplinkRefusal::MeshInventoryNotCurrent)?;
    if let Some(bssid) = &target.bssid
        && inventory
            .bssids
            .iter()
            .any(|b| b.eq_ignore_ascii_case(bssid))
    {
        return Err(UplinkRefusal::MeshBssid(bssid.clone()));
    }
    let mut mesh_ssids: BTreeSet<&str> = inventory.ssids.iter().map(String::as_str).collect();
    mesh_ssids.insert(HOUSE_CLIENT_SSID);
    mesh_ssids.extend(facts.client_ssid.as_deref());
    mesh_ssids.extend(mjolnir.named("radio").and_then(|s| s.get("ssid")));
    for iface in wireless.of_kind("wifi-iface") {
        if iface.get("mode") != Some("sta") {
            mesh_ssids.extend(iface.get("ssid"));
            mesh_ssids.extend(iface.get("mesh_id"));
        }
    }
    if mesh_ssids.contains(target.ssid.as_str()) {
        return Err(UplinkRefusal::MeshSsid(target.ssid.clone()));
    }

    // The radio exists, is on, and carries no mesh-point interface, whether or
    // not a profile exists.
    let radio = wireless
        .of_kind("wifi-device")
        .find(|dev| dev.name.as_deref() == Some(target.radio.as_str()))
        .ok_or_else(|| UplinkRefusal::UnknownRadio(target.radio.clone()))?;
    if radio.get("disabled") == Some("1") {
        return Err(UplinkRefusal::RadioDisabled(target.radio.clone()));
    }
    let mut co_located_aps = Vec::new();
    for iface in wireless.of_kind("wifi-iface") {
        let label = iface.label();
        if iface.get("disabled") == Some("1") {
            continue;
        }
        let ours = iface.name.as_deref() == Some(STATION_IFACE);
        if iface.get("mode") == Some("sta") && !ours {
            return Err(UplinkRefusal::ExistingStation(label));
        }
        if iface.get("device") != Some(target.radio.as_str()) {
            continue;
        }
        match iface.get("mode") {
            Some("mesh") => {
                return Err(UplinkRefusal::MeshRadio {
                    radio: target.radio.clone(),
                    iface: label,
                });
            }
            Some("ap") => co_located_aps.push(label),
            _ => {}
        }
    }

    // Section names we write are ours, or free.
    if record.is_none() {
        for (doc, file, name) in [
            (&wireless, "wireless", STATION_IFACE),
            (&network, "network", STATION_NETWORK),
            (&firewall, "firewall", UPLINK_ZONE),
            (&firewall, "firewall", UPLINK_FORWARDING),
        ] {
            if doc.named(name).is_some() {
                return Err(UplinkRefusal::SectionTaken(format!("{file}.{name}")));
            }
        }
    }

    // No second default route beside an existing gateway export. Replacing the
    // house gateway's export cannot be part of this node's plan, so refuse.
    let gateway = mjolnir
        .named("meshd")
        .and_then(|s| s.get("gateway"))
        .unwrap_or("auto");
    if gateway == "never" {
        return Err(UplinkRefusal::ExportDisabled);
    }
    let live_wan = network.named("wan").is_some_and(|wan| {
        wan.get("device").is_some()
            && wan.get("disabled") != Some("1")
            && wan.get("auto") != Some("0")
    });
    if live_wan {
        return Err(UplinkRefusal::SecondDefaultRoute(format!(
            "{} network.wan",
            facts.node_id
        )));
    }
    if let Some(exporter) = facts
        .mesh_gateway_exports
        .iter()
        .find(|node| **node != facts.node_id || record.is_none())
    {
        return Err(UplinkRefusal::SecondDefaultRoute(exporter.clone()));
    }

    // Render. The passphrase reaches none of these files.
    let candidate_id = target.id();
    let previous_generation = record
        .and_then(|s| s.get("secret_generation"))
        .and_then(|g| g.parse::<u64>().ok())
        .unwrap_or(0);
    let secret = (!open).then(|| SecretFile {
        path: target.secret_path(),
        mode: SECRET_MODE,
        generation: previous_generation + 1,
    });
    let desired = NodeConfig {
        wireless: Some(replace_sections(
            facts.config.get(UciConfig::Wireless),
            &wireless,
            &[STATION_IFACE],
            &station_iface(&target, open),
        )),
        network: Some(replace_sections(
            facts.config.get(UciConfig::Network),
            &network,
            &[STATION_NETWORK],
            &station_network(),
        )),
        firewall: Some(replace_sections(
            facts.config.get(UciConfig::Firewall),
            &firewall,
            &[UPLINK_ZONE, UPLINK_FORWARDING],
            &uplink_firewall(),
        )),
        mjolnir: Some(replace_sections(
            facts.config.get(UciConfig::Mjolnir),
            &mjolnir,
            &[UPLINK_RECORD],
            &uplink_record(&target, &candidate_id, secret.as_ref()),
        )),
    };

    let plan = Plan {
        schema_version: SCHEMA_VERSION,
        transaction_id: transaction_id.to_owned(),
        node_id: facts.node_id.clone(),
        expected_revision: facts.config.revision(),
        proposed_revision: desired.revision(),
        resources: UciConfig::ALL.to_vec(),
        timeout_secs: DEFAULT_TIMEOUT_SECS,
        required_health: REQUIRED_HEALTH.to_vec(),
    };
    let effects = if co_located_aps.is_empty() {
        Vec::new()
    } else {
        vec![UplinkEffect::CoLocatedApFollowsUpstream {
            radio: target.radio.clone(),
            ifaces: co_located_aps,
        }]
    };
    Ok(UplinkPlan {
        plan,
        candidate_id,
        secret,
        desired,
        restoration_health: RESTORATION_HEALTH.to_vec(),
        export_gate: EXPORT_GATE.to_vec(),
        effects,
    })
}

fn normalize(candidate: &Candidate) -> Result<Candidate, UplinkRefusal> {
    let invalid = |field: &'static str, value: &str| UplinkRefusal::InvalidField {
        field,
        value: value.to_owned(),
    };
    let token = |value: &str| {
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    if !token(&candidate.node_id) {
        return Err(invalid("node_id", &candidate.node_id));
    }
    if !token(&candidate.radio) {
        return Err(invalid("radio", &candidate.radio));
    }
    if candidate.ssid.is_empty()
        || candidate.ssid.len() > 32
        || candidate.ssid.chars().any(char::is_control)
    {
        return Err(invalid("ssid", &candidate.ssid));
    }
    let bssid = match &candidate.bssid {
        None => None,
        Some(bssid) => {
            let lower = bssid.to_ascii_lowercase();
            let octets: Vec<&str> = lower.split(':').collect();
            if octets.len() != 6
                || !octets
                    .iter()
                    .all(|o| o.len() == 2 && o.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err(invalid("bssid", bssid));
            }
            Some(lower)
        }
    };
    Ok(Candidate {
        node_id: candidate.node_id.clone(),
        radio: candidate.radio.clone(),
        ssid: candidate.ssid.clone(),
        bssid,
    })
}

/// One station, one network, no fallback list, no `key`. The station is in
/// `wwan`, never in the client bridge.
fn station_iface(target: &Candidate, open: bool) -> String {
    let mut out = format!(
        "\nconfig wifi-iface {}\n\toption device {}\n\toption mode 'sta'\n\toption network {}\n\toption ssid {}\n",
        quote(STATION_IFACE),
        quote(&target.radio),
        quote(STATION_NETWORK),
        quote(&target.ssid),
    );
    if let Some(bssid) = &target.bssid {
        out.push_str(&format!("\toption bssid {}\n", quote(bssid)));
    }
    out.push_str(if open {
        "\toption encryption 'none'\n"
    } else {
        "\toption encryption 'psk2'\n"
    });
    out
}

/// DHCP on the station with no default route: [`default_export`] owns that.
fn station_network() -> String {
    format!(
        "\nconfig interface {}\n\toption proto 'dhcp'\n\toption defaultroute '0'\n",
        quote(STATION_NETWORK)
    )
}

fn uplink_firewall() -> String {
    format!(
        "\nconfig zone {zone}\n\toption name 'uplink'\n\tlist network {net}\n\toption input 'REJECT'\n\toption output 'ACCEPT'\n\toption forward 'REJECT'\n\toption masq '1'\n\toption mtu_fix '1'\n\
         \nconfig forwarding {fwd}\n\toption src 'lan'\n\toption dest 'uplink'\n",
        zone = quote(UPLINK_ZONE),
        net = quote(STATION_NETWORK),
        fwd = quote(UPLINK_FORWARDING),
    )
}

fn uplink_record(target: &Candidate, candidate_id: &str, secret: Option<&SecretFile>) -> String {
    let mut out = format!(
        "\nconfig uplink {}\n\toption candidate {}\n\toption radio {}\n\toption ssid {}\n",
        quote(UPLINK_RECORD),
        quote(candidate_id),
        quote(&target.radio),
        quote(&target.ssid),
    );
    if let Some(bssid) = &target.bssid {
        out.push_str(&format!("\toption bssid {}\n", quote(bssid)));
    }
    if let Some(secret) = secret {
        out.push_str(&format!(
            "\toption secret_file {}\n\toption secret_generation '{}'\n",
            quote(&secret.path),
            secret.generation
        ));
    }
    out.push_str("\toption export_gate 'upstream-bound-dns-https'\n");
    out
}

/// Drop the named sections and append `block`. Every other byte is kept, so an
/// untouched section reads the same after the plan.
fn replace_sections(text: Option<&str>, doc: &UciText, names: &[&str], block: &str) -> String {
    let text = text.unwrap_or("");
    let drop: Vec<(usize, usize)> = doc
        .sections
        .iter()
        .filter(|s| s.name.as_deref().is_some_and(|n| names.contains(&n)))
        .map(|s| (s.start, s.end))
        .collect();
    let mut out = String::new();
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if !drop
            .iter()
            .any(|(start, end)| (*start..*end).contains(&index))
        {
            out.push_str(line);
        }
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(block);
    out
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Read-only UCI view with line spans, enough to check refusals and replace
/// the sections the uplink owns.
#[derive(Debug, Default)]
struct UciText {
    sections: Vec<UciSection>,
}

#[derive(Debug)]
struct UciSection {
    kind: String,
    name: Option<String>,
    options: Vec<(String, String)>,
    /// Line span `[start, end)` in the source text.
    start: usize,
    end: usize,
}

impl UciText {
    fn parse(file: &'static str, text: Option<&str>) -> Result<Self, UplinkRefusal> {
        let mut sections: Vec<UciSection> = Vec::new();
        let mut lines = 0;
        for (index, line) in text.unwrap_or("").split_inclusive('\n').enumerate() {
            lines = index + 1;
            let error = |detail: String| UplinkRefusal::Parse {
                file,
                detail: format!("line {}: {detail}", index + 1),
            };
            let words = split_words(line).map_err(error)?;
            let Some((head, rest)) = words.split_first() else {
                continue;
            };
            match (head.as_str(), rest) {
                ("package", [_]) => {}
                ("config", [kind, name @ ..]) if name.len() <= 1 => {
                    if let Some(last) = sections.last_mut() {
                        last.end = index;
                    }
                    sections.push(UciSection {
                        kind: kind.clone(),
                        name: name.first().cloned(),
                        options: Vec::new(),
                        start: index,
                        end: index + 1,
                    });
                }
                ("option" | "list", [key, value]) => {
                    let section = sections
                        .last_mut()
                        .ok_or_else(|| error(format!("{head} before any config section")))?;
                    if head == "option" {
                        section.options.push((key.clone(), value.clone()));
                    }
                }
                _ => return Err(error(format!("unexpected `{}`", line.trim()))),
            }
        }
        if let Some(last) = sections.last_mut() {
            last.end = lines;
        }
        Ok(Self { sections })
    }

    fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a UciSection> {
        self.sections.iter().filter(move |s| s.kind == kind)
    }

    fn named(&self, name: &str) -> Option<&UciSection> {
        self.sections
            .iter()
            .find(|s| s.name.as_deref() == Some(name))
    }
}

impl UciSection {
    fn get(&self, key: &str) -> Option<&str> {
        self.options
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn label(&self) -> String {
        format!(
            "{}.{}",
            self.kind,
            self.name.as_deref().unwrap_or("@anonymous")
        )
    }
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
    use crate::model::{ObservationStatus, Outcome, Vantage};
    use crate::storage::TxnPaths;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// The shipped `/etc/config/mjolnir` (comments and all).
    const MJOLNIR: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/openwrt/files/etc/config/mjolnir"
    ));

    /// `setup-wireless.sh` output: 2.4 GHz client AP, 5 GHz mesh.
    const WIRELESS: &str = "
config wifi-device 'radio0'
	option type 'mac80211'
	option band '2g'
	option channel '6'
	option disabled '0'

config wifi-device 'radio1'
	option type 'mac80211'
	option band '5g'
	option channel '36'
	option disabled '0'

config wifi-iface 'meshbh'
	option device 'radio1'
	option mode 'mesh'
	option mesh_id 'mjolnir-mesh'
	option network 'mesh'
	option encryption 'none'

config wifi-iface 'clientap'
	option device 'radio0'
	option mode 'ap'
	option ssid '⚡'
	option network 'lan'
	option encryption 'none'
";

    /// A satellite: routed client /24, no WAN.
    const NETWORK: &str = "
config interface 'loopback'
	option device 'lo'
	option proto 'static'
	option ipaddr '127.0.0.1'

config device
	option name 'br-lan'
	option type 'bridge'
	list ports 'lan1'

config interface 'lan'
	option device 'br-lan'
	option proto 'static'
	option ipaddr '10.42.7.1'
	option netmask '255.255.255.0'
";

    const GATEWAY_WAN: &str = "
config interface 'wan'
	option device 'wan'
	option proto 'dhcp'
";

    const FIREWALL: &str = "
config zone
	option name 'lan'
	list network 'lan'
	option input 'ACCEPT'
	option output 'ACCEPT'
	option forward 'ACCEPT'
";

    const NODE: &str = "node-a";
    const UPSTREAM_SSID: &str = "Neighbour 5G";
    const UPSTREAM_BSSID: &str = "02:11:22:33:44:55";
    const MESH_BSSID: &str = "0a:00:00:00:07:01";
    const PASSPHRASE: &str = "correct horse battery staple";

    fn config() -> NodeConfig {
        NodeConfig {
            wireless: Some(WIRELESS.into()),
            network: Some(NETWORK.into()),
            firewall: Some(FIREWALL.into()),
            mjolnir: Some(MJOLNIR.into()),
        }
    }

    fn facts() -> UplinkFacts {
        UplinkFacts {
            node_id: NODE.into(),
            config: config(),
            mesh_inventory: Some(MeshInventory {
                ssids: ["⚡".to_owned(), "Lightning Guest".to_owned()].into(),
                bssids: [MESH_BSSID.to_owned()].into(),
            }),
            client_ssid: Some(HOUSE_CLIENT_SSID.into()),
            mesh_gateway_exports: Vec::new(),
        }
    }

    fn candidate() -> Candidate {
        Candidate {
            node_id: NODE.into(),
            radio: "radio0".into(),
            ssid: UPSTREAM_SSID.into(),
            bssid: Some(UPSTREAM_BSSID.into()),
        }
    }

    fn request_with(passphrase: &str) -> UplinkRequest {
        UplinkRequest {
            authorization: Authorization {
                candidate: candidate(),
                open: false,
            },
            target: candidate(),
            target_open: false,
            passphrase: Some(Passphrase::new(passphrase).unwrap()),
        }
    }

    fn request() -> UplinkRequest {
        request_with(PASSPHRASE)
    }

    fn build(request: &UplinkRequest, facts: &UplinkFacts) -> Result<UplinkPlan, UplinkRefusal> {
        build_uplink_plan(request, facts, "txn-uplink-1")
    }

    /// Everything controller-visible that is derived from the plan.
    fn visible(plan: &UplinkPlan) -> String {
        let mut out = serde_json::to_string(&plan.plan).unwrap();
        for config in UciConfig::ALL {
            out.push_str(plan.desired.get(config).unwrap_or(""));
        }
        out.push_str(&format!("{:?}", plan));
        out
    }

    fn assert_no_secret(text: &str, passphrase: &str) {
        assert!(!text.contains(passphrase), "passphrase leaked");
        let unkeyed = blake3::hash(passphrase.as_bytes()).to_hex().to_string();
        assert!(!text.contains(&unkeyed), "unkeyed passphrase hash leaked");
    }

    #[test]
    fn uplink_plan_binds_one_candidate_with_station_health() {
        let built = build(&request(), &facts()).unwrap();
        assert_eq!(built.plan.required_health, REQUIRED_HEALTH.to_vec());
        assert_eq!(
            built.restoration_health,
            vec![
                ObservationKind::ManagementReachability,
                ObservationKind::AppliedRevision
            ]
        );
        assert_eq!(built.plan.node_id, NODE);
        assert_eq!(built.plan.expected_revision, config().revision());
        assert_ne!(built.plan.proposed_revision, built.plan.expected_revision);
        let secret = built.secret.as_ref().unwrap();
        assert_eq!(
            secret.path,
            format!("/etc/mjolnir/secrets/uplink/{}", candidate().id())
        );
        assert_eq!(secret.mode, 0o600);
        assert_eq!(secret.generation, 1);

        let wireless = built.desired.wireless.as_deref().unwrap();
        assert!(
            wireless.starts_with(WIRELESS),
            "untouched sections keep their bytes"
        );
        assert!(wireless.contains("option mode 'sta'"));
        assert!(wireless.contains("option bssid '02:11:22:33:44:55'"));
        assert!(!wireless.contains("option key"));
        assert_eq!(wireless.matches("option mode 'sta'").count(), 1);
        let network = built.desired.network.as_deref().unwrap();
        assert!(network.contains("config interface 'wwan'"));
        assert!(network.contains("option defaultroute '0'"));
        assert!(
            !network.contains("list ports 'wwan'"),
            "station is not in br-lan"
        );
        assert_eq!(
            built.effects,
            vec![UplinkEffect::CoLocatedApFollowsUpstream {
                radio: "radio0".into(),
                ifaces: vec!["wifi-iface.clientap".into()],
            }]
        );
    }

    #[test]
    fn uplink_refuses_wrong_node_and_wrong_candidate() {
        let mut other_node = request();
        other_node.target.node_id = "node-b".into();
        assert!(matches!(
            build(&other_node, &facts()),
            Err(UplinkRefusal::WrongNode { .. })
        ));

        let mut other_facts = facts();
        other_facts.node_id = "node-b".into();
        assert!(matches!(
            build(&request(), &other_facts),
            Err(UplinkRefusal::WrongNode { .. })
        ));

        let mut other_radio = request();
        other_radio.target.radio = "radio1".into();
        assert!(matches!(
            build(&other_radio, &facts()),
            Err(UplinkRefusal::WrongCandidate { .. })
        ));

        let mut other_ssid = request();
        other_ssid.target.ssid = "Neighbour 2G".into();
        assert!(matches!(
            build(&other_ssid, &facts()),
            Err(UplinkRefusal::WrongCandidate { .. })
        ));
    }

    #[test]
    fn uplink_different_bssid_is_a_different_candidate() {
        let mut moved = request();
        moved.target.bssid = Some("02:11:22:33:44:66".into());
        assert_ne!(moved.target.id(), moved.authorization.candidate.id());
        assert!(matches!(
            build(&moved, &facts()),
            Err(UplinkRefusal::WrongCandidate { .. })
        ));

        let mut unrecorded = request();
        unrecorded.authorization.candidate.bssid = None;
        assert!(matches!(
            build(&unrecorded, &facts()),
            Err(UplinkRefusal::WrongCandidate { .. })
        ));

        let mut case = request();
        case.target.bssid = Some(UPSTREAM_BSSID.to_ascii_uppercase());
        assert!(build(&case, &facts()).is_ok(), "BSSID case is not identity");
    }

    #[test]
    fn uplink_refuses_mesh_bssid_and_house_ssids() {
        let mut ours = request();
        for c in [&mut ours.authorization.candidate, &mut ours.target] {
            c.bssid = Some(MESH_BSSID.to_ascii_uppercase());
        }
        assert_eq!(
            build(&ours, &facts()),
            Err(UplinkRefusal::MeshBssid(MESH_BSSID.into()))
        );

        for ssid in [HOUSE_CLIENT_SSID, "⚡", "Lightning Guest", "mjolnir-mesh"] {
            let mut house = request();
            for c in [&mut house.authorization.candidate, &mut house.target] {
                c.ssid = ssid.into();
            }
            assert_eq!(
                build(&house, &facts()),
                Err(UplinkRefusal::MeshSsid(ssid.into())),
                "{ssid}"
            );
        }

        let mut stale = facts();
        stale.mesh_inventory = None;
        assert_eq!(
            build(&request(), &stale),
            Err(UplinkRefusal::MeshInventoryNotCurrent)
        );
    }

    #[test]
    fn uplink_refuses_mesh_radio() {
        let mut mesh = request();
        for c in [&mut mesh.authorization.candidate, &mut mesh.target] {
            c.radio = "radio1".into();
        }
        assert_eq!(
            build(&mesh, &facts()),
            Err(UplinkRefusal::MeshRadio {
                radio: "radio1".into(),
                iface: "wifi-iface.meshbh".into(),
            })
        );

        let mut missing = request();
        for c in [&mut missing.authorization.candidate, &mut missing.target] {
            c.radio = "radio9".into();
        }
        assert_eq!(
            build(&missing, &facts()),
            Err(UplinkRefusal::UnknownRadio("radio9".into()))
        );
    }

    #[test]
    fn uplink_refuses_second_default_route_beside_gateway_export() {
        let mut beside_ap3000 = facts();
        beside_ap3000.mesh_gateway_exports = vec!["ap3000".into()];
        assert_eq!(
            build(&request(), &beside_ap3000),
            Err(UplinkRefusal::SecondDefaultRoute("ap3000".into()))
        );

        let mut local_wan = facts();
        local_wan.config.network = Some(format!("{NETWORK}{GATEWAY_WAN}"));
        assert!(matches!(
            build(&request(), &local_wan),
            Err(UplinkRefusal::SecondDefaultRoute(_))
        ));

        let mut never = facts();
        never.config.mjolnir =
            Some(MJOLNIR.replace("option gateway 'auto'", "option gateway 'never'"));
        assert_eq!(
            build(&request(), &never),
            Err(UplinkRefusal::ExportDisabled)
        );
    }

    #[test]
    fn uplink_refuses_unselected_open_network_and_fallback_station() {
        let mut open = request();
        open.target_open = true;
        assert_eq!(
            build(&open, &facts()),
            Err(UplinkRefusal::UnselectedOpen(UPSTREAM_SSID.into()))
        );

        let mut selected = request();
        selected.target_open = true;
        selected.authorization.open = true;
        selected.passphrase = None;
        let built = build(&selected, &facts()).unwrap();
        assert!(built.secret.is_none());

        let mut no_key = request();
        no_key.passphrase = None;
        assert_eq!(
            build(&no_key, &facts()),
            Err(UplinkRefusal::MissingPassphrase)
        );

        let mut fallback = facts();
        fallback.config.wireless = Some(format!(
            "{WIRELESS}\nconfig wifi-iface 'other'\n\toption device 'radio0'\n\toption mode 'sta'\n\toption ssid 'Cafe'\n"
        ));
        assert_eq!(
            build(&request(), &fallback),
            Err(UplinkRefusal::ExistingStation("wifi-iface.other".into()))
        );
    }

    #[test]
    fn uplink_revision_and_plan_do_not_confirm_a_guessed_passphrase() {
        let right = build(&request_with(PASSPHRASE), &facts()).unwrap();
        let guess = build(&request_with("a wrong guess here"), &facts()).unwrap();
        assert_eq!(right, guess, "plans are identical whatever the passphrase");
        assert_eq!(right.plan.proposed_revision, guess.plan.proposed_revision);
        assert_eq!(
            serde_json::to_vec(&right.plan).unwrap(),
            serde_json::to_vec(&guess.plan).unwrap(),
            "plan_digest cannot be an oracle"
        );
        assert_no_secret(&visible(&right), PASSPHRASE);
        assert!(!format!("{:?}", request()).contains(PASSPHRASE));

        // A passphrase-only change still moves the revision, by generation.
        let mut committed = facts();
        committed.config = right.desired.clone();
        let rekey = build(&request_with("a newer passphrase"), &committed).unwrap();
        assert_eq!(rekey.plan.expected_revision, right.plan.proposed_revision);
        assert_ne!(rekey.plan.proposed_revision, rekey.plan.expected_revision);
        assert_eq!(rekey.secret.as_ref().unwrap().generation, 2);
        let wireless = rekey.desired.wireless.as_deref().unwrap();
        assert_eq!(
            wireless
                .matches("config wifi-iface 'mjolnir_uplink'")
                .count(),
            1
        );
    }

    #[test]
    fn uplink_captive_lease_withholds_mesh_default() {
        let captive = [
            obs(
                ObservationKind::UpstreamAssociation,
                ObservationStatus::Passed,
            ),
            obs(ObservationKind::UpstreamAddress, ObservationStatus::Passed),
            obs(ObservationKind::UpstreamRoute, ObservationStatus::Passed),
            obs(
                ObservationKind::UpstreamBoundDnsHttps,
                ObservationStatus::Failed,
            ),
        ];
        assert_eq!(default_export(&captive), DefaultExport::Withheld);
        assert_eq!(default_export(&captive[..2]), DefaultExport::Withheld);
        let mut online = captive.to_vec();
        online[3].status = ObservationStatus::Passed;
        assert_eq!(default_export(&online), DefaultExport::Announce);

        let mut fixture = Fixture::new();
        fixture.adapter.upstream = Upstream::Captive;
        let built = build(&request(), &fixture.facts()).unwrap();
        let receipt = fixture.submit(&built);
        assert_eq!(receipt.outcome, Outcome::Restored);
        assert_eq!(receipt.resulting_revision, built.plan.expected_revision);
        assert!(
            !fixture.adapter.mesh_default_announced,
            "no mesh default from a lease"
        );
        assert_eq!(
            fixture.adapter.current_revision().unwrap(),
            built.plan.expected_revision
        );
    }

    #[test]
    fn uplink_wrong_credentials_restore_previous_revision_without_upstream_evidence() {
        let mut fixture = Fixture::new();
        // A previous committed uplink with its own secret generation.
        let first = build(&request_with(PASSPHRASE), &fixture.facts()).unwrap();
        assert_eq!(fixture.submit(&first).outcome, Outcome::Committed);
        assert!(fixture.adapter.mesh_default_announced);
        let committed_secret = fs::read_to_string(fixture.adapter.secret_file(&first)).unwrap();

        fixture.adapter.upstream = Upstream::WrongPassphrase;
        fixture.adapter.mesh_default_announced = false;
        fixture.passphrase = Passphrase::new("not the passphrase").unwrap();
        let retry = request_with("not the passphrase");
        let second = build_uplink_plan(&retry, &fixture.facts(), "txn-uplink-2").unwrap();
        assert_eq!(second.plan.expected_revision, first.plan.proposed_revision);
        let receipt = fixture.submit(&second);

        assert_eq!(receipt.outcome, Outcome::Restored);
        assert_eq!(receipt.previous_revision, first.plan.proposed_revision);
        assert_eq!(receipt.resulting_revision, first.plan.proposed_revision);
        assert_eq!(
            fixture.adapter.current_revision().unwrap(),
            first.plan.proposed_revision
        );
        assert!(
            receipt
                .observations
                .iter()
                .all(|o| RESTORATION_HEALTH.contains(&o.kind)),
            "restoration did not need upstream evidence"
        );
        assert_eq!(
            fs::read_to_string(fixture.adapter.secret_file(&first)).unwrap(),
            committed_secret,
            "the previous secret is back before the result reads restored"
        );
        assert!(!fixture.adapter.mesh_default_announced);

        // Nothing controller-visible confirms either passphrase.
        for passphrase in [PASSPHRASE, "not the passphrase"] {
            assert_no_secret(&serde_json::to_string(&receipt).unwrap(), passphrase);
            for entry in walk(&fixture.paths_root()) {
                assert_no_secret(
                    &String::from_utf8_lossy(&fs::read(entry).unwrap()),
                    passphrase,
                );
            }
        }
    }

    #[test]
    fn uplink_restoration_needs_only_management_and_revision() {
        assert_eq!(
            check_restoration(&[obs(
                ObservationKind::ManagementReachability,
                ObservationStatus::Passed
            )]),
            Err(ObservationKind::AppliedRevision)
        );
        assert_eq!(
            check_restoration(&[
                obs(
                    ObservationKind::ManagementReachability,
                    ObservationStatus::Passed
                ),
                obs(ObservationKind::AppliedRevision, ObservationStatus::Passed),
                obs(
                    ObservationKind::UpstreamAssociation,
                    ObservationStatus::Failed
                ),
            ]),
            Ok(())
        );
    }

    #[test]
    fn uplink_lease_overlapping_mesh_prefixes_conflicts() {
        assert!(lease_conflicts(Ipv4Addr::new(10, 42, 3, 20), 24));
        assert!(lease_conflicts(Ipv4Addr::new(10, 254, 9, 9), 16));
        assert!(lease_conflicts(Ipv4Addr::new(10, 0, 0, 5), 8));
        assert!(lease_conflicts(Ipv4Addr::new(192, 168, 1, 50), 24));
        assert!(!lease_conflicts(Ipv4Addr::new(192, 168, 0, 50), 24));
        assert!(!lease_conflicts(Ipv4Addr::new(10, 43, 0, 5), 16));
    }

    #[test]
    fn uplink_passphrase_rules() {
        assert_eq!(
            Passphrase::new("short"),
            Err(UplinkRefusal::InvalidPassphrase)
        );
        assert!(Passphrase::new("z".repeat(64)).is_err());
        assert!(Passphrase::new("ab".repeat(32)).is_ok());
        assert_eq!(
            format!("{:?}", Passphrase::new(PASSPHRASE).unwrap()),
            "Passphrase(<redacted>)"
        );
    }

    fn obs(kind: ObservationKind, status: ObservationStatus) -> Observation {
        let (vantage, interface) = match kind {
            ObservationKind::ManagementReachability => (Vantage::ManagementPeer, "mjolnir0"),
            ObservationKind::MeshReachability => (Vantage::MeshPeer, "br-mesh"),
            ObservationKind::AppliedRevision => (Vantage::TargetLocal, "uci"),
            _ => (Vantage::UpstreamInterface, "phy0-sta0"),
        };
        Observation {
            kind,
            status,
            vantage,
            target: "probe".into(),
            interface: interface.into(),
            boot_id: "boot-a".into(),
            monotonic_ms: 1_001,
            detail: None,
        }
    }

    fn walk(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(walk(&path));
            } else {
                files.push(path);
            }
        }
        files
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Upstream {
        Online,
        Captive,
        WrongPassphrase,
    }

    struct FixedRuntime;

    impl Runtime for FixedRuntime {
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

    /// Fake target: the four files on disk, a root-only secret directory with
    /// its own sidecar snapshot, and a scripted upstream. It announces the mesh
    /// default only through [`default_export`], in the Verifying step.
    struct FakeUplinkAdapter {
        config_root: PathBuf,
        secret_root: PathBuf,
        sidecar: Option<Vec<(PathBuf, Option<Vec<u8>>)>>,
        pending_secret: Option<(String, String)>,
        upstream: Upstream,
        mesh_default_announced: bool,
    }

    impl FakeUplinkAdapter {
        fn read_config(&self) -> NodeConfig {
            let read = |c: UciConfig| fs::read_to_string(self.config_root.join(c.file_name())).ok();
            NodeConfig {
                wireless: read(UciConfig::Wireless),
                network: read(UciConfig::Network),
                firewall: read(UciConfig::Firewall),
                mjolnir: read(UciConfig::Mjolnir),
            }
        }

        fn secret_file(&self, plan: &UplinkPlan) -> PathBuf {
            let path = &plan.secret.as_ref().unwrap().path;
            self.secret_root
                .join(path.strip_prefix(&format!("{SECRET_DIR}/")).unwrap())
        }

        /// Stage the desired files and secret for the next apply.
        fn stage(&mut self, plan: &UplinkPlan, passphrase: &Passphrase) {
            for config in UciConfig::ALL {
                let path = self
                    .config_root
                    .join(format!("{}.desired", config.file_name()));
                fs::write(path, plan.desired.get(config).unwrap_or("")).unwrap();
            }
            let secret = plan.secret.as_ref().unwrap();
            self.pending_secret = Some((
                self.secret_file(plan).to_string_lossy().into_owned(),
                format!(
                    "generation {}\n{}\n",
                    secret.generation,
                    passphrase.expose()
                ),
            ));
        }
    }

    impl ApplyAdapter for FakeUplinkAdapter {
        fn current_revision(&mut self) -> Result<String, String> {
            Ok(self.read_config().revision())
        }

        fn apply(&mut self, _plan: &Plan) -> Result<(), String> {
            let (path, bytes) = self.pending_secret.take().ok_or("no staged secret")?;
            let path = PathBuf::from(path);
            // Root-only sidecar of every secret before the write.
            let mut sidecar = Vec::new();
            for entry in fs::read_dir(&self.secret_root).map_err(|e| e.to_string())? {
                let existing = entry.map_err(|e| e.to_string())?.path();
                sidecar.push((existing.clone(), Some(fs::read(&existing).unwrap())));
            }
            if !path.exists() {
                sidecar.push((path.clone(), None));
            }
            self.sidecar = Some(sidecar);
            fs::write(&path, bytes).map_err(|e| e.to_string())?;
            for config in UciConfig::ALL {
                let staged = self
                    .config_root
                    .join(format!("{}.desired", config.file_name()));
                fs::rename(staged, self.config_root.join(config.file_name()))
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }

        fn check_health(&mut self, _plan: &Plan) -> Result<Vec<Observation>, String> {
            use ObservationKind::*;
            use ObservationStatus::*;
            let status = |kind| match (self.upstream, kind) {
                (Upstream::WrongPassphrase, UpstreamAssociation) => Failed,
                (Upstream::WrongPassphrase, UpstreamAddress | UpstreamRoute) => Unknown,
                (Upstream::WrongPassphrase, UpstreamBoundDnsHttps) => Unknown,
                (Upstream::Captive, UpstreamBoundDnsHttps) => Failed,
                _ => Passed,
            };
            let mut observations: Vec<Observation> = EXPORT_GATE
                .into_iter()
                .map(|kind| obs(kind, status(kind)))
                .collect();
            if default_export(&observations) == DefaultExport::Announce {
                self.mesh_default_announced = true;
            }
            observations.push(obs(MeshReachability, Passed));
            observations.push(obs(ManagementReachability, Passed));
            Ok(observations)
        }

        fn verify_restoration(
            &mut self,
            previous_revision: &str,
        ) -> Result<Vec<Observation>, String> {
            // Withdraw any export, then put the previous secret back before
            // the result can read restored.
            self.mesh_default_announced = false;
            for (path, bytes) in self.sidecar.take().unwrap_or_default() {
                match bytes {
                    Some(bytes) => fs::write(&path, bytes).map_err(|e| e.to_string())?,
                    None => {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
            if self.read_config().revision() != previous_revision {
                return Err("restored files do not match the previous revision".into());
            }
            let observations = vec![
                obs(
                    ObservationKind::ManagementReachability,
                    ObservationStatus::Passed,
                ),
                obs(ObservationKind::AppliedRevision, ObservationStatus::Passed),
            ];
            check_restoration(&observations)
                .map_err(|missing| format!("restoration did not prove {missing:?}"))?;
            Ok(observations)
        }
    }

    impl ProvesUpstream for FakeUplinkAdapter {}

    struct Fixture {
        temp: tempfile::TempDir,
        paths: TxnPaths,
        adapter: FakeUplinkAdapter,
        passphrase: Passphrase,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let config_root = temp.path().join("config");
            let secret_root = temp.path().join("secrets");
            fs::create_dir_all(&config_root).unwrap();
            fs::create_dir_all(&secret_root).unwrap();
            let config = config();
            for c in UciConfig::ALL {
                fs::write(config_root.join(c.file_name()), config.get(c).unwrap()).unwrap();
            }
            let paths = TxnPaths::new(temp.path().join("txn"), &config_root);
            Self {
                temp,
                paths,
                adapter: FakeUplinkAdapter {
                    config_root,
                    secret_root,
                    sidecar: None,
                    pending_secret: None,
                    upstream: Upstream::Online,
                    mesh_default_announced: false,
                },
                passphrase: Passphrase::new(PASSPHRASE).unwrap(),
            }
        }

        fn facts(&self) -> UplinkFacts {
            UplinkFacts {
                config: self.adapter.read_config(),
                ..facts()
            }
        }

        fn paths_root(&self) -> PathBuf {
            self.temp.path().join("txn")
        }

        fn submit(&mut self, plan: &UplinkPlan) -> Receipt {
            let engine = Engine::new(self.paths.clone()).with_runtime(Arc::new(FixedRuntime));
            self.adapter.stage(plan, &self.passphrase);
            submit_uplink_plan(&engine, plan, &mut self.adapter).unwrap()
        }
    }
}
