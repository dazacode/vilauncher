//! Known problems shipped with the launcher: a versioned JSON manifest that names a
//! `VICTORY_*` switch other players reported for a symptom, and optionally advice for
//! a switch that suits certain hardware.
//!
//! Entries are suggestions only. A matching entry appears in the project's known-problem
//! list as a note that names its source; the launcher never sets its switch. Advice is
//! applied only when the player presses Apply (launcher/actions.rs), and then only that
//! one switch.
//!
//! Schema (`known-problems/known-problems.json`; `schema` 1 or 2, 2 adds `advice`):
//! - `id`: unique, non-empty.
//! - `project`: the repo's name without its owner (`victory`); also matches a folder slug
//!   ending in `_victory`.
//! - `gpu_vendor`: optional, `nvidia`, `amd` or `intel`.
//! - `symptom`: optional text the project's last output must contain (case-insensitive).
//! - `switch`: one `VICTORY_NAME=value` pair.
//! - `summary`: what the problem looks like, shown to the player.
//! - `source`: where the entry comes from (a report, a run, a repo doc). Shown with it.
//! - `status`: `single_run_report`, `confirmed` or `retired` (retired entries are never shown).
//! - `advice` (schema 2, optional): a switch that may suit this PC. An entry with advice
//!   names its hardware in `advice.when` only, so it has no `gpu_vendor` or `symptom`.
//!   - `switch`: one `VICTORY_NAME=value` pair.
//!   - `evidence`: `single_run_report`, `self_reported` or `two_machine_ab`. Only
//!     `two_machine_ab` is presented as recommended; the rest say "possible setting, not
//!     verified on your hardware".
//!   - `source`: the citation, shown with it.
//!   - `when`: a non-empty list of conditions, all of which must hold. Each is one key:
//!     `gpu_vendor` (`nvidia`, `amd` or `intel`; some card is that maker), `gpu_name_contains`
//!     (text in some card's name, case-insensitive), `ram_gb_at_least`, `ram_gb_at_most`,
//!     `os_build_at_least` (Windows build number) and `windows_only` (`true`).
//!     A condition whose hardware fact is unknown does not hold.

use crate::system::{SystemInfo, Vendor};
use serde::Deserialize;

const SHIPPED: &str = include_str!("../known-problems/known-problems.json");
/// The manifest schema this launcher writes: 2 adds the advice block.
pub(crate) const SCHEMA: u32 = 2;
/// The schemas this launcher reads. Schema 1 (no advice) still loads.
const READABLE: [u32; 2] = [1, 2];

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub schema: u32,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub id: String,
    pub project: String,
    #[serde(default)]
    pub gpu_vendor: Option<String>,
    #[serde(default)]
    pub symptom: Option<String>,
    pub switch: String,
    pub summary: String,
    pub source: String,
    pub status: Status,
    #[serde(default)]
    pub advice: Option<Advice>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Advice {
    pub switch: String,
    pub evidence: Evidence,
    pub source: String,
    pub when: Vec<Condition>,
}

/// How strong the evidence behind a piece of advice is. Unknown names do not parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Evidence {
    SingleRunReport,
    SelfReported,
    TwoMachineAb,
}

/// One hardware condition, written as a single key: `{"ram_gb_at_least": 16}`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Condition {
    GpuVendor(String),
    GpuNameContains(String),
    RamGbAtLeast(f64),
    RamGbAtMost(f64),
    OsBuildAtLeast(u32),
    WindowsOnly(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    SingleRunReport,
    Confirmed,
    Retired,
}

/// How advice is shown: only `Recommended` is ever called recommended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presentation {
    Recommended,
    Possible,
}

/// The rule: only two-machine A/B evidence is recommended.
pub(crate) fn presentation(evidence: Evidence) -> Presentation {
    match evidence {
        Evidence::TwoMachineAb => Presentation::Recommended,
        Evidence::SingleRunReport | Evidence::SelfReported => Presentation::Possible,
    }
}

impl Evidence {
    /// How the player reads the evidence behind a piece of advice.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Evidence::SingleRunReport => {
                msg_str!("possible setting, not verified on your hardware (reported in a single run)")
            }
            Evidence::SelfReported => {
                msg_str!("possible setting, not verified on your hardware (self-reported)")
            }
            Evidence::TwoMachineAb => msg_str!("recommended (checked on two machines)"),
        }
    }
}

impl Entry {
    /// How the player reads the entry's status.
    pub(crate) fn status_label(&self) -> &'static str {
        match self.status {
            Status::SingleRunReport => msg_str!("reported in a single run"),
            Status::Confirmed => msg_str!("confirmed"),
            Status::Retired => msg_str!("retired"),
        }
    }

    /// The switch's name (the part before `=`).
    pub(crate) fn switch_name(&self) -> &str {
        self.switch.split('=').next().unwrap_or("")
    }
}

impl Advice {
    /// The advice switch's name (the part before `=`).
    pub(crate) fn switch_name(&self) -> &str {
        self.switch.split('=').next().unwrap_or("")
    }
}

/// What the launcher knows about this PC. Every fact is optional: a fact that is not
/// known does not satisfy a condition about it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Hardware {
    /// Each card's maker (`nvidia`, `amd` or `intel`; None for another maker) and name.
    pub gpus: Vec<(Option<&'static str>, String)>,
    pub ram_gb: Option<f64>,
    /// The Windows build number (None off Windows, and when Windows does not say).
    pub os_build: Option<u32>,
    /// Running on Windows.
    pub windows: bool,
}

impl Hardware {
    pub(crate) fn from_system(system: &SystemInfo, windows: bool) -> Self {
        Self {
            gpus: system
                .gpus
                .iter()
                .map(|gpu| (vendor_key(gpu.vendor), gpu.name.clone()))
                .collect(),
            ram_gb: system.ram_gb,
            os_build: system.os_build,
            windows,
        }
    }
}

fn vendor_key(vendor: Vendor) -> Option<&'static str> {
    match vendor {
        Vendor::Nvidia => Some("nvidia"),
        Vendor::Amd => Some("amd"),
        Vendor::Intel => Some("intel"),
        Vendor::Apple | Vendor::Other => None,
    }
}

/// The manifest the launcher ships with.
pub(crate) fn shipped() -> Result<Manifest, String> {
    parse(SHIPPED)
}

/// Reads and checks a manifest.
pub(crate) fn parse(text: &str) -> Result<Manifest, String> {
    let manifest: Manifest = serde_json::from_str(text)
        .map_err(|error| format!("not a valid known-problems manifest: {error}"))?;
    validate(&manifest)?;
    Ok(manifest)
}

fn validate(manifest: &Manifest) -> Result<(), String> {
    if !READABLE.contains(&manifest.schema) {
        return Err(format!(
            "schema {} is not supported (expected {SCHEMA})",
            manifest.schema
        ));
    }
    let mut ids: Vec<&str> = Vec::new();
    for entry in &manifest.entries {
        let id = entry.id.as_str();
        if id.trim().is_empty() {
            return Err("an entry has no id".to_string());
        }
        if ids.contains(&id) {
            return Err(format!("the id {id} is used twice"));
        }
        ids.push(id);
        if entry.project.trim().is_empty() || entry.project.contains(['/', '\\']) {
            return Err(format!("{id}: project must be a repo name without its owner"));
        }
        if let Some(gpu) = &entry.gpu_vendor {
            if !matches!(gpu.as_str(), "nvidia" | "amd" | "intel") {
                return Err(format!("{id}: gpu_vendor {gpu} is not nvidia, amd or intel"));
            }
        }
        if entry.symptom.as_deref().is_some_and(|s| s.trim().is_empty()) {
            return Err(format!("{id}: symptom is empty; leave it out instead"));
        }
        if !valid_switch(&entry.switch) {
            return Err(format!("{id}: switch must be VICTORY_NAME=value"));
        }
        if entry.summary.trim().is_empty() || entry.source.trim().is_empty() {
            return Err(format!("{id}: summary and source are required"));
        }
        if let Some(advice) = &entry.advice {
            if manifest.schema < 2 {
                return Err(format!("{id}: advice needs schema 2"));
            }
            if entry.gpu_vendor.is_some() || entry.symptom.is_some() {
                return Err(format!(
                    "{id}: an entry with advice names its hardware in advice.when, not gpu_vendor or symptom"
                ));
            }
            validate_advice(id, advice)?;
        }
    }
    Ok(())
}

fn validate_advice(id: &str, advice: &Advice) -> Result<(), String> {
    if !valid_switch(&advice.switch) {
        return Err(format!("{id}: advice.switch must be VICTORY_NAME=value"));
    }
    if advice.source.trim().is_empty() {
        return Err(format!("{id}: advice.source is required"));
    }
    if advice.when.is_empty() {
        return Err(format!("{id}: advice.when needs at least one hardware condition"));
    }
    for condition in &advice.when {
        let ok = match condition {
            Condition::GpuVendor(vendor) => matches!(vendor.as_str(), "nvidia" | "amd" | "intel"),
            Condition::GpuNameContains(text) => !text.trim().is_empty(),
            Condition::RamGbAtLeast(gb) | Condition::RamGbAtMost(gb) => gb.is_finite() && *gb > 0.0,
            Condition::OsBuildAtLeast(build) => *build > 0,
            Condition::WindowsOnly(on) => *on,
        };
        if !ok {
            return Err(format!("{id}: advice.when has a condition with a bad value: {condition:?}"));
        }
    }
    Ok(())
}

/// `VICTORY_` then upper-case letters, digits and `_`, then `=` and a plain value.
fn valid_switch(text: &str) -> bool {
    let Some((name, value)) = text.split_once('=') else {
        return false;
    };
    let Some(rest) = name.strip_prefix("VICTORY_") else {
        return false;
    };
    let name_ok = !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    let value_ok = !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
    name_ok && value_ok
}

/// Whether the entry's project is the repo `slug` (a folder name such as `owner_victory`,
/// or a repo name such as `owner/victory`).
fn project_matches(project: &str, slug: &str) -> bool {
    let project = project.to_lowercase();
    let slug = slug.to_lowercase();
    slug == project || slug.ends_with(&format!("_{project}")) || slug.ends_with(&format!("/{project}"))
}

/// Whether the project's switch text already sets `name`.
fn already_set(switches: &str, name: &str) -> bool {
    let prefix = format!("{name}=");
    switches
        .lines()
        .any(|line| line.trim_start().starts_with(&prefix))
}

/// The known-problem entries that apply to a project now. `switches` is the project's
/// switch lines; an entry whose switch the player already sets is left out, as the repo
/// notes are. Entries with advice are not problems and are left out (see `advice_for`).
pub(crate) fn matching<'a>(
    manifest: &'a Manifest,
    slug: &str,
    gpu: Option<&str>,
    last_log: &[String],
    switches: &str,
) -> Vec<&'a Entry> {
    manifest
        .entries
        .iter()
        .filter(|entry| entry.status != Status::Retired)
        .filter(|entry| entry.advice.is_none())
        .filter(|entry| project_matches(&entry.project, slug))
        .filter(|entry| match &entry.gpu_vendor {
            Some(vendor) => gpu == Some(vendor.as_str()),
            None => true,
        })
        .filter(|entry| match &entry.symptom {
            Some(symptom) => {
                let symptom = symptom.to_lowercase();
                last_log
                    .iter()
                    .any(|line| line.to_lowercase().contains(&symptom))
            }
            None => true,
        })
        .filter(|entry| !already_set(switches, entry.switch_name()))
        .collect()
}

/// Whether one condition holds for this PC. Unknown facts never hold.
pub(crate) fn condition_holds(condition: &Condition, hardware: &Hardware) -> bool {
    match condition {
        Condition::GpuVendor(vendor) => hardware
            .gpus
            .iter()
            .any(|(key, _)| *key == Some(vendor.as_str())),
        Condition::GpuNameContains(text) => {
            let text = text.to_lowercase();
            hardware
                .gpus
                .iter()
                .any(|(_, name)| name.to_lowercase().contains(&text))
        }
        Condition::RamGbAtLeast(gb) => hardware.ram_gb.is_some_and(|ram| ram >= *gb),
        Condition::RamGbAtMost(gb) => hardware.ram_gb.is_some_and(|ram| ram <= *gb),
        Condition::OsBuildAtLeast(build) => hardware.os_build.is_some_and(|have| have >= *build),
        Condition::WindowsOnly(_) => hardware.windows,
    }
}

/// Whether every condition of the advice holds for this PC (and there is at least one).
pub(crate) fn advice_applies(advice: &Advice, hardware: &Hardware) -> bool {
    !advice.when.is_empty()
        && advice
            .when
            .iter()
            .all(|condition| condition_holds(condition, hardware))
}

/// The advice entries that apply to a project on this PC now: not retired, for the
/// project, every condition holds, and the advice switch is not already set.
pub(crate) fn advice_for<'a>(
    manifest: &'a Manifest,
    slug: &str,
    hardware: &Hardware,
    switches: &str,
) -> Vec<&'a Entry> {
    manifest
        .entries
        .iter()
        .filter(|entry| entry.status != Status::Retired)
        .filter(|entry| project_matches(&entry.project, slug))
        .filter(|entry| {
            entry.advice.as_ref().is_some_and(|advice| {
                advice_applies(advice, hardware) && !already_set(switches, advice.switch_name())
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{
        "schema": 1,
        "entries": [{
            "id": "a", "project": "victory", "gpu_vendor": "nvidia",
            "symptom": "Black Surface", "switch": "VICTORY_A=1",
            "summary": "Black surfaces.", "source": "one run", "status": "single_run_report"
        }]
    }"#;

    fn with(entry_json: &str) -> String {
        format!(r#"{{"schema": 2, "entries": [{entry_json}]}}"#)
    }

    /// An advice entry for VICTORY_ADV=1 whose conditions are `when`.
    fn advised(evidence: &str, when: &str) -> String {
        with(&format!(
            r#"{{"id":"adv","project":"victory","switch":"VICTORY_BASE=1","summary":"s","source":"x","status":"confirmed",
               "advice":{{"switch":"VICTORY_ADV=1","evidence":"{evidence}","source":"cite","when":[{when}]}}}}"#
        ))
    }

    fn pc() -> Hardware {
        Hardware {
            gpus: vec![(Some("nvidia"), "NVIDIA GeForce RTX 4070".to_string())],
            ram_gb: Some(32.0),
            os_build: Some(22631),
            windows: true,
        }
    }

    fn holds(when: &str, hardware: &Hardware) -> bool {
        let manifest = parse(&advised("self_reported", when)).unwrap();
        let advice = manifest.entries[0].advice.as_ref().unwrap();
        advice_applies(advice, hardware)
    }

    #[test]
    fn the_shipped_manifest_is_valid_and_names_its_source() {
        let manifest = shipped().expect("the shipped manifest parses");
        assert_eq!(manifest.schema, SCHEMA);
        assert!(!manifest.entries.is_empty());
        for entry in &manifest.entries {
            assert!(!entry.source.trim().is_empty());
            assert_ne!(entry.status, Status::Retired);
        }
        assert!(manifest
            .entries
            .iter()
            .any(|e| e.switch == "VICTORY_INPLACE=1" && e.status == Status::SingleRunReport));
    }

    #[test]
    fn the_shipped_manifest_has_no_recommended_advice() {
        // No entry has two-machine A/B evidence yet, so the shipped manifest has no advice.
        let manifest = shipped().unwrap();
        for entry in &manifest.entries {
            if let Some(advice) = &entry.advice {
                assert_ne!(advice.evidence, Evidence::TwoMachineAb, "{}", entry.id);
                assert!(!advice.source.trim().is_empty());
            }
        }
    }

    #[test]
    fn a_valid_entry_parses() {
        let manifest = parse(ONE).unwrap();
        let entry = &manifest.entries[0];
        assert_eq!(entry.switch_name(), "VICTORY_A");
        assert_eq!(entry.gpu_vendor.as_deref(), Some("nvidia"));
        assert_eq!(entry.status_label(), "reported in a single run");
        assert!(entry.advice.is_none(), "schema 1 entries carry no advice");
    }

    #[test]
    fn bad_manifests_are_refused() {
        let bad = [
            r#"{"schema": 3, "entries": []}"#.to_string(),
            r#"{"schema": 2, "entries": [], "extra": true}"#.to_string(),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"bogus"}"#),
            with(r#"{"id":"a","project":"a/victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"PATH=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_a=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=C:\\x","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","gpu_vendor":"matrox","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","symptom":" ","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"","source":"x","status":"confirmed"}"#),
            format!(
                r#"{{"schema": 2, "entries": [{e}, {e}]}}"#,
                e = r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#
            ),
            // An unknown evidence status is refused.
            advised("trusted_forum", r#"{"ram_gb_at_least": 8}"#),
            // Advice needs a condition, a source, a valid switch and good values.
            advised("self_reported", ""),
            advised("self_reported", r#"{"gpu_vendor": "matrox"}"#),
            advised("self_reported", r#"{"ram_gb_at_least": -1}"#),
            advised("self_reported", r#"{"windows_only": false}"#),
            advised("self_reported", r#"{"wifi_speed": 3}"#),
            advised("self_reported", r#"{"gpu_name_contains": " "}"#),
            // Advice in a schema 1 manifest, advice with a symptom or gpu_vendor, a bad switch.
            advised("self_reported", r#"{"windows_only": true}"#).replace("\"schema\": 2", "\"schema\": 1"),
            advised("self_reported", r#"{"windows_only": true}"#).replace("\"status\":\"confirmed\"", "\"status\":\"confirmed\",\"symptom\":\"x\""),
            advised("self_reported", r#"{"windows_only": true}"#).replace("\"status\":\"confirmed\"", "\"status\":\"confirmed\",\"gpu_vendor\":\"nvidia\""),
            advised("self_reported", r#"{"windows_only": true}"#).replace("VICTORY_ADV=1", "PATH=1"),
        ];
        for text in bad {
            assert!(parse(&text).is_err(), "should be refused: {text}");
        }
    }

    #[test]
    fn a_schema_1_manifest_without_advice_still_loads() {
        assert!(parse(ONE).is_ok());
        assert!(parse(&with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#)).is_ok());
    }

    #[test]
    fn project_matches_the_repo_name_in_any_case() {
        assert!(project_matches("victory", "owner_victory"));
        assert!(project_matches("Victory", "OWNER_VICTORY"));
        assert!(project_matches("victory", "victory"));
        assert!(project_matches("victory", "owner/victory"));
        assert!(!project_matches("victory", "owner_victory2"));
        assert!(!project_matches("tory", "owner_victory"));
    }

    #[test]
    fn matching_needs_project_gpu_and_symptom() {
        let manifest = parse(ONE).unwrap();
        let log = vec!["frame: BLACK SURFACE on the roof".to_string()];
        let found = |slug: &str, gpu: Option<&str>, log: &[String], switches: &str| {
            matching(&manifest, slug, gpu, log, switches).len()
        };
        assert_eq!(found("owner_victory", Some("nvidia"), &log, ""), 1);
        assert_eq!(found("owner_other", Some("nvidia"), &log, ""), 0, "other project");
        assert_eq!(found("owner_victory", Some("amd"), &log, ""), 0, "other gpu");
        assert_eq!(found("owner_victory", None, &log, ""), 0, "gpu unknown");
        assert_eq!(found("owner_victory", Some("nvidia"), &[], ""), 0, "no symptom in log");
    }

    #[test]
    fn a_switch_the_player_already_sets_is_not_suggested_and_retired_is_never() {
        let manifest = parse(&with(
            r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#,
        ))
        .unwrap();
        assert_eq!(matching(&manifest, "owner_victory", None, &[], "VICTORY_A=0").len(), 0);
        assert_eq!(matching(&manifest, "owner_victory", None, &[], "VICTORY_B=0").len(), 1);

        let retired = parse(&with(
            r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"retired"}"#,
        ))
        .unwrap();
        assert!(matching(&retired, "owner_victory", None, &[], "").is_empty());
    }

    #[test]
    fn each_condition_on_its_own_matches_the_pc() {
        let hw = pc();
        assert!(holds(r#"{"gpu_vendor": "nvidia"}"#, &hw));
        assert!(!holds(r#"{"gpu_vendor": "amd"}"#, &hw));
        assert!(holds(r#"{"gpu_name_contains": "rtx 4070"}"#, &hw), "case-insensitive");
        assert!(!holds(r#"{"gpu_name_contains": "radeon"}"#, &hw));
        assert!(holds(r#"{"ram_gb_at_least": 32}"#, &hw));
        assert!(!holds(r#"{"ram_gb_at_least": 33}"#, &hw));
        assert!(holds(r#"{"ram_gb_at_most": 32}"#, &hw));
        assert!(!holds(r#"{"ram_gb_at_most": 16}"#, &hw));
        assert!(holds(r#"{"os_build_at_least": 22621}"#, &hw));
        assert!(!holds(r#"{"os_build_at_least": 26100}"#, &hw));
        assert!(holds(r#"{"windows_only": true}"#, &hw));
        let mac = Hardware { windows: false, ..pc() };
        assert!(!holds(r#"{"windows_only": true}"#, &mac));
    }

    #[test]
    fn unknown_hardware_data_is_no_match() {
        let unknown = Hardware::default();
        for when in [
            r#"{"gpu_vendor": "nvidia"}"#,
            r#"{"gpu_name_contains": "rtx"}"#,
            r#"{"ram_gb_at_least": 1}"#,
            r#"{"ram_gb_at_most": 999}"#,
            r#"{"os_build_at_least": 1}"#,
            r#"{"windows_only": true}"#,
        ] {
            assert!(!holds(when, &unknown), "{when} must not match unknown hardware");
        }
        // A card of another maker has no key, so a vendor condition does not match it.
        let other = Hardware {
            gpus: vec![(None, "Apple M3".to_string())],
            ..Hardware::default()
        };
        assert!(!holds(r#"{"gpu_vendor": "nvidia"}"#, &other));
        assert!(holds(r#"{"gpu_name_contains": "m3"}"#, &other));
    }

    #[test]
    fn every_condition_must_hold() {
        let manifest = parse(&advised(
            "self_reported",
            r#"{"gpu_vendor": "nvidia"}, {"ram_gb_at_least": 64}"#,
        ))
        .unwrap();
        assert!(advice_for(&manifest, "owner_victory", &pc(), "").is_empty(), "32 GB is not 64");
        let bigger = Hardware { ram_gb: Some(64.0), ..pc() };
        assert_eq!(advice_for(&manifest, "owner_victory", &bigger, "").len(), 1);
    }

    #[test]
    fn advice_is_for_its_project_and_not_already_set() {
        let manifest = parse(&advised("two_machine_ab", r#"{"windows_only": true}"#)).unwrap();
        assert_eq!(advice_for(&manifest, "owner_victory", &pc(), "").len(), 1);
        assert!(advice_for(&manifest, "owner_other", &pc(), "").is_empty(), "other project");
        assert!(advice_for(&manifest, "owner_victory", &pc(), "VICTORY_ADV=0").is_empty());
        assert_eq!(advice_for(&manifest, "owner_victory", &pc(), "VICTORY_BASE=0").len(), 1);

        let retired = parse(&advised("two_machine_ab", r#"{"windows_only": true}"#).replace("\"confirmed\"", "\"retired\"")).unwrap();
        assert!(advice_for(&retired, "owner_victory", &pc(), "").is_empty());
    }

    #[test]
    fn advice_entries_are_not_listed_as_known_problems() {
        let manifest = parse(&advised("self_reported", r#"{"windows_only": true}"#)).unwrap();
        assert!(matching(&manifest, "owner_victory", None, &[], "").is_empty());
    }

    #[test]
    fn only_two_machine_evidence_is_presented_as_recommended() {
        assert_eq!(presentation(Evidence::TwoMachineAb), Presentation::Recommended);
        assert_eq!(presentation(Evidence::SelfReported), Presentation::Possible);
        assert_eq!(presentation(Evidence::SingleRunReport), Presentation::Possible);
        for evidence in [Evidence::SelfReported, Evidence::SingleRunReport] {
            let label = evidence.label().to_lowercase();
            assert!(!label.contains("recommended"), "{label}");
            assert!(label.contains("not verified on your hardware"), "{label}");
        }
        assert!(Evidence::TwoMachineAb.label().contains("recommended"));
    }

    #[test]
    fn the_evidence_names_parse_and_unknown_ones_do_not() {
        for name in ["single_run_report", "self_reported", "two_machine_ab"] {
            let parsed: Result<Evidence, _> = serde_json::from_str(&format!("\"{name}\""));
            assert!(parsed.is_ok(), "{name}");
        }
        assert!(serde_json::from_str::<Evidence>("\"two_machine\"").is_err());
    }
}
