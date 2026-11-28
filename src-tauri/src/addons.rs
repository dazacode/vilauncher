//! Add-ons: optional features for a VIctory project, described by a JSON manifest.
//!
//! A manifest names a source repository at a pinned commit, an optional patch series
//! (`git am`) that the project's code needs, the `VICTORY_*` switches to turn on, and
//! launch arguments. Nothing is on by default. Turning an add-on on puts its patches on
//! a `vilauncher/<id>` branch of the project; turning it off goes back to the branch the
//! project was on. Manifests ship with the launcher, or the player can drop more into
//! the manifests folder.

use crate::tools;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The manifests the launcher ships with. Their ids cannot be replaced by a player's file.
const BUILT_IN: &[&str] = &[include_str!("../addons/memory-stack.json")];

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Addon {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub source: Source,
    #[serde(default)]
    pub patches: Option<Patches>,
    #[serde(default)]
    pub switches: BTreeMap<String, String>,
    #[serde(default)]
    pub args: Vec<String>,
    /// Where the port is described, for a project the patches do not apply to.
    #[serde(default)]
    pub guide: Option<String>,
    /// What turning the add-on on changes, shown before the player confirms.
    pub warning: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Source {
    /// An https clone address.
    pub repo: String,
    /// The commit (or tag) the add-on was written against.
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Patches {
    /// Folder of the source holding the `*.patch` files, applied in name order.
    pub dir: String,
    /// The commit the patches were made on; informational, shown in the warning.
    #[serde(default)]
    pub base: Option<String>,
}

/// What a project has from an add-on, kept so turning it off puts back exactly what was there.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Applied {
    /// The branch the project was on (or its commit when detached).
    pub original: String,
    /// The add-on's branch; empty when the add-on only sets switches.
    pub branch: String,
    /// Each switch the add-on sets, with the value the player had before (None: not set).
    #[serde(default)]
    pub prior: BTreeMap<String, Option<String>>,
    /// The launch arguments the add-on added (arguments already present are not listed).
    #[serde(default)]
    pub added_args: Vec<String>,
}

/// Reads and checks one manifest.
pub fn parse(text: &str) -> Result<Addon, String> {
    let addon: Addon =
        serde_json::from_str(text).map_err(|error| format!("not a valid add-on: {error}"))?;
    addon.validate()?;
    Ok(addon)
}

impl Addon {
    fn validate(&self) -> Result<(), String> {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 40
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !id_ok {
            return Err(format!(
                "the id \"{}\" must be 1 to 40 lowercase letters, digits or dashes",
                self.id
            ));
        }
        for (what, text) in [
            ("name", &self.name),
            ("summary", &self.summary),
            ("warning", &self.warning),
        ] {
            if text.trim().is_empty() {
                return Err(format!("{} has no {what}", self.id));
            }
        }
        if !self.source.repo.starts_with("https://") || self.source.repo.contains(char::is_whitespace)
        {
            return Err(format!("{}: the source must be an https address", self.id));
        }
        if !is_commit(&self.source.reference) {
            return Err(format!(
                "{}: the source ref must be a commit id (7 to 40 hex digits)",
                self.id
            ));
        }
        if let Some(patches) = &self.patches {
            if !is_relative(&patches.dir) {
                return Err(format!(
                    "{}: the patch folder must be a relative path inside the source",
                    self.id
                ));
            }
            if patches.base.as_deref().is_some_and(|base| !is_commit(base)) {
                return Err(format!("{}: the patch base must be a commit id", self.id));
            }
        }
        if let Some(guide) = &self.guide {
            if !is_relative(guide) {
                return Err(format!(
                    "{}: the guide must be a relative path inside the source",
                    self.id
                ));
            }
        }
        for (key, value) in &self.switches {
            let key_ok = key.starts_with("VICTORY_")
                && key
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
            if !key_ok || value.contains(['\n', '\r']) {
                return Err(format!("{}: \"{key}\" is not a VICTORY_* switch", self.id));
            }
        }
        if self.args.iter().any(|arg| arg.is_empty() || arg.contains(['\n', '\r'])) {
            return Err(format!("{}: launch arguments must be single words", self.id));
        }
        Ok(())
    }
}

fn is_commit(text: &str) -> bool {
    (7..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && !path.split(['/', '\\']).any(|part| part == "..")
}

/// The add-ons the player can choose from: the shipped ones, then the manifests in
/// `folder`. Returns the problems found in the player's files, so they can be shown.
pub fn catalog(folder: &Path) -> (Vec<Addon>, Vec<String>) {
    let mut addons = Vec::new();
    let mut problems = Vec::new();
    for text in BUILT_IN {
        match parse(text) {
            Ok(addon) => addons.push(addon),
            Err(error) => problems.push(format!("built in: {error}")),
        }
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return (addons, problems);
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    for file in files {
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
        let name = name.unwrap_or_default();
        match std::fs::read_to_string(&file)
            .map_err(|error| error.to_string())
            .and_then(|text| parse(&text))
        {
            Ok(addon) if addons.iter().any(|known| known.id == addon.id) => {
                problems.push(format!("{name}: the id \"{}\" is already used", addon.id));
            }
            Ok(addon) => addons.push(addon),
            Err(error) => problems.push(format!("{name}: {error}")),
        }
    }
    addons.sort_by(|a, b| a.name.cmp(&b.name));
    (addons, problems)
}

/// The value a switch has in `text` (its last line), or None when it is not set.
pub fn switch_value(text: &str, key: &str) -> Option<String> {
    text.lines().rev().find_map(|line| {
        let (k, value) = line.split_once('=')?;
        (k.trim() == key).then(|| value.to_string())
    })
}

/// The switch text with each switch in `prior` put back to the value it had (None: removed).
pub fn restore_switches(current: &str, prior: &BTreeMap<String, Option<String>>) -> String {
    let mut lines: Vec<String> = current
        .lines()
        .filter(|line| !line.split_once('=').is_some_and(|(key, _)| prior.contains_key(key.trim())))
        .map(str::to_owned)
        .collect();
    lines.extend(prior.iter().filter_map(|(key, value)| value.as_ref().map(|v| format!("{key}={v}"))));
    lines.join("\n")
}

/// The switch text with the add-on's switches set (`on`) or removed (off). Other lines
/// are kept as they are.
pub fn with_switches(current: &str, switches: &BTreeMap<String, String>, on: bool) -> String {
    let mut lines: Vec<String> = current
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .is_some_and(|(key, _)| switches.contains_key(key.trim()))
        })
        .map(str::to_owned)
        .collect();
    if on {
        lines.extend(switches.iter().map(|(key, value)| format!("{key}={value}")));
    }
    lines.join("\n")
}

/// The launch arguments with the add-on's words added (`on`) or taken out (off).
pub fn with_args(current: &str, args: &[String], on: bool) -> String {
    let mut words: Vec<String> = current.split_whitespace().map(str::to_owned).collect();
    if on {
        for arg in args {
            if !words.contains(arg) {
                words.push(arg.clone());
            }
        }
    } else {
        words.retain(|word| !args.contains(word));
    }
    words.join(" ")
}

fn git_output(mut c: Command) -> Result<String, String> {
    crate::launcher::hide_window(&mut c);
    let output = c
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            format!("git ended with {}", output.status)
        } else {
            error
        })
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let mut c = tools::git();
    c.current_dir(dir).args(args);
    git_output(c)
}

/// Clones the add-on's source into `cache` on first use, then checks out the pinned commit.
pub fn fetch_source(addon: &Addon, cache: &Path) -> Result<PathBuf, String> {
    if !cache.join(".git").is_dir() {
        if let Some(parent) = cache.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        let mut c = tools::git();
        c.args(["clone", "--quiet", "--"]).arg(&addon.source.repo).arg(cache);
        git_output(c).map_err(|error| format!("could not download {}: {error}", addon.source.repo))?;
    }
    git(cache, &["fetch", "--quiet", "origin"])
        .map_err(|error| format!("could not update {}: {error}", addon.source.repo))?;
    git(cache, &["checkout", "--quiet", "--detach", &addon.source.reference]).map_err(|error| {
        format!(
            "{} has no commit {}: {error}",
            addon.source.repo, addon.source.reference
        )
    })?;
    Ok(cache.to_path_buf())
}

/// The branch the project is on, or its commit when it is detached.
fn current_ref(repo: &Path) -> Result<String, String> {
    let branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == "HEAD" {
        git(repo, &["rev-parse", "HEAD"])
    } else {
        Ok(branch)
    }
}

fn has_changes(repo: &Path) -> Result<bool, String> {
    Ok(!git(repo, &["status", "--porcelain", "--untracked-files=no"])?.is_empty())
}

fn patch_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|error| format!("the add-on has no patch folder {}: {error}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "patch"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err("the add-on has no .patch files".into());
    }
    Ok(files)
}

/// Turns the add-on's patches on in `repo`, on a `vilauncher/<id>` branch. When they do not
/// apply, nothing is left changed and the error says where the port is described.
/// An add-on with no patches only records where the project was.
pub fn apply(repo: &Path, addon: &Addon, source: &Path) -> Result<Applied, String> {
    if has_changes(repo)? {
        return Err(
            "The project has uncommitted changes. Save or discard them before turning on an add-on."
                .into(),
        );
    }
    let original = current_ref(repo)?;
    let Some(patches) = &addon.patches else {
        return Ok(Applied {
            original,
            ..Applied::default()
        });
    };
    let files = patch_files(&source.join(&patches.dir))?;
    let branch = format!("vilauncher/{}", addon.id);
    if git(repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok() {
        // Our branch from an earlier turn-on: keep what is on it (the player may have
        // committed there), as long as it still has the add-on's commits on the original.
        let ahead = git(repo, &["rev-list", "--count", &format!("{original}..{branch}")])?;
        if ahead == "0" {
            return Err(format!(
                "The branch {branch} exists without the add-on's commits. Delete it, then turn the add-on on again."
            ));
        }
        git(repo, &["checkout", "--quiet", &branch])?;
        return Ok(Applied {
            original,
            branch,
            ..Applied::default()
        });
    }
    git(repo, &["checkout", "--quiet", "-b", &branch])?;
    let mut c = tools::git();
    c.current_dir(repo).args([
        "-c",
        "user.name=VILauncher",
        "-c",
        "user.email=vilauncher@localhost",
        "am",
        "--3way",
        "--quiet",
    ]);
    c.args(&files);
    match git_output(c) {
        Ok(_) => Ok(Applied {
            original,
            branch,
            ..Applied::default()
        }),
        Err(error) => {
            let _ = git(repo, &["am", "--abort"]);
            let _ = git(repo, &["checkout", "--quiet", &original]);
            let _ = git(repo, &["branch", "-D", &branch]);
            let guide = addon
                .guide
                .as_deref()
                .map(|guide| format!(" To port it by hand, read {guide} in {}.", addon.source.repo))
                .unwrap_or_default();
            Err(format!(
                "The add-on's patches do not apply to this project, so nothing was changed ({error}).{guide}"
            ))
        }
    }
}

/// Takes the add-on's patches off `repo`: back to the branch the project was on. The
/// add-on's branch is kept, so turning it on again needs no new patch step.
pub fn remove(repo: &Path, applied: &Applied) -> Result<(), String> {
    if applied.branch.is_empty() {
        return Ok(());
    }
    if has_changes(repo)? {
        return Err(
            "The project has uncommitted changes on the add-on's branch. Save or discard them before turning the add-on off."
                .into(),
        );
    }
    if current_ref(repo)? != applied.branch {
        return Err(format!(
            "The project is not on {} now, so the add-on stays on. Switch the project back to {} to turn it off.",
            applied.branch, applied.branch
        ));
    }
    git(repo, &["checkout", "--quiet", &applied.original]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vl-addon-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let mut c = Command::new("git");
        c.current_dir(dir).args(["-c", "user.name=t", "-c", "user.email=t@t"]).args(args);
        git_output(c).unwrap()
    }

    fn manifest(patch_dir: Option<&str>, reference: &str, switches: &[(&str, &str)]) -> String {
        let switches: BTreeMap<_, _> = switches
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let addon = Addon {
            id: "test-addon".into(),
            name: "Test add-on".into(),
            summary: "For the tests.".into(),
            source: Source {
                repo: "https://example.invalid/source.git".into(),
                reference: reference.into(),
            },
            patches: patch_dir.map(|dir| Patches {
                dir: dir.into(),
                base: None,
            }),
            switches,
            args: vec!["--fast".into()],
            guide: Some("PORT.md".into()),
            warning: "Changes the code.".into(),
        };
        serde_json::to_string(&addon).unwrap()
    }

    #[test]
    fn shipped_manifests_are_valid() {
        let (addons, problems) = catalog(Path::new("does-not-exist"));
        assert!(problems.is_empty(), "{problems:?}");
        let stack = addons.iter().find(|a| a.id == "memory-stack").unwrap();
        assert_eq!(stack.source.reference.len(), 40);
        assert_eq!(stack.patches.as_ref().unwrap().dir, "patches");
        assert_eq!(stack.switches["VICTORY_SLOT_EVICT_PERCENT"], "60");
    }

    #[test]
    fn a_manifest_is_checked_before_use() {
        let good = manifest(Some("patches"), "85c86c1", &[("VICTORY_X", "1")]);
        assert!(parse(&good).is_ok());
        let bad_id = good.replace("test-addon", "Bad Id");
        assert!(parse(&bad_id).is_err());
        assert!(parse(&manifest(Some("../escape"), "85c86c1", &[])).is_err());
        assert!(parse(&manifest(None, "not-a-commit!", &[])).is_err());
        assert!(parse(&manifest(None, "85c86c1", &[("PATH", "x")])).is_err());
        let http = good.replace("https://", "http://");
        assert!(parse(&http).is_err());
    }

    #[test]
    fn restoring_switches_gives_back_the_players_own_values() {
        let prior: BTreeMap<String, Option<String>> = [
            ("VICTORY_A".to_string(), Some("40".to_string())),
            ("VICTORY_B".to_string(), None),
        ]
        .into();
        let on: BTreeMap<String, String> = [
            ("VICTORY_A".to_string(), "60".to_string()),
            ("VICTORY_B".to_string(), "1".to_string()),
        ]
        .into();
        let applied = with_switches("VICTORY_GLOBAL=1\nVICTORY_A=40", &on, true);
        assert_eq!(switch_value(&applied, "VICTORY_A").as_deref(), Some("60"));
        assert_eq!(restore_switches(&applied, &prior), "VICTORY_GLOBAL=1\nVICTORY_A=40");
    }

    #[test]
    fn switches_and_args_turn_on_and_off_without_touching_other_lines() {
        let switches: BTreeMap<_, _> = [("VICTORY_A".to_string(), "1".to_string())].into();
        let on = with_switches("VICTORY_GLOBAL=1\nVICTORY_A=0", &switches, true);
        assert_eq!(on, "VICTORY_GLOBAL=1\nVICTORY_A=1");
        let off = with_switches(&on, &switches, false);
        assert_eq!(off, "VICTORY_GLOBAL=1");
        let args = vec!["--fast".to_string()];
        assert_eq!(with_args("--strict", &args, true), "--strict --fast");
        assert_eq!(with_args("--strict --fast", &args, true), "--strict --fast");
        assert_eq!(with_args("--strict --fast", &args, false), "--strict");
    }

    #[test]
    fn an_add_on_is_applied_and_removed_from_a_real_repo() {
        let root = temp("apply");
        // The add-on's source: a base commit, then one commit that becomes the patch.
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        run_git(&source, &["init", "-q", "-b", "main"]);
        std::fs::write(source.join("engine.txt"), "one\n").unwrap();
        run_git(&source, &["add", "."]);
        run_git(&source, &["commit", "-q", "-m", "base"]);
        std::fs::write(source.join("engine.txt"), "one\ntwo\n").unwrap();
        run_git(&source, &["commit", "-qam", "Add a line"]);
        let patch_dir = source.join("patches");
        std::fs::create_dir_all(&patch_dir).unwrap();
        run_git(&source, &["format-patch", "-q", "-1", "HEAD", "-o", patch_dir.to_str().unwrap()]);
        run_git(&source, &["add", "patches"]);
        run_git(&source, &["commit", "-q", "-m", "Patches for the add-on"]);
        let pinned = run_git(&source, &["rev-parse", "HEAD"]);
        // The project: a clone of the base, on main.
        let project = root.join("project");
        std::fs::create_dir_all(&project).unwrap();
        run_git(&project, &["init", "-q", "-b", "main"]);
        std::fs::write(project.join("engine.txt"), "one\n").unwrap();
        run_git(&project, &["add", "."]);
        run_git(&project, &["commit", "-q", "-m", "base"]);

        let mut addon = parse(&manifest(Some("patches"), &pinned, &[("VICTORY_A", "1")])).unwrap();
        addon.source.repo = source.to_string_lossy().into_owned();
        let source_cache = fetch_source(&addon, &root.join("cache")).unwrap();

        let applied = apply(&project, &addon, &source_cache).unwrap();
        assert_eq!(applied.original, "main");
        assert_eq!(applied.branch, "vilauncher/test-addon");
        assert_eq!(run_git(&project, &["rev-parse", "--abbrev-ref", "HEAD"]), "vilauncher/test-addon");
        // Git on Windows may write CRLF line endings; compare the text, not the bytes.
        let text = |path: PathBuf| std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        assert_eq!(text(project.join("engine.txt")), "one\ntwo\n");

        remove(&project, &applied).unwrap();
        assert_eq!(run_git(&project, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert_eq!(text(project.join("engine.txt")), "one\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn turning_an_add_on_on_again_keeps_the_players_commits_on_its_branch() {
        let root = temp("reuse");
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        run_git(&source, &["init", "-q", "-b", "main"]);
        std::fs::write(source.join("engine.txt"), "one
").unwrap();
        run_git(&source, &["add", "."]);
        run_git(&source, &["commit", "-q", "-m", "base"]);
        std::fs::write(source.join("engine.txt"), "one
two
").unwrap();
        run_git(&source, &["commit", "-qam", "Add a line"]);
        let patch_dir = source.join("patches");
        std::fs::create_dir_all(&patch_dir).unwrap();
        run_git(&source, &["format-patch", "-q", "-1", "HEAD", "-o", patch_dir.to_str().unwrap()]);
        run_git(&source, &["add", "patches"]);
        run_git(&source, &["commit", "-q", "-m", "Patches for the add-on"]);
        let pinned = run_git(&source, &["rev-parse", "HEAD"]);
        let project = root.join("project");
        std::fs::create_dir_all(&project).unwrap();
        run_git(&project, &["init", "-q", "-b", "main"]);
        std::fs::write(project.join("engine.txt"), "one
").unwrap();
        run_git(&project, &["add", "."]);
        run_git(&project, &["commit", "-q", "-m", "base"]);

        let mut addon = parse(&manifest(Some("patches"), &pinned, &[])).unwrap();
        addon.source.repo = source.to_string_lossy().into_owned();
        let source_cache = fetch_source(&addon, &root.join("cache")).unwrap();

        let first = apply(&project, &addon, &source_cache).unwrap();
        // The player works on the add-on's branch and commits there.
        std::fs::write(project.join("mine.txt"), "mine
").unwrap();
        run_git(&project, &["add", "mine.txt"]);
        run_git(&project, &["commit", "-q", "-m", "Player work"]);
        remove(&project, &first).unwrap();
        assert_eq!(run_git(&project, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");

        // Turning it on again reuses the branch: the player's commit is still there.
        let again = apply(&project, &addon, &source_cache).unwrap();
        assert_eq!(again.branch, "vilauncher/test-addon");
        assert!(project.join("mine.txt").is_file());
        assert_eq!(
            run_git(&project, &["log", "-1", "--format=%s"]),
            "Player work"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn patches_that_do_not_apply_leave_the_project_as_it_was() {
        let root = temp("conflict");
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        run_git(&source, &["init", "-q", "-b", "main"]);
        std::fs::write(source.join("engine.txt"), "one\n").unwrap();
        run_git(&source, &["add", "."]);
        run_git(&source, &["commit", "-q", "-m", "base"]);
        std::fs::write(source.join("engine.txt"), "one\ntwo\n").unwrap();
        run_git(&source, &["commit", "-qam", "Add a line"]);
        let patch_dir = source.join("patches");
        std::fs::create_dir_all(&patch_dir).unwrap();
        run_git(&source, &["format-patch", "-q", "-1", "HEAD", "-o", patch_dir.to_str().unwrap()]);
        run_git(&source, &["add", "patches"]);
        run_git(&source, &["commit", "-q", "-m", "Patches for the add-on"]);
        let pinned = run_git(&source, &["rev-parse", "HEAD"]);
        // The project diverged on the same line, so the patch cannot apply.
        let project = root.join("project");
        std::fs::create_dir_all(&project).unwrap();
        run_git(&project, &["init", "-q", "-b", "main"]);
        std::fs::write(project.join("engine.txt"), "one\nother\n").unwrap();
        run_git(&project, &["add", "."]);
        run_git(&project, &["commit", "-q", "-m", "diverged"]);

        let mut addon = parse(&manifest(Some("patches"), &pinned, &[])).unwrap();
        addon.source.repo = source.to_string_lossy().into_owned();
        addon.patches.as_mut().unwrap().base = None;
        let source_cache = fetch_source(&addon, &root.join("cache")).unwrap();

        let error = apply(&project, &addon, &source_cache).unwrap_err();
        assert!(error.contains("PORT.md"), "{error}");
        assert_eq!(run_git(&project, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert!(run_git(&project, &["branch", "--list", "vilauncher/*"]).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
