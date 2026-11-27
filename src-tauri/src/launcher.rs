//! The launcher itself, without a window: the project list, installs,
//! builds, Play, saves, the settings and the background job. The window
//! (src/main.rs and the web UI in ../src) asks it for a snapshot and sends
//! it actions (src/commands.rs); it says when something changed through
//! [`Ctx`].
//!
//! No game files are included or downloaded: the player supplies their own.

use crate::notify::Ctx;
use crate::{brand, build_activity, build_zip, fullscreen, rust_project, saves, script_preset, tools, updater};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod actions;
mod presets;
mod reported;
mod view;

pub(crate) use actions::{Action, Outcome};
pub(crate) use view::{LogView, Snapshot};

pub(crate) const EXE_NAME: &str = "game_scarlett_master.exe";
/// What a game folder must hold: the executable, its config and the main archives.
const REQUIRED: &[&str] = &[
    EXE_NAME,
    "MicrosoftGame.Config",
    "common.rpf",
    "data.rpf",
    "levels.rpf",
    "script.rpf",
    "shaders.rpf",
    "textures.rpf",
];

/// A project the player can pick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Project {
    name: String,
    /// `owner/name` on GitHub.
    repo: String,
    branch: Option<String>,
    gpu: Vec<String>,
    fps: Option<f64>,
    render: Option<f64>,
    verified: bool,
    summary: String,
    custom: bool,
    /// A build already on this PC (not downloaded or built by the launcher).
    #[serde(default)]
    local_exe: Option<String>,
    #[serde(default)]
    archive_exe: Option<PathBuf>,
    /// What works, from the tracker.
    #[serde(default)]
    highlights: Vec<String>,
    /// What still stops it, from the tracker.
    #[serde(default)]
    blockers: Vec<String>,
}


impl Project {
    fn is_git(&self) -> bool {
        self.local_exe.is_none() && self.archive_exe.is_none()
    }

    fn archive_program(&self) -> Option<PathBuf> {
        self.archive_exe.as_ref().map(|exe| repo_dir(&self.repo).join(exe))
    }

    fn runs_standalone(&self) -> bool {
        self.archive_exe.as_deref().is_some_and(|exe| !build_zip::is_victory(exe))
            || rust_install(&self.repo).is_some()
    }

    fn web_url(&self) -> String {
        if self.repo.contains("://") { self.repo.clone() } else { format!("https://github.com/{}", self.repo) }
    }

    fn clone_url(&self) -> String {
        if self.repo.contains("://") { self.repo.clone() } else { format!("https://github.com/{}.git", self.repo) }
    }
}

/// What is kept between runs.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Saved {
    game_dir: String,
    /// "8 GB graphics card fix": smaller textures and a grey stand-in for missing ones.
    fix_8gb: bool,
    /// "Brighter shadows": a constant ambient light where ray tracing does not run.
    bright_shadows: bool,
    /// Whether the defaults above were set once.
    defaults_set: bool,
    /// Play in borderless fullscreen (off: a normal window). Off by default.
    fullscreen: bool,
    /// Where projects are downloaded and built (empty: the default folder).
    install_dir: String,
    /// Updates put off with "Later": (repo, the new version's commit).
    later: Vec<(String, String)>,
    selected: Option<String>,
    custom: Vec<Project>,
    series_s: bool,
    render_scale: f32,
    extra_env: String,
    gpu_filter: usize,
    /// Translate the shaders into the cache (`victory translate --cache`) before each Play.
    warm_before_play: bool,
    /// With the warm-up, also build the GPU pipelines (`--pipelines`; can take hours).
    warm_pipelines: bool,
    /// Extra arguments for `victory run`; the game's own go after `--`.
    launch_args: String,
    project_launch: std::collections::BTreeMap<String, ProjectLaunch>,
    launch_presets: Vec<LaunchPreset>,
    /// The version of the legal notice the player agreed to (0: not yet).
    notice_accepted: u32,
}

/// The legal notice's version: raised when its text changes in a way that needs a new yes.
pub(crate) const NOTICE_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct LaunchPreset {
    name: String,
    args: String,
    switches: Option<String>,
    #[serde(default)]
    script_options: Option<script_preset::Options>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    projects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repo_script: Option<String>,
}

impl LaunchPreset {
    fn apply(&self, launch: &mut ProjectLaunch) {
        launch.args = Some(self.args.clone());
        if let Some(switches) = &self.switches {
            launch.switches = Some(switches.clone());
            launch.script_options = self.script_options;
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProjectLaunch {
    args: Option<String>,
    switches: Option<String>,
    script_options: Option<script_preset::Options>,
    selected_preset: Option<String>,
    preset_choice_confirmed: bool,
    /// Records a custom script accepted by an earlier build.
    confirmed_repo_script: Option<String>,
    graphics: Option<GraphicsSettings>,
    notes: String,
}

impl ProjectLaunch {
    fn has_confirmed_preset(&self) -> bool {
        self.preset_choice_confirmed || self.selected_preset.is_some() || self.confirmed_repo_script.is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct GraphicsSettings {
    series_s: bool,
    render_scale: f32,
    fix_8gb: bool,
    bright_shadows: bool,
    fullscreen: bool,
    #[serde(default)]
    output: OutputResolution,
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) enum OutputResolution {
    #[default]
    Default,
    P720,
    P936,
    P1080,
    P1440,
    P2160,
}

impl OutputResolution {
    fn dimensions(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::P720 => Some("1280x720"),
            Self::P936 => Some("1664x936"),
            Self::P1080 => Some("1920x1080"),
            Self::P1440 => Some("2560x1440"),
            Self::P2160 => Some("3840x2160"),
        }
    }
}

impl Saved {
    fn default_graphics(&self) -> GraphicsSettings {
        GraphicsSettings {
            series_s: self.series_s,
            render_scale: self.render_scale,
            fix_8gb: self.fix_8gb,
            bright_shadows: self.bright_shadows,
            fullscreen: self.fullscreen,
            output: OutputResolution::Default,
        }
    }

    fn graphics_for(&self, repo: &str) -> GraphicsSettings {
        self.project_launch.get(&project_key(repo))
            .and_then(|project| project.graphics)
            .unwrap_or_else(|| self.default_graphics())
    }

    fn launch_overrides(&self, repo: &str) -> (Vec<String>, String) {
        let graphics = self.graphics_for(repo);
        let project = self.project_launch.get(&project_key(repo));
        let args = project.and_then(|p| p.args.as_deref()).unwrap_or(&self.launch_args);
        let switches = project.and_then(|p| p.switches.as_deref());
        let mut extra = switches.map_or_else(|| self.extra_env.clone(), |_| String::new());
        if graphics.fix_8gb {
            extra.push_str("\nVICTORY_TEXTURE_DROP_MIPS=2\nVICTORY_NULL_COLOR=grey");
        }
        if graphics.bright_shadows {
            extra.push_str("\nVICTORY_AMBIENT=0.6");
        }
        if let Some(switches) = switches {
            extra.push('\n');
            extra.push_str(switches);
        }
        (split_args(args), extra)
    }
}

#[cfg(test)]
#[test]
fn project_launch_overrides_are_independent_and_can_be_empty() {
    let mut saved = Saved {
        launch_args: "--strict".into(),
        extra_env: "VICTORY_GLOBAL=1".into(),
        ..Saved::default()
    };
    saved.project_launch.insert("owner/test".into(), ProjectLaunch {
        args: Some(String::new()),
        switches: None,
        ..ProjectLaunch::default()
    });
    assert_eq!(saved.launch_overrides("Owner/Test"), (Vec::new(), "VICTORY_GLOBAL=1".into()));
    saved.project_launch.get_mut("owner/test").unwrap().switches = Some("VICTORY_LOCAL=1".into());
    assert_eq!(saved.launch_overrides("owner/test"), (Vec::new(), "\nVICTORY_LOCAL=1".into()));
    assert_eq!(saved.launch_overrides("owner/other"), (vec!["--strict".into()], "VICTORY_GLOBAL=1".into()));
    let mut graphics = saved.default_graphics();
    graphics.series_s = true;
    graphics.render_scale = 0.5;
    graphics.fix_8gb = true;
    graphics.fullscreen = true;
    saved.project_launch.get_mut("owner/test").unwrap().graphics = Some(graphics);
    assert!(saved.graphics_for("Owner/Test").fullscreen);
    assert!(!saved.graphics_for("owner/other").fullscreen);
    assert!(saved.launch_overrides("owner/test").1.contains("VICTORY_TEXTURE_DROP_MIPS=2"));
    assert!(!saved.launch_overrides("owner/other").1.contains("VICTORY_TEXTURE_DROP_MIPS"));
}

/// Shared state of the background job (download, build, play).
#[derive(Default)]
pub(crate) struct Job {
    running: bool,
    /// The project the job works on.
    repo: Option<String>,
    stage: String,
    log: Vec<String>,
    omitted_log_lines: usize,
    imported_project: Option<Project>,
    removed_archive: Option<String>,
    import_error: Option<String>,
    failed: bool,
    /// The process running now, while the player may stop it (a warm-up).
    pid: Option<u32>,
    stoppable: bool,
    /// The player pressed Stop.
    stopped: bool,
}

/// How a project is started, read from the settings when its job starts.
/// The warm-up and Play share it, so the warm-up fills the cache Play reads.
#[derive(Clone)]
struct Launch {
    game_dir: PathBuf,
    series_s: bool,
    scale: f32,
    output: OutputResolution,
    /// `VICTORY_*` lines: the player's switches and the graphics fixes.
    extra: String,
    use_script_defaults: bool,
    fullscreen: bool,
    warm_first: bool,
    pipelines: bool,
    args: Vec<String>,
    program_args: Vec<String>,
}

/// The game files `victory translate --cache` reads shaders from (those that exist are passed).
const WARM_FILES: &[&str] = &["shaders.rpf", EXE_NAME, "XboxBvhBuilder.dll", "amd_fidelityfx_upscaler_xbox.dll"];

/// Launch arguments as the player typed them: split at spaces, "double quotes" keeping one together.
fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted, mut any) = (Vec::new(), String::new(), false, false);
    for ch in s.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split_args;

    #[test]
    fn launch_arguments_split_at_spaces_and_keep_quotes_together() {
        assert_eq!(split_args(""), Vec::<String>::new());
        assert_eq!(split_args("  --strict   "), ["--strict"]);
        assert_eq!(split_args("--strict -- -windowed"), ["--strict", "--", "-windowed"]);
        assert_eq!(split_args(r#"-- -name "two words" """#), ["--", "-name", "two words", ""]);
    }
}

pub(crate) struct App {
    pub(crate) saved: Saved,
    pub(crate) updater: updater::Updater,
    projects: Arc<Mutex<Option<Result<Vec<Project>, String>>>>,
    pub(crate) job: Arc<Mutex<Job>>,
    files: Vec<(&'static str, bool)>,
    files_for: String,
    /// A new install folder waiting for "move the projects there?" (from, to).
    pub(crate) pending_move: Option<(PathBuf, PathBuf)>,
    /// The folder the files were checked in was empty.
    empty: bool,
    /// The drive search: `None` idle, `Some(None)` searching, `Some(Some(r))` done.
    finding: Arc<Mutex<Option<Option<Result<String, String>>>>>,
    /// What is known about each project's install, by repo.
    repos: Arc<Mutex<std::collections::HashMap<String, RepoState>>>,
    /// When every installed project was last checked for updates.
    last_auto_check: Option<Instant>,
    started: Instant,
    /// Why the last drive search found no game.
    pub(crate) find_error: Option<String>,
    /// Says that something changed, so the window asks for a new snapshot.
    pub(crate) ctx: Ctx,
    /// Which projects are on disk, and when that was looked at.
    installed_cache: Mutex<std::collections::HashMap<String, (Instant, bool)>>,
    /// The player's PC, once it was looked at (in the background at start).
    pub(crate) system: Arc<Mutex<Option<crate::system::SystemInfo>>>,
}

/// How often installed projects are checked for updates: every 15 minutes.
const AUTO_CHECK_SECS: f64 = 15.0 * 60.0;

/// A project's install on this PC and the newest version online.
#[derive(Clone, Default)]
pub(crate) struct RepoState {
    checking: bool,
    /// Built and ready to play.
    installed: bool,
    /// The commit the installed build was made from.
    built: Option<String>,
    /// The newest commit online (`None` while unknown or unreachable).
    remote: Option<String>,
    activity: Option<build_activity::BuildActivity>,
}

/// What the background job does.
#[derive(Clone, PartialEq)]
pub(crate) enum Kind {
    Install,
    Play(saves::LaunchSave),
    Uninstall,
    /// Shaders into the cache ahead of time, without playing.
    Warm,
}

fn project_key(repo: &str) -> String {
    if repo.contains("://") { repo.to_owned() } else { repo.to_ascii_lowercase() }
}

fn slug(repo: &str) -> String {
    if repo.contains("://") {
        let hash = ring::digest::digest(&ring::digest::SHA256, repo.as_bytes());
        let hex: String = hash.as_ref()[..16].iter().map(|byte| format!("{byte:02x}")).collect();
        return format!("git-{hex}");
    }
    repo.replace('/', "_")
}

#[derive(Serialize, Deserialize)]
struct RustInstall {
    executable: Option<PathBuf>,
}

fn rust_install_file(repo: &str) -> PathBuf {
    target_dir(repo).join("mandyhub-rust.json")
}

fn rust_install(repo: &str) -> Option<RustInstall> {
    serde_json::from_slice(&std::fs::read(rust_install_file(repo)).ok()?).ok()
}

fn project_installed(repo: &str) -> bool {
    match rust_install(repo) {
        Some(install) => install.executable.is_none_or(|exe| target_dir(repo).join(exe).is_file()),
        None => built_exe(repo).is_file(),
    }
}
fn repo_dir(repo: &str) -> PathBuf {
    workspace().join("repos").join(slug(repo))
}
fn target_dir(repo: &str) -> PathBuf {
    workspace().join("target").join(slug(repo))
}
fn built_exe(repo: &str) -> PathBuf {
    let dir = repo_dir(repo);
    let workspace = cargo_workspace(&dir).unwrap_or_else(|_| dir.clone());
    let target = target_dir(repo);
    let executable = target.join(detect_build_profile(&workspace)).join("victory.exe");
    let previous = target.join("release").join("victory.exe");
    if workspace != dir && !executable.is_file() && previous.is_file() {
        return previous;
    }
    executable
}

/// Most projects build at the root. Some wrap a Cargo workspace one level down.
fn cargo_workspace(repo: &Path) -> Result<PathBuf, String> {
    if repo.join("Cargo.toml").is_file() {
        return Ok(repo.to_owned());
    }
    let entries = std::fs::read_dir(repo).map_err(|error| format!("Could not inspect {}: {error}", repo.display()))?;
    let mut nested = Vec::new();
    let mut victory = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not inspect {}: {error}", repo.display()))?;
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) && path.join("Cargo.toml").is_file() {
            if path.join("apps/victory/Cargo.toml").is_file() {
                victory.push(path.clone());
            }
            nested.push(path);
        }
    }
    let candidates = if victory.is_empty() { &nested } else { &victory };
    match candidates.as_slice() {
        [workspace] => Ok(workspace.clone()),
        [] => Err(format!("No Cargo.toml found at {} or one folder down.", repo.display())),
        _ => Err(format!("Multiple nested Cargo projects found in {}. Put a workspace manifest at the repository root.", repo.display())),
    }
}

#[cfg(test)]
#[test]
fn finds_a_unique_nested_victory_workspace_and_prefers_the_root() {
    let root = std::env::temp_dir().join(format!("vilauncher-workspace-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let nested = root.join("cool_fox");
    std::fs::create_dir_all(nested.join("apps/victory")).unwrap();
    std::fs::write(nested.join("Cargo.toml"), "[profile.diag]").unwrap();
    std::fs::write(nested.join("apps/victory/Cargo.toml"), "").unwrap();
    assert_eq!(cargo_workspace(&root).unwrap(), nested);
    assert_eq!(detect_build_profile(&nested), "diag");
    let other = root.join("other");
    std::fs::create_dir_all(other.join("apps/victory")).unwrap();
    std::fs::write(other.join("Cargo.toml"), "").unwrap();
    std::fs::write(other.join("apps/victory/Cargo.toml"), "").unwrap();
    assert!(cargo_workspace(&root).is_err());
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    assert_eq!(cargo_workspace(&root).unwrap(), root);
    std::fs::remove_dir_all(root).unwrap();
}

/// Checks a cloned repo's Cargo.toml for a custom build profile (e.g. `diag`).
fn detect_build_profile(repo_dir: &Path) -> &'static str {
    let cargo_toml = repo_dir.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&cargo_toml) {
        if text.contains("[profile.diag]") {
            return "diag";
        }
    }
    "release"
}

/// Reads VICTORY_* environment variables from a project's scripts/play.ps1.
fn detect_launch_env(repo_dir: &Path) -> Vec<(String, String)> {
    let script = repo_dir.join("scripts").join("play.ps1");
    let Ok(text) = std::fs::read_to_string(&script) else { return Vec::new() };
    let mut vars = Vec::new();
    // Parse PowerShell hashtable lines like: VICTORY_FOO = '1'; VICTORY_BAR = '2'
    // They appear inside $settings = @{ ... } and in conditional blocks.
    for line in text.lines() {
        let line = line.trim().trim_end_matches(';').trim();
        // Match lines containing VICTORY_ assignments with single-quoted values.
        for part in line.split(';') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=') {
                let k = k.trim().trim_start_matches("$settings.");
                let v = v.trim().trim_matches('\'').trim_matches('"');
                if k.starts_with("VICTORY_") && !k.contains(' ') {
                    vars.push((k.to_string(), v.to_string()));
                }
            }
        }
    }
    vars
}
/// The commit an installed build was made from, written after a build.
fn built_file(repo: &str) -> PathBuf {
    target_dir(repo).join("built-from.txt")
}

/// The newest commit of `p`'s branch online (`git ls-remote`, no download).
fn remote_head(p: &Project) -> Option<String> {
    let url = p.clone_url();
    let mut c = tools::git();
    c.args(["ls-remote", "--"]).arg(&url);
    match &p.branch {
        Some(b) => c.arg(format!("refs/heads/{b}")),
        None => c.arg("HEAD"),
    };
    hide_window(&mut c);
    let out = c.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().next()
        .filter(|hash| matches!(hash.len(), 40 | 64) && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_string)
}

fn repo_state(p: &Project) -> RepoState {
    if let Some(exe) = p.archive_program() {
        return RepoState {
            installed: exe.is_file(),
            activity: build_activity::BuildActivity::read(&target_dir(&p.repo)),
            ..RepoState::default()
        };
    }
    if let Some(exe) = &p.local_exe {
        return RepoState { installed: Path::new(exe).is_file(), built: Some("local".into()), ..RepoState::default() };
    }
    RepoState {
        checking: false,
        installed: project_installed(&p.repo),
        built: std::fs::read_to_string(built_file(&p.repo)).ok().map(|s| s.trim().to_string()),
        remote: remote_head(p),
        activity: build_activity::BuildActivity::read(&target_dir(&p.repo)),
    }
}

/// A folder from what the player typed or picked: quotes and spaces off, and
/// the folder of a file if a file was given (the game's exe, say).
fn normalize_dir(s: &str) -> PathBuf {
    let t = s.trim().trim_matches('"').trim();
    let p = PathBuf::from(t);
    if p.is_file() {
        p.parent().map(Path::to_path_buf).unwrap_or(p)
    } else {
        p
    }
}

/// The folder holding the game's exe: `dir` itself or a folder under it, at
/// most `depth` levels down.
fn find_game_in(dir: &Path, depth: u32) -> Option<PathBuf> {
    if dir.join(EXE_NAME).is_file() {
        return Some(dir.to_path_buf());
    }
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        // Not worth walking into.
        let skip = ["windows", "program files", "program files (x86)", "programdata", "appdata"];
        if name.starts_with('$') || name.starts_with('.') || skip.contains(&name.as_str()) {
            continue;
        }
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(found) = find_game_in(&e.path(), depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Every drive's folders, three levels down, for the game's exe.
fn search_drives() -> Result<String, String> {
    for letter in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if !root.is_dir() {
            continue;
        }
        if let Some(found) = find_game_in(&root, 3) {
            return Ok(found.display().to_string());
        }
    }
    Err("The game was not found on any drive. Choose its folder with Browse.".into())
}

/// Where the launcher keeps repos it downloads: `%LOCALAPPDATA%\VILauncher` unless moved.
fn workspace() -> PathBuf {
    INSTALL_DIR.lock().unwrap().clone().unwrap_or_else(default_workspace)
}

/// `%LOCALAPPDATA%\VILauncher`: where projects go unless the player chose
/// another folder (Settings: Install folder).
pub(crate) fn default_workspace() -> PathBuf {
    brand::local_base().join(brand::FOLDER)
}

/// The install folder the player chose, if any.
static INSTALL_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn set_install_dir(dir: &str) {
    let d = dir.trim();
    *INSTALL_DIR.lock().unwrap() = (!d.is_empty()).then(|| PathBuf::from(d));
}

/// The parts of an install folder that belong to projects.
const PROJECT_PARTS: &[&str] = &["repos", "target", "cache", "data", "checkpoints", "deleted-saves"];

/// Whether `dir` holds any installed project.
fn has_projects(dir: &Path) -> bool {
    std::fs::read_dir(dir.join("repos")).map(|mut d| d.next().is_some()).unwrap_or(false)
}

/// Moves a folder: a rename on the same drive, else a copy and a delete.
fn move_dir(from: &Path, to: &Path, log: &dyn Fn(String)) -> std::io::Result<()> {
    if !from.exists() {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !to.exists() && std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_dir(from, to, log)?;
    std::fs::remove_dir_all(from)
}

fn copy_dir(from: &Path, to: &Path, log: &dyn Fn(String)) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let target = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &target, log)?;
        } else {
            std::fs::copy(e.path(), &target)?;
        }
    }
    log(format!("copied {}", from.display()));
    Ok(())
}

/// The launcher had other names before (Mandyhub, VI Launcher). On the first
/// start, the newest of their settings folders is copied, not moved, so the
/// old launcher keeps working beside this one. Projects are not copied: see
/// `legacy_workspace`.
pub(crate) fn migrate_old_folders() {
    let base = brand::roaming_base();
    let new = base.join(brand::FOLDER);
    if new.exists() {
        return;
    }
    if let Some(old) = brand::LEGACY_FOLDERS.iter().map(|name| base.join(name)).find(|old| old.is_dir()) {
        if copy_dir(&old, &new, &|_| {}).is_err() {
            let _ = std::fs::remove_dir_all(&new);
        }
    }
}

/// An install folder of the launcher's earlier names that holds projects:
/// used in place of a new, empty one, so nothing has to be built again.
fn legacy_workspace() -> Option<PathBuf> {
    let base = brand::local_base();
    brand::LEGACY_FOLDERS.iter().map(|name| base.join(name)).find(|dir| has_projects(dir))
}

/// Words kept out of the source as text: the game's name in the ways other
/// sites write it, to replace or skip (`clean_text`, `names_the_game`), and
/// PortWatch's field name for scores. Each byte is XORed with 0x5A.
const NAMES: &[&[u8]] = &[&[29, 40, 59, 52, 62, 122, 14, 50, 63, 60, 46, 122, 27, 47, 46, 53, 122, 12, 19], &[29, 40, 59, 52, 62, 122, 14, 50, 63, 60, 46, 122, 27, 47, 46, 53, 122, 108], &[29, 14, 27, 122, 12, 19], &[29, 14, 27, 122, 108], &[29, 14, 27, 108], &[29, 14, 27, 12, 19], &[61, 46, 59, 108], &[61, 46, 59, 122, 108]];
const KEYS: &[&[u8]] = &[&[61, 46, 59, 108], &[61, 46, 59, 44, 51], &[61, 40, 59, 52, 62, 46, 50, 63, 60, 46, 59, 47, 46, 53]];
const FIELD: &[u8] = &[61, 46, 59];

fn hidden(words: &[&[u8]]) -> Vec<String> {
    words.iter().map(|w| w.iter().map(|b| (b ^ 0x5A) as char).collect()).collect()
}

/// Text from other sites without the game's name: "the game" in its place.
fn clean_text(s: &str) -> String {
    let mut out = s.to_string();
    for pat in hidden(NAMES) {
        let pat = pat.as_str();
        for (from, to) in [(format!("{pat}'s"), "the game's".to_string()), (pat.to_string(), "the game".to_string())] {
            out = out.replace(&from, &to);
        }
    }
    out.replace(" PC port of the game", " PC port").replace("the game PC", "the game on PC")
}

/// Whether a repo's own name carries the game's name (not shown).
fn names_the_game(s: &str) -> bool {
    let l = s.to_lowercase().replace(['-', '_', ' ', '.'], "");
    hidden(KEYS).iter().any(|k| l.contains(k.as_str()))
}

/// `owner/name` from a GitHub link or `owner/name` itself.
fn parse_git_repo(link: &str) -> Option<String> {
    let link = link.trim();
    let normalized;
    let link = if !link.contains("://") && link.split('/').next()?.contains('.') && !link.starts_with("github.com/") {
        normalized = format!("https://{link}");
        &normalized
    } else {
        link
    };
    if !link.contains("://") {
        return parse_repo(link);
    }
    let mut url = url::Url::parse(link).ok()?;
    if !matches!(url.scheme(), "https" | "http" | "ssh" | "git")
        || url.host_str().is_none()
        || (url.scheme() != "ssh" && !url.username().is_empty())
        || url.password().is_some() || url.query().is_some() || url.fragment().is_some()
    {
        return None;
    }
    let path = url.path().trim_end_matches('/').to_owned();
    if path.is_empty() {
        return None;
    }
    if url.host_str() == Some("github.com") && matches!(url.scheme(), "https" | "http") && url.port().is_none() {
        return parse_repo(url.as_str());
    }
    url.set_path(&path);
    Some(url.into())
}

#[cfg(test)]
#[test]
fn custom_git_links_preserve_non_github_paths_and_old_github_ids() {
    assert_eq!(parse_git_repo("https://github.com/Owner/Project.git").as_deref(), Some("Owner/Project"));
    assert_eq!(parse_git_repo("Owner/Project").as_deref(), Some("Owner/Project"));
    assert_eq!(
        parse_git_repo("gitlab.example.org/group/SubGroup/Project.git").as_deref(),
        Some("https://gitlab.example.org/group/SubGroup/Project.git")
    );
    assert_eq!(
        parse_git_repo("ssh://git@192.0.2.1:2222/group/project.git").as_deref(),
        Some("ssh://git@192.0.2.1:2222/group/project.git")
    );
    for invalid in ["file:///C:/", "ext::cmd/path", "https://user:password@git.example/repo", "https://git.example/"] {
        assert!(parse_git_repo(invalid).is_none(), "{invalid}");
    }
}

fn parse_repo(link: &str) -> Option<String> {
    let s = link.trim().trim_end_matches('/').trim_end_matches(".git");
    let s = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("github.com/"))
        .unwrap_or(s);
    let mut parts = s.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let ok = |p: &str| p.chars().any(|c| c.is_ascii_alphanumeric()) && p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    (ok(owner) && ok(name)).then(|| format!("{owner}/{name}"))
}

/// The public projects, best first: from VIPortLive, else from PortWatch.
fn fetch_projects() -> Result<Vec<Project>, String> {
    let mut errors = Vec::new();
    for (name, url) in brand::PROJECT_SOURCES {
        match fetch_list(name, url) {
            Ok(list) => return Ok(list),
            Err(e) => errors.push(e),
        }
    }
    Err(errors.join(" "))
}

/// A list of strings from the tracker, without the game's name and blanks.
fn texts(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|items| items.iter().filter_map(|x| x.as_str()).map(|s| clean_text(s.trim())).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

/// One source's list. VIPortLive's feed has the same shape as PortWatch's tracker.
fn fetch_list(name: &str, url: &str) -> Result<Vec<Project>, String> {
    let body: serde_json::Value = ureq::get(url)
        .set("User-Agent", brand::USER_AGENT)
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .map_err(|e| format!("{name} could not be reached: {e}."))?
        .into_json()
        .map_err(|e| format!("{name} sent something unreadable: {e}."))?;
    let efforts = body["efforts"].as_array().ok_or_else(|| format!("{name}'s list has no projects."))?;
    // PortWatch keeps each project's scores under a field named after the game.
    let field = hidden(&[FIELD]).remove(0);
    let mut out: Vec<Project> = efforts
        .iter()
        .filter_map(|e| {
            let repo = e["repo"].as_str()?.to_string();
            parse_repo(&repo)?;
            if names_the_game(&repo) || names_the_game(e["name"].as_str().unwrap_or("")) {
                return None;
            }
            if e["stats"]["missing"].as_bool() == Some(true) || e["stats"]["archived"].as_bool() == Some(true) {
                return None;
            }
            Some(Project {
                name: clean_text(e["name"].as_str().unwrap_or(&repo)),
                branch: e["branch"].as_str().map(str::to_string),
                gpu: e["gpu"].as_array().map(|g| g.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                fps: e[field.as_str()]["fps"].as_f64(),
                render: e[field.as_str()]["render"].as_f64(),
                verified: e["verification"].as_str() == Some("portwatch"),
                summary: clean_text(e["summary"].as_str().unwrap_or("")),
                highlights: texts(&e["highlights"]),
                blockers: texts(&e["blockers"]),
                custom: false,
                local_exe: None,
                archive_exe: None,
                repo,
            })
        })
        .collect();
    let score = |p: &Project| p.render.unwrap_or(0.0) * 10.0 + p.fps.unwrap_or(0.0) + if p.verified { 5.0 } else { 0.0 };
    out.sort_by(|a, b| score(b).total_cmp(&score(a)));
    Ok(out)
}

#[cfg(windows)]
pub(crate) fn hide_window(c: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    c.creation_flags(CREATE_NO_WINDOW);
}
#[cfg(not(windows))]
pub(crate) fn hide_window(_: &mut Command) {}

impl App {
    fn import_build_zip(&mut self, ctx: &Ctx, input: PathBuf) {
        let name = input.file_stem().and_then(|name| name.to_str()).unwrap_or("Imported build").to_owned();
        let job = self.job.clone();
        let ctx = ctx.clone();
        {
            let mut state = job.lock().unwrap();
            if state.running {
                return;
            }
            *state = Job { running: true, stage: "Extracting build ZIP".into(), ..Job::default() };
        }
        std::thread::spawn(move || {
            let result = (|| -> Result<Project, String> {
                use ring::rand::{SecureRandom, SystemRandom};
                let mut id = [0; 16];
                SystemRandom::new().fill(&mut id).map_err(|_| "Could not create a build ID.")?;
                let hex: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
                let repo = format!("archive://{hex}");
                std::fs::create_dir_all(workspace().join("repos")).map_err(|error| error.to_string())?;
                let folder = repo_dir(&repo);
                let executable = build_zip::extract(&input, &folder)?;
                let target = target_dir(&repo);
                if let Err(error) = std::fs::create_dir_all(&target)
                    .and_then(|_| std::fs::write(built_file(&repo), "zip"))
                    .and_then(|_| build_activity::BuildActivity::record_build(&target, false))
                {
                    job.lock().unwrap().log.push(format!("Could not record the import date: {error}"));
                }
                Ok(Project {
                    name, repo, archive_exe: Some(executable), custom: true,
                    branch: None, gpu: Vec::new(), fps: None, render: None, verified: false,
                    summary: String::new(), local_exe: None,
                    highlights: Vec::new(), blockers: Vec::new(),
                })
            })();
            let mut state = job.lock().unwrap();
            state.running = false;
            match result {
                Ok(project) => {
                    state.stage = "Build imported".into();
                    state.imported_project = Some(project);
                }
                Err(error) => {
                    state.stage = "Build import failed".into();
                    state.failed = true;
                    state.log.push(format!("ERROR: {error}"));
                    state.import_error = Some(error);
                }
            }
            ctx.request_repaint();
        });
    }

    pub(crate) fn new(ctx: Ctx, mut saved: Saved) -> App {
        if saved.render_scale <= 0.0 {
            saved.render_scale = 0.5;
        }
        let mut app = App {
            saved,
            updater: updater::Updater::default(),
            projects: Arc::new(Mutex::new(None)),
            job: Arc::new(Mutex::new(Job::default())),
            files: Vec::new(),
            files_for: String::new(),
            pending_move: None,
            empty: false,
            finding: Arc::new(Mutex::new(None)),
            repos: Arc::new(Mutex::new(std::collections::HashMap::new())),
            last_auto_check: None,
            started: Instant::now(),
            find_error: None,
            ctx: ctx.clone(),
            installed_cache: Mutex::new(std::collections::HashMap::new()),
            system: Arc::new(Mutex::new(None)),
        };
        if !app.saved.defaults_set {
            app.saved.defaults_set = true;
            app.saved.fix_8gb = true;
            app.saved.bright_shadows = true;
            app.saved.series_s = true;
            app.saved.render_scale = 0.5;
        }
        if app.saved.install_dir.trim().is_empty() && !has_projects(&default_workspace()) {
            if let Some(old) = legacy_workspace() {
                app.saved.install_dir = old.display().to_string();
            }
        }
        set_install_dir(&app.saved.install_dir);
        app.updater.check(&ctx);
        app.refresh_projects(&ctx);
        let (system, system_ctx) = (app.system.clone(), ctx.clone());
        std::thread::spawn(move || {
            let info = crate::system::detect();
            *system.lock().unwrap() = Some(info);
            system_ctx.request_repaint();
        });
        let job = app.job.clone();
        std::thread::spawn(move || {
            let _ = tools::detect();
            job.lock().unwrap().stage = String::new();
            ctx.request_repaint();
        });
        app
    }

    fn refresh_projects(&mut self, ctx: &Ctx) {
        *self.projects.lock().unwrap() = None;
        let (slot, ctx) = (self.projects.clone(), ctx.clone());
        std::thread::spawn(move || {
            let r = fetch_projects();
            *slot.lock().unwrap() = Some(r);
            ctx.request_repaint();
        });
    }

    fn check_files(&mut self) {
        if self.files_for == self.saved.game_dir && !self.files.is_empty() {
            return;
        }
        let mut dir = normalize_dir(&self.saved.game_dir);
        // The exe in a folder under the one given: that folder is the game's.
        if !self.saved.game_dir.trim().is_empty() && !dir.join(EXE_NAME).is_file() {
            if let Some(found) = find_game_in(&dir, 2) {
                dir = found;
            }
        }
        let text = dir.display().to_string();
        if !self.saved.game_dir.trim().is_empty() && text != self.saved.game_dir {
            self.saved.game_dir = text;
        }
        self.empty = !self.saved.game_dir.is_empty()
            && std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true);
        self.files = REQUIRED.iter().map(|f| (*f, !self.saved.game_dir.is_empty() && dir.join(f).is_file())).collect();
        self.files_for = self.saved.game_dir.clone();
    }

    fn files_ok(&self) -> bool {
        !self.files.is_empty() && self.files.iter().all(|f| f.1)
    }

    fn all_projects(&self) -> Vec<Project> {
        let mut v: Vec<Project> = self.saved.custom.clone();
        if let Some(Ok(list)) = &*self.projects.lock().unwrap() {
            v.extend(list.iter().cloned());
        }
        v
    }

    /// Looks again at `p`'s install and the newest version online.
    fn check_repo(&self, ctx: &Ctx, p: &Project) {
        {
            let mut m = self.repos.lock().unwrap();
            let e = m.entry(p.repo.clone()).or_default();
            if e.checking {
                return;
            }
            e.checking = true;
        }
        let (repos, ctx, p) = (self.repos.clone(), ctx.clone(), p.clone());
        std::thread::spawn(move || {
            let st = repo_state(&p);
            repos.lock().unwrap().insert(p.repo.clone(), st);
            ctx.request_repaint();
        });
    }

    fn state_of(&self, repo: &str) -> Option<RepoState> {
        self.repos.lock().unwrap().get(repo).cloned()
    }

    /// A newer version online than the one installed, and not put off.
    fn update_for(&self, p: &Project) -> Option<String> {
        if !p.is_git() {
            return None;
        }
        // Not while this project is being installed or updated.
        {
            let j = self.job.lock().unwrap();
            if j.running && j.repo.as_deref() == Some(p.repo.as_str()) {
                return None;
            }
        }
        let remote = self.state_of(&p.repo)?.remote?;
        // What is installed now, from disk: a check that started before the
        // last update finished must not bring the old version back.
        if !project_installed(&p.repo) {
            return None;
        }
        let built = std::fs::read_to_string(built_file(&p.repo)).ok().map(|s| s.trim().to_string());
        let newer = built.as_deref() != Some(remote.as_str());
        let later = self.saved.later.iter().any(|(r, c)| r == &p.repo && c == &remote);
        (newer && !later).then_some(remote)
    }

    fn start(&mut self, ctx: &Ctx, p: Project, kind: Kind) {
        let job = self.job.clone();
        {
            let mut j = job.lock().unwrap();
            if j.running {
                return;
            }
            *j = Job { running: true, stage: "Starting".into(), repo: Some(p.repo.clone()), ..Default::default() };
        }
        // An install over an installed project is an update.
        let updating = kind == Kind::Install && project_installed(&p.repo);
        let settings = self.saved.project_launch.get(&project_key(&p.repo));
        let (args, extra) = self.saved.launch_overrides(&p.repo);
        let graphics = self.saved.graphics_for(&p.repo);
        let launch = Launch {
            game_dir: PathBuf::from(&self.saved.game_dir),
            series_s: graphics.series_s,
            scale: graphics.render_scale,
            output: graphics.output,
            extra,
            use_script_defaults: settings.is_some_and(|settings| settings.switches.is_some() && settings.script_options.is_some()),
            fullscreen: graphics.fullscreen,
            warm_first: self.saved.warm_before_play,
            pipelines: self.saved.warm_pipelines,
            args,
            program_args: settings.and_then(|settings| settings.args.as_deref()).map(split_args).unwrap_or_default(),
        };
        let (ctx, repos) = (ctx.clone(), self.repos.clone());
        std::thread::spawn(move || {
            let r = match &kind {
                Kind::Install => install(&job, &ctx, &p, updating),
                Kind::Play(choice) => play(&job, &ctx, &p, &launch, choice),
                Kind::Uninstall => uninstall(&job, &ctx, &p),
                Kind::Warm => warm(&job, &ctx, &p, &launch),
            };
            {
                let mut j = job.lock().unwrap();
                j.running = false;
                j.pid = None;
                j.stoppable = false;
                match r {
                    _ if j.stopped => j.stage = "Stopped".into(),
                    Ok(()) => {
                        if kind == Kind::Uninstall && p.archive_exe.is_some() {
                            j.removed_archive = Some(p.repo.clone());
                        }
                        j.stage = match kind {
                            Kind::Install if updating => "Updated: ready to play".into(),
                            Kind::Install => "Installed: ready to play".into(),
                            Kind::Play(_) => if p.runs_standalone() { "Program closed".into() } else { "Game closed".into() },
                            Kind::Uninstall => "Uninstalled".into(),
                            Kind::Warm => "Shaders warmed up: ready to play".into(),
                        }
                    }
                    Err(e) => {
                        j.failed = true;
                        j.stage = match kind {
                            Kind::Install if updating => "Update failed: the old version is still installed (see the log)".into(),
                            Kind::Install => "Install failed (see the log)".into(),
                            Kind::Warm => "Warm-up failed (see the log)".into(),
                            _ => "Failed".into(),
                        };
                        j.log.push(format!("ERROR: {e}"));
                    }
                }
            }
            if matches!(kind, Kind::Install | Kind::Uninstall) {
                let st = repo_state(&p);
                repos.lock().unwrap().insert(p.repo.clone(), st);
            } else if matches!(kind, Kind::Play(_)) && p.local_exe.is_none() {
                if let Some(state) = repos.lock().unwrap().get_mut(&p.repo) {
                    state.activity = build_activity::BuildActivity::read(&target_dir(&p.repo));
                }
            }
            ctx.request_repaint();
        });
    }
}


fn stage(job: &Arc<Mutex<Job>>, ctx: &Ctx, s: &str) {
    let mut j = job.lock().unwrap();
    j.stage = s.to_string();
    j.log.push(format!("== {s}"));
    ctx.request_repaint();
}

/// Downloads `p` (or its newest version) and builds it, once; it stays
/// installed until the player updates it.
fn install(job: &Arc<Mutex<Job>>, ctx: &Ctx, p: &Project, updating: bool) -> Result<(), String> {
    let ws = workspace();
    std::fs::create_dir_all(ws.join("repos")).map_err(|e| format!("cannot make {}: {e}", ws.display()))?;
    let dir = repo_dir(&p.repo);
    let url = p.clone_url();
    if dir.join(".git").is_dir() {
        stage(job, ctx, "Downloading the new version");
        let refspec = p.branch.clone().unwrap_or_else(|| "HEAD".into());
        run(job, ctx, tools::git().args(["fetch", "--depth", "1", "origin", &refspec]).current_dir(&dir))?;
        run(job, ctx, tools::git().args(["reset", "--hard", "FETCH_HEAD"]).current_dir(&dir))?;
    } else {
        stage(job, ctx, "Downloading the project");
        let mut c = tools::git();
        c.args(["clone", "--depth", "1"]);
        if let Some(b) = &p.branch {
            c.args(["--branch", b]);
        }
        c.arg("--").arg(&url).arg(&dir);
        run(job, ctx, &mut c)?;
    }
    stage(job, ctx, "Building (the first time takes several minutes)");
    let target = target_dir(&p.repo);
    build_project(job, ctx, p, &dir, &target)?;
    let mut c = tools::git();
    c.args(["rev-parse", "HEAD"]).current_dir(&dir);
    hide_window(&mut c);
    if let Ok(out) = c.output() {
        if out.status.success() {
            let record = std::fs::write(built_file(&p.repo), String::from_utf8_lossy(&out.stdout).trim())
                .and_then(|_| build_activity::BuildActivity::record_build(&target, updating));
            if let Err(error) = record {
                job.lock().unwrap().log.push(format!("Could not record the update: {error}"));
            }
        }
    }
    Ok(())
}

fn build_project(job: &Arc<Mutex<Job>>, ctx: &Ctx, p: &Project, dir: &Path, target: &Path) -> Result<(), String> {
    let workspace = cargo_workspace(dir)?;
    let metadata = rust_project::metadata(&workspace)?;
    let victory = !p.custom || rust_project::is_victory(&workspace, &metadata);
    let profile = if victory { detect_build_profile(&workspace) } else { "release" };
    let has_lockfile = workspace.join("Cargo.lock").is_file();
    let mut cargo = tools::cargo();
    cargo.args(["build", "--profile", profile, "--target-dir"]);
    cargo.arg(target);
    if has_lockfile {
        cargo.arg("--locked");
    }
    cargo.current_dir(&workspace);
    if victory {
        cargo.args(["-p", "victory"]);
        run(job, ctx, &mut cargo)?;
        let executable = target.join(profile).join("victory.exe");
        if !executable.is_file() {
            return Err(format!("the build made no {}", executable.display()));
        }
        let marker = rust_install_file(&p.repo);
        if marker.exists() {
            std::fs::remove_file(marker).map_err(|error| error.to_string())?;
        }
    } else {
        cargo.args(["--workspace", "--message-format=json-render-diagnostics"]);
        let artifacts = Arc::new(Mutex::new(Vec::new()));
        let captured = artifacts.clone();
        run_with_output(job, ctx, &mut cargo, |_| {}, move |line| {
            let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else { return Some(line) };
            if message["reason"] == "compiler-artifact" && message["executable"].is_string() {
                captured.lock().unwrap().push(message);
                return None;
            }
            message["message"]["rendered"].as_str().map(str::to_owned)
        })?;
        let artifacts = artifacts.lock().unwrap();
        let executable = rust_project::select_executable(&metadata, &artifacts)
            .map(|exe| Path::new(exe).strip_prefix(target).map(Path::to_owned))
            .transpose().map_err(|_| "Cargo reported an executable outside the install folder")?;
        let marker = rust_install_file(&p.repo);
        let temporary = marker.with_extension("tmp");
        let record = serde_json::to_vec(&RustInstall { executable }).map_err(|error| error.to_string())?;
        std::fs::write(&temporary, record).and_then(|_| std::fs::rename(&temporary, &marker))
            .map_err(|error| format!("Could not record the installed Rust project: {error}"))?;
    }
    Ok(())
}

/// Deletes `p`'s download and build (and its shader cache); its saves stay.
fn uninstall(job: &Arc<Mutex<Job>>, ctx: &Ctx, p: &Project) -> Result<(), String> {
    if p.local_exe.is_some() {
        return Err("this build was not installed by VILauncher".into());
    }
    stage(job, ctx, "Uninstalling");
    let ws = workspace();
    for dir in [repo_dir(&p.repo), target_dir(&p.repo), ws.join("cache").join(slug(&p.repo))] {
        if dir.exists() {
            job.lock().unwrap().log.push(format!("removing {}", dir.display()));
            ctx.request_repaint();
            std::fs::remove_dir_all(&dir).map_err(|e| format!("could not remove {}: {e}", dir.display()))?;
        }
    }
    Ok(())
}

/// The victory.exe `p` runs with: its build, or a build already on this PC.
fn project_exe(p: &Project) -> Result<PathBuf, String> {
    let exe = p.archive_program().or_else(|| p.local_exe.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| built_exe(&p.repo));
    if exe.is_file() {
        Ok(exe)
    } else {
        Err("this project is not installed yet: press Install".into())
    }
}

/// The environment victory.exe runs with, the same for the warm-up and for
/// Play: the project's own env vars (from its scripts/play.ps1) first, then
/// the mode, its data and cache folders, and the player's switches.
fn victory_env(c: &mut Command, p: &Project, l: &Launch) {
    if let Some(program) = p.archive_program() {
        if let Some(directory) = program.parent() {
            c.current_dir(directory);
            if !l.use_script_defaults {
                for (key, value) in build_zip::launch_defaults(directory) {
                    c.env(key, value);
                }
            }
        }
        c.env("VICTORY_GAME_DIR", &l.game_dir);
    } else if l.use_script_defaults {
        c.env("VICTORY_GAME_DIR", &l.game_dir);
    }
    if !l.use_script_defaults {
        for (k, v) in detect_launch_env(&repo_dir(&p.repo)) {
            c.env(&k, &v);
        }
    }
    if l.series_s {
        c.env("VICTORY_OWN_UPSCALER", "1");
        c.env("VICTORY_RENDER_SCALE", format!("{:.2}", l.scale));
    }
    if let Some(size) = l.output.dimensions() {
        c.env("VICTORY_OUTPUT", size);
    }
    let ws = workspace();
    let data = ws.join("data").join(slug(&p.repo));
    let _ = std::fs::create_dir_all(&data);
    c.env("VICTORY_DATA_DIR", &data);
    c.env("VICTORY_CACHE_DIR", ws.join("cache").join(slug(&p.repo)));
    for line in l.extra.lines() {
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            if k.starts_with("VICTORY_") {
                c.env(k, v.trim());
            }
        }
    }
}

fn victory_help(exe: &Path, subcommand: &str) -> Option<String> {
    let mut command = Command::new(exe);
    command.args([subcommand, "--help"]);
    hide_window(&mut command);
    let output = command.output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Some forks open their pipeline bookkeeping files even before the first game run.
fn prepare_pipeline_cache(command: &Command) -> Result<(), String> {
    let Some(cache) = command.get_envs().find_map(|(key, value)| {
        (key == "VICTORY_CACHE_DIR").then_some(value).flatten().filter(|path| !path.is_empty())
    }) else {
        return Ok(());
    };
    let cache = Path::new(cache);
    let folder = command.get_current_dir().unwrap_or(Path::new(".")).join(cache).join("pipelines");
    std::fs::create_dir_all(&folder)
        .map_err(|error| format!("Could not create the pipeline cache folder {}: {error}", folder.display()))
}

#[cfg(test)]
#[test]
fn pipeline_warm_up_creates_the_configured_cache_on_first_use() {
    let root = std::env::temp_dir().join(format!("vilauncher-pipeline-cache-{}", std::process::id()));
    let mut command = Command::new("victory.exe");
    command.current_dir(&root).env("VICTORY_CACHE_DIR", "custom cache");
    prepare_pipeline_cache(&command).unwrap();
    prepare_pipeline_cache(&command).unwrap();
    assert!(root.join("custom cache").join("pipelines").is_dir());
    std::fs::remove_dir(root.join("custom cache").join("pipelines")).unwrap();
    std::fs::remove_dir(root.join("custom cache")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

/// Fills the same shader cache as Play, using the commands the installed fork exposes.
fn warm(job: &Arc<Mutex<Job>>, ctx: &Ctx, p: &Project, l: &Launch) -> Result<(), String> {
    let exe = project_exe(p)?;
    let files: Vec<PathBuf> = WARM_FILES.iter().map(|f| l.game_dir.join(f)).filter(|f| f.is_file()).collect();
    if files.is_empty() {
        return Err("the game files were not found: choose their folder in Settings".into());
    }
    let translate = victory_help(&exe, "translate");
    let supports_flag = |help: &str, flag: &str| help.split_whitespace().any(|word| word == flag);
    let translate_cache = translate.as_deref().filter(|help| supports_flag(help, "--cache"));
    let mut commands = Vec::new();
    let mut shaders = Command::new(&exe);
    if let Some(help) = translate_cache {
        shaders.args(["translate", "--cache"]);
        if l.pipelines && supports_flag(help, "--pipelines") {
            shaders.arg("--pipelines");
        }
    } else if victory_help(&exe, "shaders").is_some() {
        shaders.arg("shaders");
    } else {
        return Err("This build offers neither translate --cache nor shaders for shader warm-up.".into());
    }
    shaders.args(&files);
    commands.push(shaders);
    if l.pipelines && !translate_cache.is_some_and(|help| supports_flag(help, "--pipelines")) {
        if victory_help(&exe, "warm").is_some_and(|help| supports_flag(&help, "--used")) {
            let mut pipelines = Command::new(&exe);
            pipelines.args(["warm", "--used"]);
            commands.push(pipelines);
        } else {
            job.lock().unwrap().log.push("This build does not expose a supported pipeline warm-up. Warming shaders only.".into());
        }
    }
    stage(
        job,
        ctx,
        if l.pipelines {
            "Warming up shaders and pipelines (this can take hours)"
        } else {
            "Warming up shaders (the first time takes minutes)"
        },
    );
    for mut command in commands {
        victory_env(&mut command, p, l);
        if command.get_args().next().is_some_and(|arg| arg == "warm") {
            prepare_pipeline_cache(&command)?;
        }
        job.lock().unwrap().stoppable = true;
        let result = run_with(job, ctx, &mut command, |pid| job.lock().unwrap().pid = Some(pid));
        let stopped = {
            let mut j = job.lock().unwrap();
            j.stoppable = false;
            j.pid = None;
            j.stopped
        };
        if stopped {
            return Err("stopped".into());
        }
        result?;
    }
    Ok(())
}

/// Starts the installed build of `p`, warming its shaders first when the player asked for that.
fn play(job: &Arc<Mutex<Job>>, ctx: &Ctx, p: &Project, l: &Launch, choice: &saves::LaunchSave) -> Result<(), String> {
    if p.runs_standalone() {
        let (exe, dir) = if let Some(exe) = p.archive_program() {
            let dir = exe.parent().ok_or("The archive executable has no parent folder.")?.to_owned();
            (exe, dir)
        } else {
            let exe = rust_install(&p.repo).and_then(|install| install.executable)
                .ok_or("This Rust workspace has no runnable binary.")?;
            (target_dir(&p.repo).join(exe), cargo_workspace(&repo_dir(&p.repo))?)
        };
        let mut command = Command::new(exe);
        command.current_dir(dir).args(&l.program_args);
        stage(job, ctx, "Playing");
        return run_with(job, ctx, &mut command, |_| {
            if let Err(error) = build_activity::BuildActivity::record_run(&target_dir(&p.repo)) {
                job.lock().unwrap().log.push(format!("Could not record the launch: {error}"));
            }
        });
    }
    let exe = project_exe(p)?;
    if l.warm_first {
        if let Err(e) = warm(job, ctx, p, l) {
            if job.lock().unwrap().stopped {
                return Err("stopped".into());
            }
            job.lock().unwrap().log.push(format!("Warm-up failed: {e}. Starting the game anyway."));
        }
    }
    stage(job, ctx, "Playing");
    let mut c = Command::new(&exe);
    c.arg("run").arg(l.game_dir.join(EXE_NAME)).arg("--execute");
    let explicit_console = l.args.iter().take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg == "--console" || arg.starts_with("--console="));
    if !explicit_console && (l.series_s || l.output != OutputResolution::Default) {
        c.args(["--console", if l.series_s { "series-s" } else { "series-x" }]);
    }
    // The player's own: more `run` flags, and after `--` the game's.
    c.args(&l.args);
    victory_env(&mut c, p, l);
    let project_data = workspace().join("data").join(slug(&p.repo));
    let data = saves::prepare(&project_data, choice).map_err(|error| format!("Could not prepare saves: {error}"))?;
    if data != project_data {
        c.env("VICTORY_DATA_DIR", data);
    }
    let fullscreen = l.fullscreen;
    run_with(job, ctx, &mut c, |pid| {
        if p.local_exe.is_none() {
            if let Err(error) = build_activity::BuildActivity::record_run(&target_dir(&p.repo)) {
                job.lock().unwrap().log.push(format!("Could not record the launch: {error}"));
            }
        }
        if fullscreen {
            fullscreen::keep_borderless(pid);
        }
    })
}

/// Runs `c`, its output lines into the job's log.
fn run(job: &Arc<Mutex<Job>>, ctx: &Ctx, c: &mut Command) -> Result<(), String> {
    run_with(job, ctx, c, |_| {})
}

/// [`run`], telling `spawned` the process id once it started.
fn run_with(job: &Arc<Mutex<Job>>, ctx: &Ctx, c: &mut Command, spawned: impl FnOnce(u32)) -> Result<(), String> {
    run_with_output(job, ctx, c, spawned, Some)
}

fn run_with_output(
    job: &Arc<Mutex<Job>>,
    ctx: &Ctx,
    c: &mut Command,
    spawned: impl FnOnce(u32),
    filter: impl Fn(String) -> Option<String> + Send + Sync + 'static,
) -> Result<(), String> {
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    hide_window(c);
    let mut child = c.spawn().map_err(|e| format!("could not start {:?}: {e}", c.get_program()))?;
    spawned(child.id());
    let (out, err) = (child.stdout.take(), child.stderr.take());
    let filter = Arc::new(filter);
    let pipe = |r: Box<dyn std::io::Read + Send>, filter_stdout: bool| {
        let (job, ctx) = (job.clone(), ctx.clone());
        let filter = filter.clone();
        std::thread::spawn(move || {
            for bytes in BufReader::new(r).split(b'\n').map_while(Result::ok) {
                let line = String::from_utf8_lossy(bytes.strip_suffix(b"\r").unwrap_or(&bytes)).into_owned();
                let Some(line) = (if filter_stdout { filter(line) } else { Some(line) }) else { continue };
                let mut j = job.lock().unwrap();
                j.log.push(line);
                if j.log.len() > 4000 {
                    j.log.drain(..1000);
                    j.omitted_log_lines += 1000;
                }
                drop(j);
                ctx.request_repaint();
            }
        })
    };
    let a = out.map(|o| pipe(Box::new(o), true));
    let b = err.map(|e| pipe(Box::new(e), false));
    let status = child.wait().map_err(|e| e.to_string())?;
    for t in [a, b].into_iter().flatten() {
        let _ = t.join();
    }
    status.success().then_some(()).ok_or_else(|| format!("{:?} ended with {status}", c.get_program()))
}

impl App {
    /// Installs what is missing with winget, then looks again.
    fn start_tools(&mut self, ctx: &Ctx) {
        let job = self.job.clone();
        {
            let mut j = job.lock().unwrap();
            if j.running {
                return;
            }
            *j = Job { running: true, stage: "Installing tools (Windows will ask for permission)".into(), ..Default::default() };
        }
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let have = tools::detect();
            let result = if !tools::has_winget() {
                Err("winget (the Windows package manager) is not on this PC. Install them by hand: git-scm.com, rustup.rs and the Visual Studio Build Tools (Desktop development with C++).".to_string())
            } else {
                let mut r = Ok(());
                for (what, args) in tools::install_steps(&have) {
                    stage(&job, &ctx, &format!("Installing {what} (this can take a while)"));
                    let mut c = Command::new("winget");
                    c.args(&args);
                    if let Err(e) = run(&job, &ctx, &mut c) {
                        r = Err(format!("{what}: {e}"));
                        break;
                    }
                }
                r
            };
            let now = tools::detect();
            let mut j = job.lock().unwrap();
            j.running = false;
            match result {
                Ok(()) if now.ready() => j.stage = "Tools installed: ready to install projects".into(),
                Ok(()) => {
                    j.failed = true;
                    j.stage = format!("Still missing: {}. A restart of Windows can help.", now.missing().join(", "));
                }
                Err(e) => {
                    j.failed = true;
                    j.stage = "Tools not installed (see the log)".into();
                    j.log.push(format!("ERROR: {e}"));
                }
            }
            ctx.request_repaint();
        });
    }

    /// A new install folder: asks first if projects are installed in the old one.
    fn choose_install_dir(&mut self, new: PathBuf) {
        let old = workspace();
        if new == old {
            return;
        }
        if has_projects(&old) {
            self.pending_move = Some((old, new));
        } else {
            self.apply_install_dir(&new);
        }
    }

    fn apply_install_dir(&mut self, new: &Path) {
        self.saved.install_dir = if new == default_workspace() { String::new() } else { new.display().to_string() };
        set_install_dir(&self.saved.install_dir);
        // What is installed where is looked at again.
        self.repos.lock().unwrap().clear();
        self.last_auto_check = None;
    }

    /// Moves every installed project from `old` to `new`, then uses `new`.
    fn start_move(&mut self, ctx: &Ctx, old: PathBuf, new: PathBuf) {
        let job = self.job.clone();
        {
            let mut j = job.lock().unwrap();
            if j.running {
                return;
            }
            *j = Job { running: true, stage: "Moving installed projects".into(), ..Default::default() };
        }
        let (ctx, repos) = (ctx.clone(), self.repos.clone());
        std::thread::spawn(move || {
            let log = |line: String| {
                job.lock().unwrap().log.push(line);
                ctx.request_repaint();
            };
            let mut result = Ok(());
            for part in PROJECT_PARTS {
                log(format!("moving {}", old.join(part).display()));
                if let Err(e) = move_dir(&old.join(part), &new.join(part), &log) {
                    result = Err(format!("could not move {}: {e}", old.join(part).display()));
                    break;
                }
            }
            let mut j = job.lock().unwrap();
            j.running = false;
            match result {
                Ok(()) => j.stage = "Moved: projects are in the new folder".into(),
                Err(e) => {
                    j.failed = true;
                    j.stage = "Failed".into();
                    j.log.push(format!("ERROR: {e}"));
                }
            }
            repos.lock().unwrap().clear();
            ctx.request_repaint();
        });
    }

    /// At start and every 15 minutes: every installed project is checked for
    /// a newer version online (no download; `git ls-remote`).
    pub(crate) fn auto_check(&mut self, ctx: &Ctx) {
        let listed = self.projects.lock().unwrap().is_some();
        let due = match self.last_auto_check {
            None => listed || self.started.elapsed().as_secs_f64() > 10.0,
            Some(at) => at.elapsed().as_secs_f64() > AUTO_CHECK_SECS,
        };
        if !due {
            return;
        }
        self.last_auto_check = Some(Instant::now());
        for p in self.all_projects() {
            if p.is_git() && project_installed(&p.repo) {
                self.check_repo(ctx, &p);
            }
        }
    }
}
