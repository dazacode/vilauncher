//! Everything the window can ask the launcher to do. Each action answers
//! with a value (often nothing) or a message for the player when it can't.

use super::presets::{self, PlayPresetPicker};
use super::*;
use crate::addons;
use crate::i18n::localize;
use crate::save_zip;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Action {
    Select {
        repo: Option<String>,
    },
    RefreshProjects,
    ResetPersonalFps {
        repo: String,
    },
    SetIncludeExact60Fps {
        include: bool,
    },
    SetGpuFilter {
        value: usize,
    },
    SetListOptions {
        show_nuked: bool,
        sort: usize,
        tier: usize,
    },
    SetSpec {
        ram_gb: Option<f64>,
        vram_gb: Option<f64>,
    },
    Report {
        repo: String,
    },
    OpenLink {
        url: String,
    },
    CheckRepo {
        repo: String,
    },
    /// Install, update or reinstall.
    Install {
        repo: String,
    },
    Later {
        repo: String,
        remote: String,
    },
    /// The first step of Play: answers what to ask before the game starts.
    Play {
        repo: String,
    },
    /// A launch preset picked (or none: the current settings), then on to the saves.
    ConfirmPreset {
        repo: String,
        index: Option<usize>,
    },
    StartWithSave {
        repo: String,
        choice: SaveChoice,
    },
    Warm {
        repo: String,
    },
    /// Check the installed build for known problems, fix what can be fixed and verify it.
    Fix {
        repo: String,
    },
    /// Swap the installed build with the one before the last update.
    Rollback {
        repo: String,
    },
    /// Show the warning before an update (false: from now on, not again).
    SetUpdateWarning {
        show: bool,
    },
    /// Turns the keyboard-as-controller keys on or off (controller_keys.rs).
    SetControllerKeys {
        on: bool,
    },
    /// Gives a controller control another key; `key` None goes back to the layer's own.
    SetControllerKey {
        control: String,
        key: Option<u16>,
    },
    /// Let AI agents help with builds (off by default; turning it on needs the "agent-access" warning first).
    SetAgentAccess {
        enabled: bool,
    },
    /// The player accepted one of the responsibility warnings (only the known ids are kept).
    Acknowledge {
        id: String,
    },
    Uninstall {
        repo: String,
    },
    /// Disk use per project and in total, for the Storage section of Settings.
    StorageScan,
    /// Deletes a project's shader cache. Its saves and sources stay.
    ClearCache {
        repo: String,
    },
    /// Deletes the build kept for rollback, if there is one.
    ClearPrevious {
        repo: String,
    },
    Stop,
    RemoveCustom {
        repo: String,
    },
    AddRepoLink {
        link: String,
    },
    ImportBuildZip {
        path: String,
    },
    DismissImportError,
    InstallTools,
    SetGameDir {
        path: String,
    },
    FindGameFiles,
    SetSettings {
        patch: SettingsPatch,
    },
    SetDefaultGraphics {
        graphics: GraphicsSettings,
    },
    SetProjectGraphics {
        repo: String,
        graphics: Option<GraphicsSettings>,
    },
    /// Keeps the game's own F1 settings for resolution, render scale and upscaler (see launcher.rs).
    SetProjectKeepGameSettings {
        repo: String,
        value: bool,
    },
    SetProjectArgs {
        repo: String,
        value: Option<String>,
    },
    /// The branch a git project is built from; `None` goes back to the feed's branch.
    SetProjectBranch {
        repo: String,
        value: Option<String>,
    },
    SetProjectSwitches {
        repo: String,
        value: Option<String>,
    },
    /// Turns an add-on on or off for a project. Turning on needs `confirmed`.
    AddonSet {
        repo: String,
        id: String,
        on: bool,
        #[serde(default)]
        confirmed: bool,
    },
    SetScriptOption {
        repo: String,
        option: String,
        enabled: bool,
    },
    SetProjectNotes {
        repo: String,
        notes: String,
    },
    SavePreset {
        repo: String,
        name: String,
        include_switches: bool,
    },
    ApplyPreset {
        repo: String,
        name: Option<String>,
    },
    DeletePreset {
        repo: String,
        name: String,
    },
    ImportPresetFile {
        repo: String,
        path: String,
    },
    ImportPresetText {
        repo: String,
        text: String,
    },
    ExportPreset {
        name: String,
        path: String,
    },
    PresetText {
        name: String,
    },
    DetectedPreset {
        repo: String,
    },
    ChooseInstallDir {
        path: String,
    },
    UseDefaultInstallDir,
    /// The answer to "move the installed projects?": "move", "keep" or "cancel".
    ConfirmMove {
        answer: String,
    },
    Open {
        what: String,
        repo: Option<String>,
    },
    ExportLog {
        path: String,
        repo: Option<String>,
    },
    CheckUpdate,
    DownloadUpdate,
    RestartToUpdate,
    SavesList {
        repo: String,
    },
    Checkpoint {
        name: String,
        source: String,
    },
    ImportSaveZip {
        path: String,
    },
    ExportSave {
        source: String,
        path: String,
    },
    DeleteSave {
        path: String,
    },
    /// The player agreed to the legal notice.
    AcceptNotice,
    /// The window's language: a tag ("en", "zh-TW", "zh-CN"), or none to follow the system.
    SetLanguage {
        language: Option<String>,
    },
    /// The system's languages in order of preference (the window's `navigator.languages`).
    SetSystemLanguages {
        languages: Vec<String>,
    },
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
            Action::ResetPersonalFps { repo } => {
                self.project(&repo)?;
                self.saved.play_stats.reset_fps(&project_key(&repo));
                Value::Null
            }
            Action::SetIncludeExact60Fps { include } => {
                self.saved.include_exact_60_fps = include;
                Value::Null
            }
            Action::SetGpuFilter { value } => {
                self.saved.gpu_filter = value.min(2);
                Value::Null
            }
            Action::OpenLink { url } => {
                open_link(&url)?;
                Value::Null
            }
            Action::Report { repo } => {
                // Nothing is sent: the report is text, and the links open GitHub's issue form.
                let p = self.project(&repo)?;
                let (stage, log) = {
                    self.project_job(&repo)
                        .map(|job| {
                            let job = job.lock().unwrap();
                            (job.stage.clone(), job.log.clone())
                        })
                        .unwrap_or_default()
                };
                let system = self.system.lock().unwrap().clone().unwrap_or_default();
                let mut private = crate::report::private_paths(&workspace());
                private.extend([
                    std::env::var("USERNAME").unwrap_or_default(),
                    std::env::var("USERPROFILE").unwrap_or_default(),
                    self.saved.install_dir.clone(),
                    self.saved.game_dir.clone(),
                ]);
                private.retain(|s| !s.trim().is_empty());
                let report = crate::report::make(&crate::report::Facts {
                    project: &p.name,
                    repo: &p.repo,
                    stage: &stage,
                    log: &log,
                    gpus: system.gpus.iter().map(|g| g.name.clone()).collect(),
                    cpu: system.cpu.clone(),
                    ram_gb: system.ram_gb,
                    private,
                });
                serde_json::to_value(report).map_err(|error| error.to_string())?
            }
            Action::SetListOptions {
                show_nuked,
                sort,
                tier,
            } => {
                self.saved.show_nuked = show_nuked;
                self.saved.list_sort = sort.min(5);
                self.saved.list_tier = tier.min(2);
                Value::Null
            }
            Action::SetSpec { ram_gb, vram_gb } => {
                // Only plausible amounts are kept; anything else goes back to what the launcher found.
                let keep = |v: Option<f64>| v.filter(|g| g.is_finite() && *g > 0.0 && *g <= 1024.0);
                self.saved.spec_ram_gb = keep(ram_gb);
                self.saved.spec_vram_gb = keep(vram_gb);
                Value::Null
            }
            Action::CheckRepo { repo } => {
                let p = self.project(&repo)?;
                self.check_repo(&ctx, &p);
                Value::Null
            }
            Action::Install { repo } => {
                let p = self.project(&repo)?;
                if self.job.lock().unwrap().running
                    || self
                        .builds
                        .get(&p.repo)
                        .is_some_and(|job| job.lock().unwrap().running)
                {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                // An update resets the project's code, which would drop an add-on's patches
                // while its switches stayed on. Turn the add-ons off first.
                let on = self
                    .saved
                    .project_launch
                    .get(&project_key(&p.repo))
                    .map(|launch| launch.addons.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                if !on.is_empty() {
                    return Err(format!(
                        "Turn off {} before updating this project: an update resets its code, and the add-on's patches would be lost.",
                        on.join(", ")
                    ));
                }
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
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                let launch = self.saved.project_launch.get(&project_key(&p.repo));
                match PlayPresetPicker::for_project(&p, &self.saved.launch_presets, launch) {
                    Some(picker) => json!({ "next": "presets", "choice": picker.describe() }),
                    None => self.continue_play(&p),
                }
            }
            Action::ConfirmPreset { repo, index } => {
                let p = self.project(&repo)?;
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                let launch = self.saved.project_launch.get(&project_key(&p.repo));
                let preset = PlayPresetPicker::for_project(&p, &self.saved.launch_presets, launch)
                    .and_then(|picker| index.and_then(|i| picker.options.into_iter().nth(i)));
                let key = project_key(&p.repo);
                let launch = self.saved.project_launch.entry(key.clone()).or_default();
                if let Some(preset) = preset {
                    presets::apply_to_profile(&mut self.saved.launch_presets, launch, &key, preset)
                        .map_err(|error| msg!("Could not apply the launch preset: {error}"))?;
                }
                launch.preset_choice_confirmed = true;
                self.continue_play(&p)
            }
            Action::StartWithSave { repo, choice } => {
                let p = self.project(&repo)?;
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                // Looked at right now: the drive may have been unplugged since the last check.
                self.files_checked = None;
                self.check_files();
                if !self.files_ok() {
                    if self.unplugged {
                        return Err(msg_str!("The drive with your game files is not connected. Plug it in and press Play again.").into());
                    }
                    return Err(msg_str!("Choose the folder with your game files first.").into());
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
            Action::Rollback { repo } => {
                let p = self.project(&repo)?;
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                if previous::previous_commit(&previous_dir(&p.repo)).is_none() {
                    return Err(msg_str!(
                        "There is no previous build to restore for this project."
                    )
                    .into());
                }
                self.start(&ctx, p, Kind::Rollback);
                Value::Null
            }
            Action::SetUpdateWarning { show } => {
                self.saved.skip_update_warning = !show;
                Value::Null
            }
            Action::SetControllerKeys { on } => {
                self.saved.controller_keys_on = on;
                crate::controller_keys::configure(on, &self.saved.controller_key_bindings);
                Value::Null
            }
            Action::SetControllerKey { control, key } => {
                if crate::controller_keys::control(&control).is_none() {
                    return Err(format!("There is no controller control {control}."));
                }
                match key {
                    Some(key) => {
                        crate::controller_keys::check_key(&self.saved.controller_key_bindings, &control, key)?;
                        self.saved.controller_key_bindings.insert(control, key);
                    }
                    None => {
                        self.saved.controller_key_bindings.remove(&control);
                    }
                }
                crate::controller_keys::configure(
                    self.saved.controller_keys_on,
                    &self.saved.controller_key_bindings,
                );
                Value::Null
            }
            Action::SetAgentAccess { enabled } => {
                self.saved.agent_access = enabled;
                Value::Null
            }
            Action::Acknowledge { id } => {
                let id =
                    known_responsibility(&id).ok_or_else(|| msg!("Unknown warning: {id}"))?;
                if !self.saved.acknowledged.iter().any(|a| a == id) {
                    self.saved.acknowledged.push(id.to_string());
                }
                Value::Null
            }
            Action::Fix { repo } => {
                let p = self.project(&repo)?;
                self.start(&ctx, p, Kind::Fix);
                Value::Null
            }
            Action::Uninstall { repo } => {
                let p = self.project(&repo)?;
                self.start(&ctx, p, Kind::Uninstall);
                Value::Null
            }
            Action::StorageScan => {
                let ws = workspace();
                let projects = self.all_projects();
                let slugs: Vec<String> = projects.iter().map(|p| slug(&p.repo)).collect();
                let mut view = crate::storage::scan(&ws, &slugs);
                for (row, p) in view.projects.iter_mut().zip(&projects) {
                    row.repo = p.repo.clone();
                    row.label = p.name.clone();
                }
                view.free_bytes = free_bytes(&ws);
                serde_json::to_value(view).map_err(|error| error.to_string())?
            }
            Action::ClearCache { repo } => {
                let p = self.project(&repo)?;
                self.clear_folder(&p, "cache")?;
                Value::Null
            }
            Action::ClearPrevious { repo } => {
                let p = self.project(&repo)?;
                self.clear_folder(&p, "previous")?;
                Value::Null
            }
            Action::Stop => {
                self.stop_job();
                Value::Null
            }
            Action::RemoveCustom { repo } => {
                if self
                    .builds
                    .get(&repo)
                    .is_some_and(|job| job.lock().unwrap().running)
                {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                self.builds.0.remove(&ProjectBuilds::key(&repo));
                self.saved.custom.retain(|q| q.repo != repo);
                if self.saved.selected.as_deref() == Some(repo.as_str()) {
                    self.saved.selected = None;
                }
                Value::Null
            }
            Action::AddRepoLink { link } => {
                let repo = parse_git_repo(&link).ok_or(msg_str!(
                    "Enter an HTTPS, HTTP, SSH or Git clone link without a password in it."
                ))?;
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
                self.saved.set_default_graphics(graphics);
                Value::Null
            }
            Action::SetProjectGraphics { repo, graphics } => {
                self.launch_mut(&repo).graphics = graphics;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SetProjectBranch { repo, value } => {
                let value = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
                if let Some(branch) = &value {
                    if !crate::launcher::valid_branch(branch) {
                        return Err(msg_str!("That is not a branch name.").into());
                    }
                }
                self.saved
                    .branches
                    .retain(|(saved, _)| crate::launcher::project_key(saved) != crate::launcher::project_key(&repo));
                if let Some(branch) = value {
                    self.saved.branches.push((repo, branch));
                }
                Value::Null
            }
            Action::SetProjectKeepGameSettings { repo, value } => {
                self.launch_mut(&repo).keep_game_settings = value;
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
            Action::AddonSet {
                repo,
                id,
                on,
                confirmed,
            } => {
                self.set_addon(&repo, &id, on, confirmed)?;
                Value::Null
            }
            Action::SetScriptOption {
                repo,
                option,
                enabled,
            } => {
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
                    _ => return Err(msg!("Unknown switch {option}.")),
                }
                launch.switches = Some(switches);
                Value::Null
            }
            Action::SetProjectNotes { repo, notes } => {
                self.launch_mut(&repo).notes = notes;
                self.tidy_launch(&repo);
                Value::Null
            }
            Action::SavePreset {
                repo,
                name,
                include_switches,
            } => {
                let p = self.project(&repo)?;
                let standalone = p.runs_standalone();
                let key = project_key(&repo);
                let (global_args, global_switches) =
                    (self.saved.launch_args.clone(), self.saved.extra_env.clone());
                let launch = self.saved.project_launch.get(&key);
                let inherited_args = if standalone {
                    String::new()
                } else {
                    global_args
                };
                let include_switches = include_switches && !standalone;
                let preset = LaunchPreset {
                    name: name.trim().to_owned(),
                    args: launch
                        .and_then(|l| l.args.clone())
                        .unwrap_or(inherited_args),
                    switches: include_switches.then(|| {
                        launch
                            .and_then(|l| l.switches.clone())
                            .unwrap_or(global_switches)
                    }),
                    script_options: if include_switches {
                        launch.and_then(|l| l.script_options)
                    } else {
                        None
                    },
                    projects: vec![key],
                    repo_script: None,
                };
                let index = presets::store(&mut self.saved.launch_presets, preset)?;
                json!(self.saved.launch_presets[index].name)
            }
            Action::ApplyPreset { repo, name } => {
                let p = self.project(&repo)?;
                let preset = match name {
                    Some(name) => self
                        .saved
                        .launch_presets
                        .iter()
                        .find(|x| x.name == name)
                        .cloned(),
                    None => presets::detected_preset(&p),
                }
                .ok_or(msg_str!("That preset is gone."))?;
                let key = project_key(&repo);
                let launch = self.saved.project_launch.entry(key.clone()).or_default();
                let applied = presets::apply_to_profile(
                    &mut self.saved.launch_presets,
                    launch,
                    &key,
                    preset,
                )?;
                json!(applied)
            }
            Action::DeletePreset { repo, name } => {
                self.saved
                    .launch_presets
                    .retain(|preset| preset.name != name);
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
                let preset = self
                    .saved
                    .launch_presets
                    .iter()
                    .find(|p| p.name == name)
                    .ok_or(msg_str!("That preset is gone."))?;
                let json = presets::encode(preset)?;
                std::fs::write(&path, json)
                    .map_err(|error| msg!("Could not export the preset: {error}"))?;
                Value::Null
            }
            Action::PresetText { name } => {
                let preset = self
                    .saved
                    .launch_presets
                    .iter()
                    .find(|p| p.name == name)
                    .ok_or(msg_str!("That preset is gone."))?;
                json!(presets::encode(preset)?)
            }
            Action::DetectedPreset { repo } => {
                let p = self.project(&repo)?;
                let key = project_key(&repo);
                let detected = (!p.runs_standalone())
                    .then(|| presets::detected_preset(&p))
                    .flatten()
                    .filter(|candidate| {
                        !self.saved.launch_presets.iter().any(|saved| {
                            saved.repo_script == candidate.repo_script
                                && saved.projects.contains(&key)
                        })
                    });
                json!(detected.map(|d| d.name))
            }
            Action::ChooseInstallDir { path } => {
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
                self.choose_install_dir(PathBuf::from(path).join(brand::FOLDER));
                Value::Null
            }
            Action::UseDefaultInstallDir => {
                self.choose_install_dir(default_workspace());
                Value::Null
            }
            Action::ConfirmMove { answer } => {
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish.").into());
                }
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
                let path = match (what.as_str(), repo.as_deref()) {
                    ("project_folder", Some(repo)) => {
                        let p = self.project(repo)?;
                        p.local_exe
                            .as_deref()
                            .and_then(|exe| Path::new(exe).parent())
                            .map_or_else(|| repo_dir(&p.repo), Path::to_path_buf)
                    }
                    ("backgrounds", _) => crate::backgrounds::folder(),
                    ("workspace", _) => workspace(),
                    ("update", _) => match &self.updater.state {
                        updater::State::Downloaded(update) => update
                            .path
                            .parent()
                            .map(Path::to_path_buf)
                            .ok_or(msg_str!("No update folder."))?,
                        _ => return Err(msg_str!("No update has been downloaded.").into()),
                    },
                    _ => return Err(msg!("Nothing to open for {what}.")),
                };
                let _ = std::fs::create_dir_all(&path);
                // A build's folder opens with its program selected where the
                // system can (Finder); everything else opens the folder itself.
                match repo.as_deref().and_then(|repo| self.project(repo).ok()) {
                    Some(p) if what == "project_folder" => match p.local_exe.as_deref() {
                        Some(exe) if Path::new(exe).is_file() => reveal(Path::new(exe))?,
                        _ => open_folder(&path)?,
                    },
                    _ => open_folder(&path)?,
                }
                Value::Null
            }
            Action::ExportLog { path, repo } => {
                std::fs::write(&path, self.whole_log(repo.as_deref()))
                    .map_err(|error| msg!("Could not export the log: {error}"))?;
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
                if self.job_running().0 {
                    return Err(msg_str!("Wait for the current job to finish first.").into());
                }
                let updater::State::Downloaded(update) = &self.updater.state else {
                    return Err(msg_str!("No update is ready.").into());
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
                saves::checkpoint(&root, &name, Path::new(&source))
                    .map_err(|error| msg!("Could not save the checkpoint: {error}"))?;
                Value::Null
            }
            Action::ImportSaveZip { path } => {
                let root = workspace().join("checkpoints");
                let (name, _) = save_zip::import(&root, Path::new(&path))
                    .map_err(|error| msg!("Could not import the save: {error}"))?;
                json!(name)
            }
            Action::ExportSave { source, path } => {
                let destination = PathBuf::from(path);
                let destination = if destination.extension().is_none() {
                    destination.with_extension("zip")
                } else {
                    destination
                };
                save_zip::export(Path::new(&source), &destination)
                    .map_err(|error| msg!("Could not export the save: {error}"))?;
                json!(destination.display().to_string())
            }
            Action::AcceptNotice => {
                self.saved.notice_accepted = NOTICE_VERSION;
                Value::Null
            }
            Action::SetLanguage { language } => {
                if let Some(tag) = &language {
                    let locale = crate::i18n::Locale::from_tag(tag)
                        .ok_or_else(|| msg!("Unknown language: {tag}"))?;
                    self.saved.language = Some(locale.tag().to_owned());
                } else {
                    self.saved.language = None;
                }
                self.apply_language();
                Value::Null
            }
            Action::SetSystemLanguages { languages } => {
                self.system_languages = languages.into_iter().take(16).collect();
                self.apply_language();
                Value::Null
            }
            Action::DeleteSave { path } => {
                saves::delete_save(&workspace(), Path::new(&path))
                    .map_err(|error| msg!("Could not delete the save: {error}"))?;
                Value::Null
            }
        };
        ctx.request_repaint();
        Ok(Outcome::Value(value))
    }

    /// Shows the window in the player's language: their choice, else the system's.
    pub(crate) fn apply_language(&self) {
        crate::i18n::set_current(crate::i18n::resolve(
            self.saved.language.as_deref(),
            &self.system_languages,
        ));
    }

    /// Deletes `p`'s folder under the workspace's `part` folder (`cache` or `previous`).
    /// Saves (`data`) and sources are never touched.
    fn clear_folder(&self, p: &Project, part: &str) -> Result<(), String> {
        if self.job_running().0 {
            return Err(msg_str!("Wait until the current job finishes.").into());
        }
        let ws = workspace();
        let dir = ws.join(part).join(slug(&p.repo));
        if !crate::storage::may_clear(&ws, &dir) {
            return Err(msg_str!("That folder is outside the install folder.").into());
        }
        if !dir.exists() {
            return Ok(());
        }
        remove_tree_forcibly(&dir, &|_: String| {})
            .map_err(|error| msg!("Could not clear it: {error}"))
    }

    /// The project with this repo, listed or added.
    fn project(&self, repo: &str) -> Result<Project, String> {
        self.all_projects()
            .into_iter()
            .find(|p| p.repo == repo)
            .ok_or_else(|| msg_str!("That project is no longer in the list.").to_owned())
    }

    /// Turns an add-on on or off for a project (addons.rs). Turning on runs the add-on's
    /// patches on the project's code, so it needs the player's confirmation.
    fn set_addon(&mut self, repo: &str, id: &str, on: bool, confirmed: bool) -> Result<(), String> {
        if self.job_running().0 {
            return Err("Wait for the current job to finish.".into());
        }
        let p = self.project(repo)?;
        let dir = repo_dir(&p.repo);
        let key = project_key(repo);
        let turned_on = self
            .saved
            .project_launch
            .get(&key)
            .is_some_and(|launch| launch.addons.contains_key(id));
        if on {
            if turned_on {
                return Ok(());
            }
            let (catalog, _) = addons::catalog(&workspace().join("add-ons"));
            let addon = catalog
                .into_iter()
                .find(|a| a.id == id)
                .ok_or_else(|| format!("There is no add-on {id}."))?;
            if !confirmed {
                return Err(format!("{}\n\nConfirm to turn it on.", addon.warning));
            }
            if !dir.join(".git").is_dir() {
                return Err("Install the project first.".into());
            }
            let source = addons::fetch_source(&addon, &workspace().join("add-on-sources").join(id))?;
            let mut applied = addons::apply(&dir, &addon, &source)?;
            // Remember what the player had, so turning the add-on off gives it back.
            let switches = self.effective_switches(repo);
            let args = self.effective_args(repo);
            let words: Vec<&str> = args.split_whitespace().collect();
            applied.added_args = addon
                .args
                .iter()
                .filter(|arg| !words.contains(&arg.as_str()))
                .cloned()
                .collect();
            applied.prior = addon
                .switches
                .keys()
                .map(|key| (key.clone(), addons::switch_value(&switches, key)))
                .collect();
            let new_switches = addons::with_switches(&switches, &addon.switches, true);
            let new_args = addons::with_args(&args, &applied.added_args, true);
            let launch = self.launch_mut(repo);
            launch.switches = Some(new_switches);
            if !applied.added_args.is_empty() {
                launch.args = Some(new_args);
            }
            launch.addons.insert(id.to_string(), applied);
        } else {
            if !turned_on {
                return Ok(());
            }
            let applied = self.launch_mut(repo).addons.get(id).cloned().unwrap_or_default();
            addons::remove(&dir, &applied)?;
            let switches = self.effective_switches(repo);
            let args = self.effective_args(repo);
            let new_switches = addons::restore_switches(&switches, &applied.prior);
            let new_args = addons::with_args(&args, &applied.added_args, false);
            let launch = self.launch_mut(repo);
            launch.switches = Some(new_switches);
            if !applied.added_args.is_empty() {
                launch.args = Some(new_args);
            }
            launch.addons.remove(id);
            self.tidy_launch(repo);
        }
        Ok(())
    }

    /// The switches a project launches with: its own, else the shared ones.
    fn effective_switches(&self, repo: &str) -> String {
        self.saved
            .project_launch
            .get(&project_key(repo))
            .and_then(|launch| launch.switches.clone())
            .unwrap_or_else(|| self.saved.extra_env.clone())
    }

    /// The launch arguments a project uses: its own, else the shared ones.
    fn effective_args(&self, repo: &str) -> String {
        self.saved
            .project_launch
            .get(&project_key(repo))
            .and_then(|launch| launch.args.clone())
            .unwrap_or_else(|| self.saved.launch_args.clone())
    }

    fn launch_mut(&mut self, repo: &str) -> &mut ProjectLaunch {
        self.saved
            .project_launch
            .entry(project_key(repo))
            .or_default()
    }

    /// A project's settings that hold nothing are dropped, so it follows the global ones.
    fn tidy_launch(&mut self, repo: &str) {
        let key = project_key(repo);
        if self.saved.project_launch.get(&key).is_some_and(|launch| {
            launch.args.is_none()
                && launch.switches.is_none()
                && launch.graphics.is_none()
                && launch.notes.is_empty()
                && launch.addons.is_empty()
                && !launch.keep_game_settings
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

    fn accept_import(
        &mut self,
        repo: &str,
        imported: Result<LaunchPreset, String>,
    ) -> Result<String, String> {
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
            // A close first, so the game can save; forced only if it is still running later.
            crate::platform::stop_program(pid);
        }
    }

    /// Puts a repo added by link at the top of the list and picks it.
    fn add_custom(&mut self, repo: String) {
        let name = repo
            .split('/')
            .next_back()
            .unwrap_or(&repo)
            .trim_end_matches(".git")
            .to_string();
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
                listing: Default::default(),
            },
        );
        self.saved.selected = Some(repo);
    }
}

/// The responsibility warnings the player can accept. Anything else is refused.
const RESPONSIBILITY_IDS: [&str; 3] = ["import-repo", "agent-access", "agent-patch"];

/// The id as the fixed list spells it, or None for an unknown id.
fn known_responsibility(id: &str) -> Option<&'static str> {
    RESPONSIBILITY_IDS
        .iter()
        .copied()
        .find(|known| *known == id)
}

/// Opens an https link in the player's browser. Only plain https links are accepted, and no shell
/// is involved: the link is passed to Explorer as one argument.
fn open_link(url: &str) -> Result<(), String> {
    let plain = url.len() <= 2048
        && url.starts_with("https://")
        && !url.chars().any(|c| c.is_control() || c.is_whitespace());
    if !plain {
        return Err(msg_str!("Only https links can be opened from the launcher.").into());
    }
    crate::platform::open_url(url)
}

fn open_folder(path: &Path) -> Result<(), String> {
    crate::platform::open_folder(path)
}

fn reveal(path: &Path) -> Result<(), String> {
    crate::platform::reveal(path)
}

/// The saves a project can start from: this build's, other builds' and the favourites.
fn saves_list(project: &Project) -> Value {
    let ws = workspace();
    let data = ws.join("data");
    let project_data = data.join(slug(&project.repo));
    let checkpoint_root = ws.join("checkpoints");
    let current = saves::save_folder(&project_data);
    // The details, and the titles the launcher makes itself, in the player's language.
    // Other builds' and checkpoints' names are shown as they are.
    let option =
        |title: String, detail: &str, kind: &str, path: Option<&Path>, source: Option<&Path>| {
            json!({
                "title": title,
                "detail": localize(detail),
                "kind": kind,
                "path": path.map(|p| p.display().to_string()),
                "source": source.map(|p| p.display().to_string()),
            })
        };
    let mut this_build = vec![
        option(
            localize(msg_str!("Continue")),
            msg_str!("Use this build's active save."),
            "continue",
            None,
            current.as_deref(),
        ),
        option(
            localize(msg_str!("Start fresh")),
            msg_str!("Begin a new game in a separate save profile."),
            "fresh",
            None,
            None,
        ),
    ];
    for (name, path) in saves::previous_saves(&project_data) {
        this_build.push(option(
            localize(&name),
            msg_str!("Starts from a copy of an earlier save."),
            "copy",
            Some(&path),
            Some(&path),
        ));
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
    for entry in std::fs::read_dir(&checkpoint_root)
        .into_iter()
        .flatten()
        .flatten()
    {
        let filename = entry.file_name();
        if let Some(name) = filename
            .to_str()
            .and_then(|name| name.strip_prefix("Checkpoint - "))
        {
            if entry.path().is_dir() {
                favorites.push((name.to_owned(), entry.path()));
            }
        }
    }
    favorites.sort();
    json!({
        "has_current": current.is_some(),
        "this_build": this_build,
        "other_builds": other_builds.iter().map(|(n, p)| option(n.clone(), msg_str!("Copies this build's active save."), "copy", Some(p), Some(p))).collect::<Vec<_>>(),
        "favorites": favorites.iter().map(|(n, p)| option(n.clone(), msg_str!("Starts from a copy of this checkpoint."), "copy", Some(p), None)).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod responsibility_tests {
    use super::known_responsibility;

    #[test]
    fn only_the_fixed_warning_ids_are_accepted() {
        assert_eq!(known_responsibility("import-repo"), Some("import-repo"));
        assert_eq!(known_responsibility("agent-access"), Some("agent-access"));
        assert_eq!(known_responsibility("agent-patch"), Some("agent-patch"));
        assert_eq!(known_responsibility(""), None);
        assert_eq!(known_responsibility("Import-Repo"), None);
        assert_eq!(known_responsibility("import-repo "), None);
        assert_eq!(known_responsibility("../settings"), None);
    }
}
