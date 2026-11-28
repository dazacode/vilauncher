//! Known problems shipped with the launcher: a versioned JSON manifest that names a
//! `VICTORY_*` switch other players reported for a symptom.
//!
//! Entries are suggestions only. A matching entry appears in the project's known-problem
//! list as a note that names its source; the launcher never sets its switch. Matching
//! needs the project (the repo's name, case-insensitively), the video card vendor when the
//! entry names one, and the symptom text when the entry names one (it must appear in the
//! project's last output).
//!
//! Schema (`known-problems/known-problems.json`, `schema` must be 1):
//! - `id`: unique, non-empty.
//! - `project`: the repo's name without its owner (`victory`); also matches a folder slug
//!   ending in `_victory`.
//! - `gpu_vendor`: optional, `nvidia`, `amd` or `intel`.
//! - `symptom`: optional text the project's last output must contain (case-insensitive).
//! - `switch`: one `VICTORY_NAME=value` pair.
//! - `summary`: what the problem looks like, shown to the player.
//! - `source`: where the entry comes from (a report, a run, a repo doc). Shown with it.
//! - `status`: `single_run_report`, `confirmed` or `retired` (retired entries are never shown).

use serde::Deserialize;

const SHIPPED: &str = include_str!("../known-problems/known-problems.json");
/// The only manifest schema this launcher reads.
pub(crate) const SCHEMA: u32 = 1;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    SingleRunReport,
    Confirmed,
    Retired,
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
    if manifest.schema != SCHEMA {
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

/// Whether the entry's project is the repo `slug` (a folder name such as `owner_victory`).
fn project_matches(project: &str, slug: &str) -> bool {
    let project = project.to_lowercase();
    let slug = slug.to_lowercase();
    slug == project || slug.ends_with(&format!("_{project}")) || slug.ends_with(&format!("/{project}"))
}

/// The entries that apply to a project now. `switches` is the project's switch lines;
/// an entry whose switch the player already sets is left out, as the repo notes are.
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
        .filter(|entry| {
            let prefix = format!("{}=", entry.switch_name());
            !switches
                .lines()
                .any(|line| line.trim_start().starts_with(&prefix))
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
        format!(r#"{{"schema": 1, "entries": [{entry_json}]}}"#)
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
    fn a_valid_entry_parses() {
        let manifest = parse(ONE).unwrap();
        let entry = &manifest.entries[0];
        assert_eq!(entry.switch_name(), "VICTORY_A");
        assert_eq!(entry.gpu_vendor.as_deref(), Some("nvidia"));
        assert_eq!(entry.status_label(), "reported in a single run");
    }

    #[test]
    fn bad_manifests_are_refused() {
        let bad = [
            r#"{"schema": 2, "entries": []}"#.to_string(),
            r#"{"schema": 1, "entries": [], "extra": true}"#.to_string(),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"bogus"}"#),
            with(r#"{"id":"a","project":"a/victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"PATH=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_a=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=C:\\x","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","gpu_vendor":"matrox","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","symptom":" ","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#),
            with(r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"","source":"x","status":"confirmed"}"#),
            format!(
                r#"{{"schema": 1, "entries": [{e}, {e}]}}"#,
                e = r#"{"id":"a","project":"victory","switch":"VICTORY_A=1","summary":"s","source":"x","status":"confirmed"}"#
            ),
        ];
        for text in bad {
            assert!(parse(&text).is_err(), "should be refused: {text}");
        }
    }

    #[test]
    fn project_matches_the_repo_name_in_any_case() {
        assert!(project_matches("victory", "owner_victory"));
        assert!(project_matches("Victory", "OWNER_VICTORY"));
        assert!(project_matches("victory", "victory"));
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
}
