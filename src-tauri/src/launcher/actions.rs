//! Everything the window can ask the launcher to do. Each action answers
//! with a value (often nothing) or a message for the player when it can't.

use super::presets::{self, PlayPresetPicker};
use super::*;
use crate::save_zip;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Action {
    Select { repo: Option<String> },
    RefreshProjects,
    SetGpuFilter { value: usize },
    CheckRepo { repo: String },
    /// Install, update or reinstall.
    Install { repo: String },
    Later { repo: String, remote: String },
    /// The first step of Play: answers what to ask before the game starts.
    Play { repo: String },
    /// A launch preset picked (or none: the current settings), then on to the saves.
    ConfirmPreset { repo: String, index: Option<usize> },
    StartWithSave { repo: String, choice: SaveChoice },
    Warm { repo: String },
    Uninstall { repo: String },
    Stop,
    RemoveCustom { repo: String },
    AddRepoLink { link: String },
    ImportBuildZip { path: String },
    DismissImportError,
    InstallTools,
    SetGameDir { path: String },
    FindGameFiles,
    SetSettings { patch: SettingsPatch },
    SetDefaultGraphics { graphics: GraphicsSettings },
    SetProjectGraphics { repo: String, graphics: Option<GraphicsSettings> },
    SetProjectArgs { repo: String, value: Option<String> },
    SetProjectSwitches { repo: String, value: Option<String> },
    SetScriptOption { repo: String, option: String, enabled: bool },
    SetProjectNotes { repo: String, notes: String },
    SavePreset { repo: String, name: String, include_switches: bool },
    ApplyPreset { repo: String, name: Option<String> },
    DeletePreset { repo: String, name: String },
    ImportPresetFile { repo: String, path: String },
    ImportPresetText { repo: String, text: String },
    ExportPreset { name: String, path: String },
    PresetText { name: String },
    DetectedPreset { repo: String },
    ChooseInstallDir { path: String },
    UseDefaultInstallDir,
    /// The answer to "move the installed projects?": "move", "keep" or "cancel".
    ConfirmMove { answer: String },
    Open { what: String, repo: Option<String> },
    ExportLog { path: String },
    CheckUpdate,
    DownloadUpdate,
    RestartToUpdate,
    SavesList { repo: String },
    Checkpoint { name: String, source: String },
    ImportSaveZip { path: String },
    ExportSave { source: String, path: String },
    DeleteSave { path: String },
    /// The player agreed to the legal notice.
    AcceptNotice,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SaveChoice {
    Continue,
    Fresh,
    Copy { path: String },
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct SettingsPatch {
    warm_before_play: Option<bool>,
    warm_pipelines: Option<bool>,
    launch_args: Option<String>,
    extra_env: Option<String>,
}

/// What an action may ask of the window itself.
pub(crate) enum Outcome {
    Value(Value),
    /// The update is in place: close so the new version runs.
    Exit,
}

impl App {
    pub(crate) fn act(&mut self, action: Action) -> Result<Outcome, String> {
        let ctx = self.ctx.clone();
        let value = match action {
            Action::Select { repo } => {
                self.saved.selected = repo;
                Value::Null
            }
            Action::RefreshProjects => {
                self.refresh_projects(&ctx);
                Value::Null
            }
            Action::SetGpuFilter { value } => {
                self.saved.gpu_filter = value.min(2);
                Value::Null
            }
            Action::CheckRepo { repo } => {
                let p = self.project(&repo)?;
                self.check_repo(&ctx, &p);
                Value::Null
            }
            Action::Install { repo } => {
                let p = self.project(&repo)?;
                self.start(&ctx, p, Kind::Install);
                Value::Null
            }
            Action::Later { repo, remote } => {
                self.saved.later.retain(|(r, _)| r != &repo);
                self.saved.later.push((repo, remote));
                Value::Null
            }
            Action::Play { repo } => {
                let p = self.project(&repo)?;
                let launch = self.saved.project_launch.get(&project_key(&p.repo));
                match PlayPresetPicker::for_project(&p, &self.saved.launch_presets, launch) {
                    Some(picker) => json!({ "next": "presets", "choice": picker.describe() }),
                    None => self.continue_play(&p),
                }
            }
            Action::ConfirmPreset { repo, index } => {
                let p = self.project(&repo)?;
                if self.job.lock().unwrap().running {
                    return Err("Wait for the current job to finish.".into());
                }
                let launch = self.saved.project_launch.get(&project_key(&p.repo));
                let preset = PlayPresetPicker::for_project(&p, &self.saved.launch_presets, launch)
                    .and_then(|picker| index.and_then(|i| picker.options.into_iter().nth(i)));
                let key = project_key(&p.repo);
                let launch = self.saved.project_launch.entry(key.clone()).or_default();
                if let Some(preset) = preset {
                    presets::apply_to_profile(&mut self.saved.launch_presets, launch, &key, preset)
                        .map_err(|e| format!("Could not apply the launch preset: {e}"))?;
                }
                launch.preset_choice_confirmed = true;
                self.continue_play(&p)
            }
            Action::StartWithSave { repo, choice } => {
                let p = self.project(&repo)?;
                if !self.files_ok() {
                    return Err("Choose the folder with your game files first.".into());
                }
                let choice = match choice {
                    SaveChoice::Continue => saves::LaunchSave::Continue,
                    SaveChoice::Fresh => saves::LaunchSave::Fresh,
                    SaveChoice::Copy { path } => saves::LaunchSave::Copy(PathBuf::from(path)),
                };
                self.start(&ctx, p, Kind::Play(choice));
                Value::Null
            }
            Action::Warm { repo } => {
                let p = self.project(&repo)?;
                self.start(&ctx, p, Kind::Warm);
                Value::Null
            }
            Action::Uninstall { repo } => {
                let p = self.project(&repo)?;
                self.start(&ctx, p, Kind::Uninstall);
                Value::Null
            }
            Action::Stop => {
                self.stop_job();
                Value::Null
            }
            Action::RemoveCustom { repo } => {
                self.saved.custom.retain(|q| q.repo != repo);
                if self.saved.selected.as_deref() == Some(repo.as_str()) {
                    self.saved.selected = None;
                }
                Value::Null
            }
            Action::AddRepoLink { link } => {
                let repo = parse_git_repo(&link)
                    .ok_or("Enter an HTTPS, HTTP, SSH or Git clone link without a password in it.")?;
                self.add_custom(repo.clone());
                json!(repo)
            }
            Action::ImportBuildZip { path } => {
                self.import_build_zip(&ctx, PathBuf::from(path));
                Value::Null
            }
            Action::DismissImportError => {
                self.job.lock().unwrap().import_error = None;
                Value::Null
            }
            Action::InstallTools => {
                self.start_tools(&ctx);
                Value::Null
            }
            Action::SetGameDir { path } => {
                self.saved.game_dir = path;
                self.files.clear();
                Value::Null
            }
            Action::FindGameFiles => {
                self.find_error = None;
                *self.finding.lock().unwrap() = Some(None);
                let (slot, notify) = (self.finding.clone(), ctx.clone());
                std::thread::spawn(move || {
                    let r = search_drives();
                    *slot.lock().unwrap() = Some(Some(r));
                    notify.request_repaint();
                });
                Value::Null
            }
            Action::SetSettings { patch } => {
                if let Some(v) = patch.warm_before_play {
                    self.saved.warm_before_play = v;
                }
                if let Some(v) = patch.warm_pipelines {
                    self.saved.warm_pipelines = v;
                }
                if let Some(v) = patch.launch_args {
                    self.saved.launch_args = v;
                }
                if let Some(v) = patch.extra_env {
                    self.saved.extra_env = v;
                }
                Value::Null
            }
            Action::SetDefaultGraphics { graphics } => {
                self.saved.series_s = graphics.series_s;
                self.saved.render_scale = graphics.render_scale;
                self.saved.fix_8gb = graphics.fix_8gb;
                self.saved.bright_shadows = graphics.bright_shadows;
                self.saved.fullscreen = graphics.fullscreen;
                Value::Null
            }
            Action::SetProjectGraphics { repo, graphics } => {
                self.launch_mut(&repo).graphics = graphics;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SetProjectArgs { repo, value } => {
                self.launch_mut(&repo).args = value;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SetProjectSwitches { repo, value } => {
                let launch = self.launch_mut(&repo);
                if value.is_none() {
                    launch.script_options = None;
                }
                launch.switches = value;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SetScriptOption { repo, option, enabled } => {
                use script_preset::{set_enabled, HOOKS, PROBE_RAYS, RAY_TRACING};
                let global = self.saved.extra_env.clone();
                let launch = self.launch_mut(&repo);
                let options = launch.script_options.unwrap_or_default();
                let mut switches = launch.switches.clone().unwrap_or(global);
                match option.as_str() {
                    "ray_tracing" => {
                        set_enabled(&mut switches, RAY_TRACING, enabled);
                        if enabled && options.probe_rays {
                            set_enabled(&mut switches, PROBE_RAYS, false);
                        }
                    }
                    "probe_rays" => set_enabled(&mut switches, PROBE_RAYS, enabled),
                    "hooks" => set_enabled(&mut switches, HOOKS, enabled),
                    _ => return Err(format!("Unknown switch {option}.")),
                }
                launch.switches = Some(switches);
                Value::Null
            }
            Action::SetProjectNotes { repo, notes } => {
                self.launch_mut(&repo).notes = notes;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SavePreset { repo, name, include_switches } => {
                let p = self.project(&repo)?;
                let standalone = p.runs_standalone();
                let key = project_key(&repo);
                let (global_args, global_switches) = (self.saved.launch_args.clone(), self.saved.extra_env.clone());
                let launch = self.saved.project_launch.get(&key);
                let inherited_args = if standalone { String::new() } else { global_args };
                let include_switches = include_switches && !standalone;
                let preset = LaunchPreset {
                    name: name.trim().to_owned(),
                    args: launch.and_then(|l| l.args.clone()).unwrap_or(inherited_args),
                    switches: include_switches.then(|| launch.and_then(|l| l.switches.clone()).unwrap_or(global_switches)),
                    script_options: if include_switches { launch.and_then(|l| l.script_options) } else { None },
                    projects: vec![key],
                    repo_script: None,
                };
                let index = presets::store(&mut self.saved.launch_presets, preset)?;
                json!(self.saved.launch_presets[index].name)
            }
            Action::ApplyPreset { repo, name } => {
                let p = self.project(&repo)?;
                let preset = match name {
                    Some(name) => self.saved.launch_presets.iter().find(|x| x.name == name).cloned(),
                    None => presets::detected_preset(&p),
                }
                .ok_or("That preset is gone.")?;
                let key = project_key(&repo);
                let launch = self.saved.project_launch.entry(key.clone()).or_default();
                let applied = presets::apply_to_profile(&mut self.saved.launch_presets, launch, &key, preset)?;
                json!(applied)
            }
            Action::DeletePreset { repo, name } => {
                self.saved.launch_presets.retain(|preset| preset.name != name);
                if let Some(launch) = self.saved.project_launch.get_mut(&project_key(&repo)) {
                    if launch.selected_preset.as_deref() == Some(&name) {
                        launch.selected_preset = None;
                    }
                }
                Value::Null
            }
            Action::ImportPresetFile { repo, path } => {
                let imported = presets::import_file(Path::new(&path));
                json!(self.accept_import(&repo, imported)?)
            }
            Action::ImportPresetText { repo, text } => {
                let imported = presets::decode(&text);
                json!(self.accept_import(&repo, imported)?)
            }
            Action::ExportPreset { name, path } => {
                let preset = self.saved.launch_presets.iter().find(|p| p.name == name).ok_or("That preset is gone.")?;
                let json = presets::encode(preset)?;
                std::fs::write(&path, json).map_err(|e| format!("Could not export the preset: {e}"))?;
                Value::Null
            }
            Action::PresetText { name } => {
                let preset = self.saved.launch_presets.iter().find(|p| p.name == name).ok_or("That preset is gone.")?;
                json!(presets::encode(preset)?)
            }
            Action::DetectedPreset { repo } => {
                let p = self.project(&repo)?;
                let key = project_key(&repo);
                let detected = (!p.runs_standalone()).then(|| presets::detected_preset(&p)).flatten().filter(|candidate| {
                    !self.saved.launch_presets.iter().any(|saved| saved.repo_script == candidate.repo_script && saved.projects.contains(&key))
                });
                json!(detected.map(|d| d.name))
            }
            Action::ChooseInstallDir { path } => {
                self.choose_install_dir(PathBuf::from(path).join(brand::FOLDER));
                Value::Null
            }
            Action::UseDefaultInstallDir => {
                self.choose_install_dir(default_workspace());
                Value::Null
            }
            Action::ConfirmMove { answer } => {
                if let Some((old, new)) = self.pending_move.take() {
                    match answer.as_str() {
                        "move" => {
                            self.apply_install_dir(&new);
                            self.start_move(&ctx, old, new);
                        }
                        "keep" => self.apply_install_dir(&new),
                        _ => {}
                    }
                }
                Value::Null
            }
            Action::Open { what, repo } => {
                let path = match (what.as_str(), repo) {
                    ("project_folder", Some(repo)) => {
                        let p = self.project(&repo)?;
                        p.local_exe
                            .as_deref()
                            .and_then(|exe| Path::new(exe).parent())
                            .map_or_else(|| repo_dir(&p.repo), Path::to_path_buf)
                    }
                    ("backgrounds", _) => crate::backgrounds::folder(),
                    ("workspace", _) => workspace(),
                    ("update", _) => match &self.updater.state {
                        updater::State::Downloaded(update) => update.path.parent().map(Path::to_path_buf).ok_or("No update folder.")?,
                        _ => return Err("No update has been downloaded.".into()),
                    },
                    _ => return Err(format!("Nothing to open for {what}.")),
                };
                let _ = std::fs::create_dir_all(&path);
                open_folder(&path)?;
                Value::Null
            }
            Action::ExportLog { path } => {
                std::fs::write(&path, self.whole_log()).map_err(|e| format!("Could not export the log: {e}"))?;
                Value::Null
            }
            Action::CheckUpdate => {
                self.updater.check(&ctx);
                Value::Null
            }
            Action::DownloadUpdate => {
                self.updater.download(&ctx);
                Value::Null
            }
            Action::RestartToUpdate => {
                if self.job.lock().unwrap().running {
                    return Err("Wait for the current job to finish first.".into());
                }
                let updater::State::Downloaded(update) = &self.updater.state else {
                    return Err("No update is ready.".into());
                };
                match updater::restart_with(update) {
                    Ok(()) => return Ok(Outcome::Exit),
                    Err(error) => {
                        self.updater.restart_error = Some(error.clone());
                        return Err(error);
                    }
                }
            }
            Action::SavesList { repo } => {
                let p = self.project(&repo)?;
                saves_list(&p)
            }
            Action::Checkpoint { name, source } => {
                let root = workspace().join("checkpoints");
                saves::checkpoint(&root, &name, Path::new(&source)).map_err(|e| format!("Could not save the checkpoint: {e}"))?;
                Value::Null
            }
            Action::ImportSaveZip { path } => {
                let root = workspace().join("checkpoints");
                let (name, _) = save_zip::import(&root, Path::new(&path)).map_err(|e| format!("Could not import the save: {e}"))?;
                json!(name)
            }
            Action::ExportSave { source, path } => {
                let destination = PathBuf::from(path);
                let destination = if destination.extension().is_none() { destination.with_extension("zip") } else { destination };
                save_zip::export(Path::new(&source), &destination).map_err(|e| format!("Could not export the save: {e}"))?;
                json!(destination.display().to_string())
            }
            Action::AcceptNotice => {
                self.saved.notice_accepted = NOTICE_VERSION;
                Value::Null
            }
            Action::DeleteSave { path } => {
                saves::delete_save(&workspace(), Path::new(&path)).map_err(|e| format!("Could not delete the save: {e}"))?;
                Value::Null
            }
        };
        ctx.request_repaint();
        Ok(Outcome::Value(value))
    }

    /// The project with this repo, listed or added.
    fn project(&self, repo: &str) -> Result<Project, String> {
        self.all_projects().into_iter().find(|p| p.repo == repo).ok_or_else(|| "That project is no longer in the list.".to_owned())
    }

    fn launch_mut(&mut self, repo: &str) -> &mut ProjectLaunch {
        self.saved.project_launch.entry(project_key(repo)).or_default()
    }

    /// A project's settings that hold nothing are dropped, so it follows the global ones.
    fn tidy_launch(&mut self, repo: &str) {
        let key = project_key(repo);
        if self.saved.project_launch.get(&key).is_some_and(|launch| {
            launch.args.is_none()
                && launch.switches.is_none()
                && launch.graphics.is_none()
                && launch.notes.is_empty()
                && !launch.has_confirmed_preset()
        }) {
            self.saved.project_launch.remove(&key);
        }
    }

    /// After the preset question: a standalone program starts; a game build asks for a save first.
    fn continue_play(&mut self, p: &Project) -> Value {
        if p.runs_standalone() {
            let ctx = self.ctx.clone();
            self.start(&ctx, p.clone(), Kind::Play(saves::LaunchSave::Continue));
            json!({ "next": "started" })
        } else {
            json!({ "next": "saves" })
        }
    }

    fn accept_import(&mut self, repo: &str, imported: Result<LaunchPreset, String>) -> Result<String, String> {
        let mut preset = imported?;
        preset.projects = vec![project_key(repo)];
        preset.repo_script = None;
        let index = presets::store(&mut self.saved.launch_presets, preset)?;
        Ok(self.saved.launch_presets[index].name.clone())
    }

    /// Ends the running warm-up (its process and the workers it started).
    fn stop_job(&self) {
        let pid = {
            let mut j = self.job.lock().unwrap();
            j.stopped = true;
            j.pid
        };
        if let Some(pid) = pid {
            #[cfg(windows)]
            {
                let mut c = Command::new("taskkill");
                c.args(["/PID", &pid.to_string(), "/T", "/F"]);
                hide_window(&mut c);
                let _ = c.spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = Command::new("kill").arg(pid.to_string()).spawn();
            }
        }
    }

    /// Puts a repo added by link at the top of the list and picks it.
    fn add_custom(&mut self, repo: String) {
        let name = repo.split('/').next_back().unwrap_or(&repo).trim_end_matches(".git").to_string();
        self.saved.custom.retain(|p| p.repo != repo);
        self.saved.custom.insert(
            0,
            Project {
                name,
                repo: repo.clone(),
                branch: None,
                gpu: Vec::new(),
                fps: None,
                render: None,
                verified: false,
                summary: String::new(),
                custom: true,
                local_exe: None,
                archive_exe: None,
                highlights: Vec::new(),
                blockers: Vec::new(),
            },
        );
        self.saved.selected = Some(repo);
    }
}

fn open_folder(path: &Path) -> Result<(), String> {
    let program = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    Command::new(program).arg(path).spawn().map(|_| ()).map_err(|e| format!("Could not open {}: {e}", path.display()))
}

/// The saves a project can start from: this build's, other builds' and the favourites.
fn saves_list(project: &Project) -> Value {
    let ws = workspace();
    let data = ws.join("data");
    let project_data = data.join(slug(&project.repo));
    let checkpoint_root = ws.join("checkpoints");
    let current = saves::save_folder(&project_data);
    let option = |title: String, detail: &str, kind: &str, path: Option<&Path>, source: Option<&Path>| {
        json!({
            "title": title,
            "detail": detail,
            "kind": kind,
            "path": path.map(|p| p.display().to_string()),
            "source": source.map(|p| p.display().to_string()),
        })
    };
    let mut this_build = vec![
        option("Continue".into(), "Use this build's active save.", "continue", None, current.as_deref()),
        option("Start fresh".into(), "Begin a new game in a separate save profile.", "fresh", None, None),
    ];
    for (name, path) in saves::previous_saves(&project_data) {
        this_build.push(option(name, "Starts from a copy of an earlier save.", "copy", Some(&path), Some(&path)));
    }
    let mut other_builds = Vec::new();
    for entry in std::fs::read_dir(&data).into_iter().flatten().flatten() {
        if entry.path() != project_data {
            if let Some(path) = saves::save_folder(&entry.path()) {
                let name = entry.file_name().to_string_lossy().replacen('_', "/", 1);
                other_builds.push((name, path));
            }
        }
    }
    other_builds.sort();
    let mut favorites = Vec::new();
    for entry in std::fs::read_dir(&checkpoint_root).into_iter().flatten().flatten() {
        let filename = entry.file_name();
        if let Some(name) = filename.to_str().and_then(|name| name.strip_prefix("Checkpoint - ")) {
            if entry.path().is_dir() {
                favorites.push((name.to_owned(), entry.path()));
            }
        }
    }
    favorites.sort();
    json!({
        "has_current": current.is_some(),
        "this_build": this_build,
        "other_builds": other_builds.iter().map(|(n, p)| option(n.clone(), "Copies this build's active save.", "copy", Some(p), Some(p))).collect::<Vec<_>>(),
        "favorites": favorites.iter().map(|(n, p)| option(n.clone(), "Starts from a copy of this checkpoint.", "copy", Some(p), None)).collect::<Vec<_>>(),
    })
}
