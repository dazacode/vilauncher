//! What the window shows, in one snapshot: the window asks for it again
//! each time the launcher says something changed. Taking one also moves the
//! launcher along (finished jobs, update checks, the game folder check).

use super::*;
use crate::i18n::{localize, localize_opt};
use crate::updater::State as UpdaterState;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Snapshot {
    version: &'static str,
    /// The language the launcher's messages below are in: "en", "zh-TW" or "zh-CN".
    locale: &'static str,
    /// Whether the player agreed to the current legal notice.
    notice_accepted: bool,
    brand: BrandView,
    list: ListView,
    custom: Vec<ProjectView>,
    selected: Option<String>,
    settings: SettingsView,
    files: FilesView,
    tools: Option<ToolsView>,
    job: JobView,
    active_builds: usize,
    updater: UpdaterView,
    pending_move: Option<(String, String)>,
    presets: Vec<LaunchPreset>,
    backgrounds: Vec<String>,
    /// The player's PC; none while it is still being looked at.
    system: Option<crate::system::SystemInfo>,
}

#[derive(Serialize)]
struct BrandView {
    name: &'static str,
    /// Who makes it, translated ("VIPortLive team").
    team: String,
    site: &'static str,
    site_label: &'static str,
    source: &'static str,
    /// "windows", "macos" or the system's own name: the UI follows it for
    /// labels, shortcuts and window chrome.
    platform: &'static str,
}

#[derive(Serialize)]
struct ListView {
    /// "loading", "error" or "ready".
    status: &'static str,
    error: Option<String>,
    items: Vec<ProjectView>,
}

/// The keyboard-as-controller settings: on or off, and every control with its key.
#[derive(Serialize)]
pub(crate) struct ControllerKeysView {
    on: bool,
    controls: Vec<ControlView>,
}

#[derive(Serialize)]
pub(crate) struct ControlView {
    id: &'static str,
    label: &'static str,
    default_key: u16,
    key: u16,
}

fn controller_keys_view(saved: &super::Saved) -> ControllerKeysView {
    ControllerKeysView {
        on: saved.controller_keys_on,
        controls: crate::controller_keys::CONTROLS
            .iter()
            .map(|control| ControlView {
                id: control.id,
                label: control.label,
                default_key: control.default_key,
                key: crate::controller_keys::key_of(&saved.controller_key_bindings, control),
            })
            .collect(),
    }
}

#[derive(Serialize)]
pub(crate) struct ProjectView {
    name: String,
    repo: String,
    /// The GitHub account, for its picture; none for other hosts and imports.
    owner: Option<String>,
    gpu: Vec<String>,
    fps: Option<f64>,
    personal_fps: Option<f64>,
    playtime: Option<String>,
    build_job: Option<JobView>,
    render: Option<f64>,
    verified: bool,
    summary: String,
    /// What works, from the tracker.
    highlights: Vec<String>,
    /// What still stops it, from the tracker.
    blockers: Vec<String>,
    /// The memory the author says it uses, read from the words above.
    reported: reported::Reported,
    custom: bool,
    /// "git", "zip" or "local".
    kind: &'static str,
    /// The program's format: "windows", "native", "script" or "unknown". A
    /// Windows program needs a compatibility runner on a Mac.
    binary: Option<&'static str>,
    web_url: Option<String>,
    listing: super::Listing,
    installed: bool,
    checking: bool,
    built: Option<String>,
    /// A newer version online than the one installed (its commit), unless put off.
    update: Option<String>,
    /// The build before the last update, when one can be restored: its commit (empty if not recorded).
    previous_build: Option<String>,
    /// Runs on its own (a Rust program or a ZIP build), not through the game files.
    standalone: bool,
    /// Has a program to start once installed.
    runnable: bool,
    activity: Option<ActivityView>,
    launch: LaunchView,
}

#[derive(Serialize)]
struct ActivityView {
    /// "Updated 3 days ago" ("Installed …" for a first install), translated.
    updated_ago: String,
    /// Only how long ago: "3 days ago", translated.
    updated_when: String,
    pending: Option<String>,
    new_install: bool,
    last_run: Option<String>,
}

#[derive(Serialize)]
struct LaunchView {
    args: Option<String>,
    switches: Option<String>,
    script_options: Option<script_preset::Options>,
    /// Which script toggles are on in the switches.
    script_state: ScriptState,
    selected_preset: Option<String>,
    graphics: GraphicsSettings,
    graphics_override: bool,
    keep_game_settings: bool,
    notes: String,
}

#[derive(Serialize, Default)]
struct ScriptState {
    ray_tracing: bool,
    probe_rays: bool,
    hooks: bool,
}

#[derive(Serialize)]
struct SettingsView {
    game_dir: String,
    workspace: String,
    install_dir_custom: bool,
    warm_before_play: bool,
    warm_pipelines: bool,
    launch_args: String,
    extra_env: String,
    graphics: GraphicsSettings,
    gpu_filter: usize,
    show_nuked: bool,
    list_sort: usize,
    include_exact_60_fps: bool,
    list_tier: usize,
    spec_ram_gb: Option<f64>,
    spec_vram_gb: Option<f64>,
    backgrounds_dir: String,
    skip_update_warning: bool,
    agent_access: bool,
    controller_keys: ControllerKeysView,
    acknowledged: Vec<String>,
    /// The player's language choice; none: the system's.
    language: Option<String>,
}

#[derive(Serialize)]
struct FilesView {
    chosen: bool,
    ok: bool,
    empty: bool,
    unplugged: bool,
    missing: Vec<&'static str>,
    searching: bool,
    find_error: Option<String>,
}

#[derive(Serialize)]
struct ToolsView {
    ready: bool,
    /// The missing tools' names, translated ("C++ build tools").
    missing: Vec<String>,
}

#[derive(Serialize)]
struct JobView {
    running: bool,
    generation: u64,
    repo: Option<String>,
    /// What the job is doing, for the player (translated).
    stage: String,
    /// The stage as a fixed word the window can act on (see `stage_kind`).
    stage_kind: &'static str,
    failed: bool,
    stoppable: bool,
    log_lines: usize,
    import_error: Option<String>,
}

impl From<&Job> for JobView {
    fn from(job: &Job) -> Self {
        Self {
            running: job.running,
            generation: job.generation,
            repo: job.repo.clone(),
            stage: localize(&job.stage),
            stage_kind: stage_kind(&job.stage),
            failed: job.failed,
            stoppable: job.running && job.stoppable,
            log_lines: job.omitted_log_lines + job.log.len(),
            import_error: localize_opt(job.import_error.as_deref()),
        }
    }
}

#[derive(Serialize)]
struct UpdaterView {
    /// "idle", "checking", "current", "available", "downloading", "downloaded" or "failed".
    state: &'static str,
    tag: Option<String>,
    error: Option<String>,
    restart_error: Option<String>,
    releases: &'static str,
}

#[derive(Serialize)]
pub(crate) struct LogView {
    /// The number of the first line given, counting lines dropped from the buffer.
    pub(crate) from: usize,
    pub(crate) lines: Vec<String>,
    pub(crate) omitted: usize,
}

/// What a job's stage (as the launcher writes it, in English) is about, as a
/// fixed word: the window acts on it ("playing" shows Stop) and shortens it
/// ("building") without reading the stage's own words.
pub(crate) fn stage_kind(stage: &str) -> &'static str {
    match stage {
        "" => "none",
        "Playing" => "playing",
        s if s.starts_with("Downloading") => "downloading",
        s if s.starts_with("Building") => "building",
        s if s.starts_with("Warming") => "warming",
        s if s.starts_with("Installing tools") => "installing_tools",
        s if s.starts_with("Uninstalling") => "uninstalling",
        s if s.starts_with("Moving") => "moving",
        s if s.starts_with("Extracting") => "importing",
        _ => "other",
    }
}

/// A time as the player's clock shows it (none where the system cannot say).
fn local_time(seconds: u64) -> Option<String> {
    crate::platform::local_time(seconds)
}

impl App {
    /// Finished work taken in, then everything the window shows.
    pub(crate) fn snapshot(&mut self) -> Snapshot {
        self.tick();
        let projects = self.projects.lock().unwrap().clone();
        let (status, error, listed) = match projects {
            None => ("loading", None, Vec::new()),
            Some(Err(e)) => ("error", Some(e), Vec::new()),
            Some(Ok(list)) => ("ready", None, list),
        };
        let items = listed.iter().map(|p| self.project_view(p)).collect();
        let custom = self
            .saved
            .custom
            .clone()
            .iter()
            .map(|p| self.project_view(p))
            .collect();
        let job = JobView::from(&*self.job.lock().unwrap());
        let searching = matches!(&*self.finding.lock().unwrap(), Some(None));
        Snapshot {
            version: env!("CARGO_PKG_VERSION"),
            locale: crate::i18n::current().tag(),
            notice_accepted: self.saved.notice_accepted >= NOTICE_VERSION,
            brand: BrandView {
                name: brand::NAME,
                team: localize(brand::TEAM),
                site: brand::SITE,
                site_label: brand::SITE_LABEL,
                source: brand::SOURCE_URL,
                platform: std::env::consts::OS,
            },
            list: ListView {
                status,
                error: localize_opt(error.as_deref()),
                items,
            },
            custom,
            selected: self.saved.selected.clone(),
            settings: SettingsView {
                game_dir: self.saved.game_dir.clone(),
                workspace: workspace().display().to_string(),
                install_dir_custom: !self.saved.install_dir.is_empty(),
                warm_before_play: self.saved.warm_before_play,
                warm_pipelines: self.saved.warm_pipelines,
                launch_args: self.saved.launch_args.clone(),
                extra_env: self.saved.extra_env.clone(),
                graphics: self.saved.default_graphics(),
                gpu_filter: self.saved.gpu_filter.min(2),
                show_nuked: self.saved.show_nuked,
                list_sort: self.saved.list_sort.min(5),
                include_exact_60_fps: self.saved.include_exact_60_fps,
                list_tier: self.saved.list_tier.min(2),
                spec_ram_gb: self.saved.spec_ram_gb,
                spec_vram_gb: self.saved.spec_vram_gb,
                backgrounds_dir: crate::backgrounds::folder().display().to_string(),
                skip_update_warning: self.saved.skip_update_warning,
                agent_access: self.saved.agent_access,
                controller_keys: controller_keys_view(&self.saved),
                acknowledged: self.saved.acknowledged.clone(),
                language: self.saved.language.clone(),
            },
            files: FilesView {
                chosen: !self.saved.game_dir.trim().is_empty(),
                ok: self.files_ok(),
                empty: self.empty,
                unplugged: self.unplugged,
                missing: self.files.iter().filter(|f| !f.1).map(|f| f.0).collect(),
                searching,
                find_error: localize_opt(self.find_error.as_deref()),
            },
            tools: tools::current().map(|t| ToolsView {
                ready: t.ready(),
                missing: t.missing().into_iter().map(localize).collect(),
            }),
            job,
            active_builds: self.builds.running(),
            updater: self.updater_view(),
            pending_move: self
                .pending_move
                .as_ref()
                .map(|(a, b)| (a.display().to_string(), b.display().to_string())),
            presets: self.saved.launch_presets.clone(),
            backgrounds: crate::backgrounds::pictures()
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            system: self.system.lock().unwrap().clone(),
        }
    }

    /// The log from line `from` on (lines dropped from the buffer are skipped).
    pub(crate) fn log_from(&self, from: usize) -> LogView {
        self.project_log_from(None, from)
    }

    pub(crate) fn project_log_from(&self, repo: Option<&str>, from: usize) -> LogView {
        let job = match repo {
            Some(repo) => self.project_job(repo),
            None => Some(&self.job),
        };
        let Some(job) = job else {
            return LogView {
                from: 0,
                lines: Vec::new(),
                omitted: 0,
            };
        };
        let j = job.lock().unwrap();
        let start = from.max(j.omitted_log_lines);
        let skip = start - j.omitted_log_lines;
        LogView {
            from: start,
            lines: j.log.iter().skip(skip).cloned().collect(),
            omitted: j.omitted_log_lines,
        }
    }

    pub(crate) fn whole_log(&self, repo: Option<&str>) -> String {
        match repo {
            Some(repo) => self.project_job(repo),
            None => Some(&self.job),
        }
        .map(|job| job.lock().unwrap().log.join("\n"))
        .unwrap_or_default()
    }

    fn updater_view(&self) -> UpdaterView {
        let (state, tag, error) = match &self.updater.state {
            UpdaterState::Idle => ("idle", None, None),
            UpdaterState::Checking => ("checking", None, None),
            UpdaterState::Current => ("current", None, None),
            UpdaterState::Available(release) => ("available", Some(release.tag_name.clone()), None),
            UpdaterState::Downloading => ("downloading", None, None),
            UpdaterState::Downloaded(_) => ("downloaded", None, None),
            UpdaterState::Failed(error) => ("failed", None, Some(localize(error))),
        };
        UpdaterView {
            state,
            tag,
            error,
            restart_error: localize_opt(self.updater.restart_error.as_deref()),
            releases: updater::RELEASES,
        }
    }

    pub(crate) fn project_view(&self, p: &Project) -> ProjectView {
        let on_disk = self.installed_on_disk(p);
        let state = match self.state_of(&p.repo) {
            Some(state) => state,
            // The picked project and installed ones are looked at closely (and
            // online); the rest only on disk, so opening the list stays quiet.
            None if on_disk || self.saved.selected.as_deref() == Some(p.repo.as_str()) => {
                self.check_repo(&self.ctx, p);
                RepoState {
                    checking: true,
                    ..Default::default()
                }
            }
            None => RepoState::default(),
        };
        // While the first look runs, what is on disk already says installed or not.
        let installed = state.installed || (state.checking && on_disk);
        let standalone = p.runs_standalone();
        let runnable = !standalone
            || p.archive_exe.is_some()
            || rust_install(&p.repo).is_some_and(|i| i.executable.is_some());
        let key = project_key(&p.repo);
        let stats = self
            .saved
            .play_stats
            .profile(&key, self.saved.include_exact_60_fps);
        let launch = self.saved.project_launch.get(&key);
        let switches = launch.and_then(|l| l.switches.clone());
        let effective = switches
            .clone()
            .unwrap_or_else(|| self.saved.extra_env.clone());
        let script_state = ScriptState {
            ray_tracing: script_preset::enabled(&effective, script_preset::RAY_TRACING),
            probe_rays: script_preset::enabled(&effective, script_preset::PROBE_RAYS),
            hooks: script_preset::enabled(&effective, script_preset::HOOKS),
        };
        let binary = p
            .archive_program()
            .or_else(|| p.local_exe.as_deref().map(PathBuf::from))
            .map(|path| match crate::platform::binary_kind(&path) {
                crate::platform::Binary::Windows => "windows",
                crate::platform::Binary::Native => "native",
                crate::platform::Binary::Script => "script",
                crate::platform::Binary::Unknown => "unknown",
            });
        let kind = if p.archive_exe.is_some() {
            "zip"
        } else if p.local_exe.is_some() {
            "local"
        } else {
            "git"
        };
        let owner = (kind == "git" && !p.repo.contains("://"))
            .then(|| p.repo.split('/').next().unwrap_or_default().to_owned());
        let web = p.web_url();
        ProjectView {
            name: p.name.clone(),
            repo: p.repo.clone(),
            owner,
            gpu: p.gpu.clone(),
            fps: p.fps,
            personal_fps: stats.as_ref().and_then(|stats| stats.average_fps),
            playtime: stats.map(|stats| stats.playtime),
            build_job: self
                .builds
                .get(&p.repo)
                .map(|job| JobView::from(&*job.lock().unwrap())),
            render: p.render,
            verified: p.verified,
            summary: p.summary.clone(),
            highlights: p.highlights.clone(),
            blockers: p.blockers.clone(),
            reported: reported::parse(
                std::iter::once(p.summary.as_str())
                    .chain(p.highlights.iter().chain(&p.blockers).map(String::as_str)),
            ),
            custom: p.custom,
            kind,
            binary,
            web_url: (kind == "git" && (web.starts_with("https://") || web.starts_with("http://")))
                .then_some(web),
            listing: p.listing.clone(),
            installed,
            checking: state.checking,
            built: state.built.clone(),
            update: self.update_for(p),
            previous_build: previous::previous_commit(&previous_dir(&p.repo)),
            standalone,
            runnable,
            activity: state.activity.as_ref().map(|a| ActivityView {
                updated_ago: localize(&a.updated_ago()),
                updated_when: localize(&a.ago()),
                pending: localize_opt(a.pending_label()),
                new_install: a.new_install,
                last_run: a.last_run.and_then(local_time),
            }),
            launch: LaunchView {
                args: launch.and_then(|l| l.args.clone()),
                switches,
                script_options: launch.and_then(|l| l.script_options),
                script_state,
                selected_preset: launch.and_then(|l| l.selected_preset.clone()),
                graphics: self.saved.graphics_for(&p.repo),
                graphics_override: launch.is_some_and(|l| l.graphics.is_some()),
                keep_game_settings: launch.is_some_and(|l| l.keep_game_settings),
                notes: launch.map(|l| l.notes.clone()).unwrap_or_default(),
            },
        }
    }

    /// Whether `p` is installed, from disk, looked up again every 2 seconds.
    fn installed_on_disk(&self, p: &Project) -> bool {
        let mut cache = self.installed_cache.lock().unwrap();
        if let Some((at, installed)) = cache.get(&p.repo) {
            if at.elapsed().as_secs_f32() < 2.0 {
                return *installed;
            }
        }
        let installed = p
            .archive_program()
            .is_some_and(|exe| crate::platform::is_program(&exe))
            || p.local_exe
                .as_deref()
                .is_some_and(|exe| Path::new(exe).is_file())
            || (p.is_git() && project_installed(&p.repo));
        cache.insert(p.repo.clone(), (Instant::now(), installed));
        installed
    }

    /// What background work finished since the last look.
    fn tick(&mut self) {
        self.updater.poll();
        let builds: Vec<_> = self.builds.0.values().cloned().collect();
        for job in builds {
            self.apply_job_results(&job);
        }
        self.apply_job_results(&self.job.clone());
        let found = {
            let mut finding = self.finding.lock().unwrap();
            if matches!(*finding, Some(Some(_))) {
                finding.take().flatten()
            } else {
                None
            }
        };
        match found {
            Some(Ok(dir)) => {
                self.saved.game_dir = dir;
                self.files.clear();
                self.find_error = None;
            }
            Some(Err(e)) => self.find_error = Some(e),
            None => {}
        }
        self.check_files();
        let ctx = self.ctx.clone();
        self.auto_check(&ctx);
    }

    fn apply_job_results(&mut self, job: &Arc<Mutex<Job>>) {
        let (imported, removed, fixed_dir, adjusted, switches) = {
            let mut job = job.lock().unwrap();
            (
                job.imported_project.take(),
                job.removed_archive.take(),
                job.fixed_game_dir.take(),
                job.adjusted_graphics.take(),
                job.adjusted_switches.take(),
            )
        };
        if let Some(dir) = fixed_dir {
            self.saved.game_dir = dir;
            self.files.clear();
            self.files_checked = None;
        }
        if let Some((repo, graphics)) = adjusted {
            let launch = self
                .saved
                .project_launch
                .entry(project_key(&repo))
                .or_default();
            launch.graphics = Some(graphics);
        }
        if let Some((repo, pairs)) = switches {
            let launch = self
                .saved
                .project_launch
                .entry(project_key(&repo))
                .or_default();
            launch.switches = Some(merge_switches(
                launch.switches.as_deref().unwrap_or(""),
                &pairs,
            ));
        }
        if let Some(project) = imported {
            self.saved.selected = Some(project.repo.clone());
            self.saved.custom.insert(0, project);
        }
        if let Some(repo) = removed {
            self.saved.custom.retain(|project| project.repo != repo);
            if self.saved.selected.as_deref() == Some(&repo) {
                self.saved.selected = None;
            }
        }
    }
}
