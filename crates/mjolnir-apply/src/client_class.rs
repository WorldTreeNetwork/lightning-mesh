//! One client SSID and three classes (`add-private-household-wifi`,
//! bead `mjolnir-mesh-ai0.7`).
//!
//! A pure plan. It does not read the node, write a file, or talk to a
//! radio. Recording the intention leaves the deployed SSID unchanged.
//! Member flows are scheduled ahead of unidentified flows. There is no
//! rate cap and no second SSID.
//!
//! A verified membership is an answer from the `mjolnir-mesh-ai0.1`
//! verifier, passed in by the caller. This builder does not issue
//! biscuits and does not treat SSID association or a directory label
//! as that answer.
//!
//! [`submit_class_plan`] accepts only an adapter that implements
//! [`SchedulesClients`]. [`crate::OpenWrtAdapter`] does not.

use crate::engine::{ApplyAdapter, Engine, EngineError};
use crate::model::{Plan, Receipt, SCHEMA_VERSION, UciConfig};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Overlay, WAN SSH, and the recovery address. Closed to client traffic.
pub const OVERLAY_PREFIX: &str = "10.254.0.0/16";
pub const HOUSE_ROUTE_PREFIX: &str = "10.42.0.0/16";
pub const RECOVERY_ADDRESS: &str = "192.168.1.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientClass {
    PublishedService,
    Member,
    Unidentified,
}

/// This change never retunes the radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioEffect {
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowInput {
    pub id: String,
    /// Associated to the one client SSID. That fact is not membership.
    pub associated: bool,
    /// A directory or guild label. Not a verifier answer.
    pub directory_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A name the house has published, such as `wiki.mesh`.
    PublishedName(String),
    Internet,
    /// A route to every address in the house client space or the overlay.
    HouseRoute,
    Overlay,
    WanSsh,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAttempt {
    pub flow_id: String,
    pub destination: Destination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRequest {
    pub deployed_ssid: String,
    pub commit: bool,
    /// A second SSID is refused. Visitors are unidentified clients.
    pub extra_ssids: Vec<String>,
    /// `Some` is refused. Priority is a schedule order, not a cap.
    pub rate_cap_kbps: Option<u64>,
    pub flows: Vec<FlowInput>,
    /// `.mesh` names a box on a node has published.
    pub published_names: Vec<String>,
    pub opens: Vec<OpenAttempt>,
    /// Flow id to guild, only where the ai0.1 verifier said yes.
    pub verified_members: BTreeMap<String, String>,
    pub node_id: String,
    pub transaction_id: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub ahead: ClientClass,
    pub behind: ClientClass,
    pub rate_cap_kbps: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachDecision {
    pub flow_id: String,
    pub destination: Destination,
    pub reach: Reach,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassPlan {
    pub ssid: String,
    pub radio: RadioEffect,
    pub classes: BTreeMap<String, ClientClass>,
    pub schedule: Schedule,
    pub reaches: Vec<ReachDecision>,
    /// Present only when `commit` is true. Resources are `mjolnir` alone.
    pub plan: Option<Plan>,
    /// The `mjolnir` text a later adapter would write. Wireless is absent.
    pub mjolnir_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClassRefusal {
    #[error("the deployed SSID is empty")]
    EmptySsid,
    #[error("a second SSID is refused: {0}")]
    SecondSsid(String),
    #[error("a rate cap is refused; members are scheduled ahead")]
    RateCap,
    #[error("duplicate flow id {0}")]
    DuplicateFlow(String),
    #[error("published name {0} is not a .mesh name")]
    BadPublishedName(String),
    #[error("open attempt names unknown flow {0}")]
    UnknownFlow(String),
}

/// An adapter allowed to record the class policy. The OpenWrt adapter
/// does not implement this.
///
/// ```compile_fail,E0277
/// fn assert_schedules<T: mjolnir_apply::SchedulesClients>() {}
/// fn check() {
///     assert_schedules::<mjolnir_apply::OpenWrtAdapter>();
/// }
/// ```
pub trait SchedulesClients: ApplyAdapter {}

/// Submit a committed class plan. The plan's resources are `mjolnir`
/// only, so a wireless file is not in the transaction.
pub fn submit_class_plan<A: SchedulesClients>(
    engine: &Engine,
    class_plan: &ClassPlan,
    adapter: &mut A,
) -> Result<Receipt, EngineError> {
    let plan = class_plan
        .plan
        .as_ref()
        .ok_or_else(|| EngineError::InvalidPlan("class plan is not committed".into()))?;
    if plan.resources.contains(&UciConfig::Wireless) {
        return Err(EngineError::InvalidPlan(
            "a class plan does not write wireless".into(),
        ));
    }
    engine.apply(plan, adapter)
}

/// Build the class plan. Pure.
pub fn build_class_plan(request: &ClassRequest) -> Result<ClassPlan, ClassRefusal> {
    let ssid = request.deployed_ssid.trim();
    if ssid.is_empty() {
        return Err(ClassRefusal::EmptySsid);
    }
    if let Some(extra) = request.extra_ssids.first() {
        return Err(ClassRefusal::SecondSsid(extra.clone()));
    }
    if request.rate_cap_kbps.is_some() {
        return Err(ClassRefusal::RateCap);
    }

    let mut published = BTreeSet::new();
    for name in &request.published_names {
        if !is_mesh_name(name) {
            return Err(ClassRefusal::BadPublishedName(name.clone()));
        }
        published.insert(name.as_str());
    }

    let mut classes = BTreeMap::new();
    for flow in &request.flows {
        if classes.insert(flow.id.clone(), classify(flow, &request.verified_members)).is_some() {
            return Err(ClassRefusal::DuplicateFlow(flow.id.clone()));
        }
    }

    let mut reaches = Vec::with_capacity(request.opens.len());
    for open in &request.opens {
        if !classes.contains_key(&open.flow_id) {
            return Err(ClassRefusal::UnknownFlow(open.flow_id.clone()));
        }
        reaches.push(ReachDecision {
            flow_id: open.flow_id.clone(),
            destination: open.destination.clone(),
            reach: reach_of(&open.destination, &published),
        });
    }

    let (plan, mjolnir_policy) = if request.commit {
        let policy = render_policy(ssid);
        let digest = blake3::hash(policy.as_bytes());
        (
            Some(Plan {
                schema_version: SCHEMA_VERSION,
                transaction_id: request.transaction_id.clone(),
                node_id: request.node_id.clone(),
                expected_revision: request.expected_revision.clone(),
                proposed_revision: digest.to_hex().to_string(),
                resources: vec![UciConfig::Mjolnir],
                timeout_secs: crate::model::DEFAULT_TIMEOUT_SECS,
                required_health: Vec::new(),
            }),
            Some(policy),
        )
    } else {
        (None, None)
    };

    Ok(ClassPlan {
        ssid: ssid.to_string(),
        radio: RadioEffect::Unchanged,
        classes,
        schedule: Schedule {
            ahead: ClientClass::Member,
            behind: ClientClass::Unidentified,
            rate_cap_kbps: None,
        },
        reaches,
        plan,
        mjolnir_policy,
    })
}

fn classify(flow: &FlowInput, verified: &BTreeMap<String, String>) -> ClientClass {
    let _ = flow.associated;
    let _ = &flow.directory_label;
    if verified.contains_key(&flow.id) {
        ClientClass::Member
    } else {
        ClientClass::Unidentified
    }
}

fn reach_of(destination: &Destination, published: &BTreeSet<&str>) -> Reach {
    match destination {
        Destination::PublishedName(name) if published.contains(name.as_str()) => Reach::Allow,
        Destination::Internet => Reach::Allow,
        Destination::PublishedName(_)
        | Destination::HouseRoute
        | Destination::Overlay
        | Destination::WanSsh
        | Destination::Recovery => Reach::Deny,
    }
}

fn is_mesh_name(name: &str) -> bool {
    let Some(label) = name.strip_suffix(".mesh") else {
        return false;
    };
    !label.is_empty()
        && !label.starts_with('.')
        && !label.ends_with('.')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        && !label.contains("..")
}

fn render_policy(ssid: &str) -> String {
    format!(
        "config client_class 'house'\n\
         \toption ssid '{ssid}'\n\
         \toption radio 'unchanged'\n\
         \toption schedule 'member-ahead'\n\
         \toption rate_cap 'none'\n\
         \tlist closed '{OVERLAY_PREFIX}'\n\
         \tlist closed '{HOUSE_ROUTE_PREFIX}'\n\
         \tlist closed '{RECOVERY_ADDRESS}'\n\
         \toption wan_ssh 'closed'\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ApplyAdapter;
    use crate::model::Observation;

    fn request() -> ClassRequest {
        ClassRequest {
            deployed_ssid: "Lightning Mesh".into(),
            commit: false,
            extra_ssids: Vec::new(),
            rate_cap_kbps: None,
            flows: vec![
                FlowInput {
                    id: "member".into(),
                    associated: true,
                    directory_label: None,
                },
                FlowInput {
                    id: "phone".into(),
                    associated: true,
                    directory_label: Some("guest".into()),
                },
            ],
            published_names: vec!["wiki.mesh".into()],
            opens: vec![
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::PublishedName("wiki.mesh".into()),
                },
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::HouseRoute,
                },
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::Overlay,
                },
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::WanSsh,
                },
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::Recovery,
                },
                OpenAttempt {
                    flow_id: "phone".into(),
                    destination: Destination::Internet,
                },
            ],
            verified_members: BTreeMap::from([("member".into(), "house".into())]),
            node_id: "node-a".into(),
            transaction_id: "txn-class".into(),
            expected_revision: "rev-1".into(),
        }
    }

    #[test]
    fn uncommitted_plan_leaves_the_radio_alone() {
        let plan = build_class_plan(&request()).unwrap();
        assert_eq!(plan.radio, RadioEffect::Unchanged);
        assert_eq!(plan.ssid, "Lightning Mesh");
        assert!(plan.plan.is_none());
        assert!(plan.mjolnir_policy.is_none());
    }

    #[test]
    fn a_second_ssid_and_a_rate_cap_are_refused() {
        let mut second = request();
        second.extra_ssids.push("Visitors".into());
        assert!(matches!(
            build_class_plan(&second),
            Err(ClassRefusal::SecondSsid(_))
        ));
        let mut capped = request();
        capped.rate_cap_kbps = Some(1_000);
        assert_eq!(build_class_plan(&capped), Err(ClassRefusal::RateCap));
    }

    #[test]
    fn association_and_a_label_are_not_membership() {
        let plan = build_class_plan(&request()).unwrap();
        assert_eq!(plan.classes["member"], ClientClass::Member);
        assert_eq!(plan.classes["phone"], ClientClass::Unidentified);
        assert_eq!(plan.schedule.ahead, ClientClass::Member);
        assert_eq!(plan.schedule.behind, ClientClass::Unidentified);
        assert_eq!(plan.schedule.rate_cap_kbps, None);
    }

    #[test]
    fn an_unidentified_client_reaches_a_published_name_and_not_management() {
        let plan = build_class_plan(&request()).unwrap();
        let reach = |destination: &Destination| {
            plan.reaches
                .iter()
                .find(|item| &item.destination == destination)
                .unwrap()
                .reach
        };
        assert_eq!(
            reach(&Destination::PublishedName("wiki.mesh".into())),
            Reach::Allow
        );
        assert_eq!(reach(&Destination::Internet), Reach::Allow);
        assert_eq!(reach(&Destination::HouseRoute), Reach::Deny);
        assert_eq!(reach(&Destination::Overlay), Reach::Deny);
        assert_eq!(reach(&Destination::WanSsh), Reach::Deny);
        assert_eq!(reach(&Destination::Recovery), Reach::Deny);
    }

    #[test]
    fn without_a_verifier_answer_every_client_is_unidentified() {
        let mut request = request();
        request.verified_members.clear();
        let plan = build_class_plan(&request).unwrap();
        assert!(plan.classes.values().all(|class| *class == ClientClass::Unidentified));
    }

    #[test]
    fn a_committed_plan_writes_mjolnir_and_not_wireless() {
        let mut request = request();
        request.commit = true;
        let plan = build_class_plan(&request).unwrap();
        assert_eq!(plan.radio, RadioEffect::Unchanged);
        let inner = plan.plan.unwrap();
        assert_eq!(inner.resources, vec![UciConfig::Mjolnir]);
        let policy = plan.mjolnir_policy.unwrap();
        assert!(policy.contains("option radio 'unchanged'"));
        assert!(policy.contains("option schedule 'member-ahead'"));
        assert!(policy.contains("option rate_cap 'none'"));
        assert!(!policy.contains("wifi-iface"));
    }

    struct FakeClassAdapter;

    impl ApplyAdapter for FakeClassAdapter {
        fn current_revision(&mut self) -> Result<String, String> {
            Ok("rev-1".into())
        }
        fn apply(&mut self, plan: &Plan) -> Result<(), String> {
            assert_eq!(plan.resources, vec![UciConfig::Mjolnir]);
            Ok(())
        }
        fn check_health(&mut self, _plan: &Plan) -> Result<Vec<Observation>, String> {
            Ok(Vec::new())
        }
        fn verify_restoration(
            &mut self,
            _previous_revision: &str,
        ) -> Result<Vec<Observation>, String> {
            Ok(Vec::new())
        }
    }

    impl SchedulesClients for FakeClassAdapter {}

    fn _scheduler<T: SchedulesClients>() {}

    #[test]
    fn the_builder_does_not_construct_an_adapter() {
        // Building the plan takes no adapter. The fake implements the
        // submit gate. OpenWrtAdapter does not: see the compile_fail
        // doctest on [`SchedulesClients`].
        let plan = build_class_plan(&request()).unwrap();
        assert!(plan.plan.is_none());
        _scheduler::<FakeClassAdapter>();
    }
}
