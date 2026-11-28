// The launcher's snapshot, as src-tauri/src/launcher/view.rs sends it.

export type OutputResolution = "Default" | "P720" | "P936" | "P1080" | "P1440" | "P2160"

export interface Graphics {
  series_s: boolean
  render_scale: number
  fix_8gb: boolean
  bright_shadows: boolean
  fullscreen: boolean
  output: OutputResolution
}

/** One control VIctory reads from the keyboard (src-tauri/src/controller_keys.rs). */
export interface ControllerControl {
  id: string
  label: string
  /** The Windows virtual-key code VIctory reads for it. */
  default_key: number
  /** The key the player chose, or the default. */
  key: number
}

export interface ControllerKeys {
  on: boolean
  controls: ControllerControl[]
}

export interface ScriptOptions {
  ray_tracing: boolean
  probe_rays: boolean
  hooks: boolean
}

export interface LaunchPreset {
  name: string
  args: string
  switches: string | null
  script_options: ScriptOptions | null
  projects?: string[]
  repo_script?: string | null
}

/** The player's PC, as the launcher found it (src-tauri/src/system.rs). */
export interface SystemInfo {
  gpus: { name: string; vendor: "NVIDIA" | "AMD" | "Intel" | "Apple" | "Other"; vram_gb: number | null }[]
  ram_gb: number | null
  cpu: string | null
  cpu_cores: number | null
  /** Running under Rosetta 2 (a translated process on Apple silicon). */
  translated?: boolean
  /** Windows only (null elsewhere): the most memory programs may commit (RAM plus page file), in GB. */
  commit_limit_gb?: number | null
  /** Windows only: the memory committed right now, in GB. */
  commit_used_gb?: number | null
}

/** A possible setting for this PC (src-tauri/src/known_problems.rs). `recommended` only for two-machine A/B evidence. */
export interface AdviceItem {
  id: string
  switch: string
  summary: string
  source: string
  recommended: boolean
}

export interface Project {
  name: string
  repo: string
  owner: string | null
  gpu: string[]
  fps: number | null
  personal_fps: number | null
  playtime: string | null
  build_job: Snapshot["job"] | null
  render: number | null
  verified: boolean
  summary: string
  /** What works, from the tracker. */
  highlights: string[]
  /** What still stops it, from the tracker. */
  blockers: string[]
  /** Possible settings for this PC (known_problems.rs); only Apply sets one. */
  advice: AdviceItem[]
  listing: {
    id: string | null
    /** Why VIPortLive took it down, when it did. */
    nuked: string | null
    tier: string | null
    stars: number | null
    commits7d: number | null
  }
  /** Memory the author reported it using, read from the summary; null when not said. */
  reported: { ram_gb: number | null; vram_gb: number | null; commit_gb: number | null }
  /** Whether the PC's commit limit is short of the author's peak commit plus the margin; null when either is not known. */
  commit_short: boolean | null
  custom: boolean
  kind: "git" | "zip" | "local"
  /** The program's format, when there is one: a Windows program needs a runner on a Mac. */
  binary: "windows" | "native" | "script" | "unknown" | null
  web_url: string | null
  installed: boolean
  checking: boolean
  built: string | null
  update: string | null
  /** The build before the last update, when it can be restored: its commit ("" if not recorded). */
  previous_build: string | null
  standalone: boolean
  runnable: boolean
  activity: {
    updated_ago: string
    /** Only how long ago ("3 days ago"), for a row already labelled Updated. */
    updated_when: string
    pending: string | null
    new_install: boolean
    last_run: string | null
    /** The build has not run since it was made: its first minutes are slow while shaders compile. */
    warm_up: boolean
  } | null
  launch: {
    args: string | null
    switches: string | null
    script_options: ScriptOptions | null
    script_state: ScriptOptions
    selected_preset: string | null
    graphics: Graphics
    graphics_override: boolean
    keep_game_settings: boolean
    notes: string
  }
}

export interface Snapshot {
  version: string
  /** The language the launcher's messages in this snapshot are in (src/i18n/locales.ts). */
  locale: string
  notice_accepted: boolean
  brand: { name: string; team: string; site: string; site_label: string; source: string; platform: "windows" | "macos" | string }
  list: { status: "loading" | "error" | "ready"; error: string | null; items: Project[] }
  custom: Project[]
  selected: string | null
  settings: {
    game_dir: string
    workspace: string
    install_dir_custom: boolean
    warm_before_play: boolean
    warm_pipelines: boolean
    launch_args: string
    extra_env: string
    graphics: Graphics
    nvidia: { busy: boolean; status: string | null; ready: boolean; got_mb: number; total_mb: number; dir: string }
    fsr_dll: string | null
    fsr_ok: boolean
    gpu_filter: number
    show_nuked: boolean
    /** 0 VIPortLive's order, 1 name, 2 frame rate, 3 stars, 4 recent commits. */
    list_sort: number
    include_exact_60_fps: boolean
    /** 0 all, 1 tracked, 2 watchlist. */
    list_tier: number
    spec_ram_gb: number | null
    spec_vram_gb: number | null
    backgrounds_dir: string
    skip_update_warning: boolean
    /** Off by default. Turning it on needs the "agent-access" warning first. */
    agent_access: boolean
    /** Warnings the player accepted. */
    acknowledged: ResponsibilityId[]
    /** The player's language ("en", "zh-TW", "zh-CN"); null follows the system. */
    language: string | null
    /** Off by default: the keyboard drives the game's controller controls while victory.exe is in front. */
    controller_keys: ControllerKeys
  }
  files: {
    chosen: boolean
    ok: boolean
    empty: boolean
    unplugged: boolean
    missing: string[]
    searching: boolean
    find_error: string | null
  }
  tools: { ready: boolean; missing: string[] } | null
  job: {
    running: boolean
    generation: number
    repo: string | null
    /** What the job is doing, in the player's language. */
    stage: string
    /** The stage as a fixed word to act on; `stage` is only for showing. */
    stage_kind: StageKind
    failed: boolean
    stoppable: boolean
    log_lines: number
    import_error: string | null
  }
  active_builds: number
  updater: {
    state: "idle" | "checking" | "current" | "available" | "downloading" | "downloaded" | "failed"
    tag: string | null
    error: string | null
    restart_error: string | null
    releases: string
  }
  pending_move: [string, string] | null
  presets: LaunchPreset[]
  backgrounds: string[]
  /** Null while the launcher is still looking. */
  system: SystemInfo | null
}

/** What a job's stage is about (src-tauri/src/launcher/view.rs, `stage_kind`). */
export type StageKind =
  | "none"
  | "playing"
  | "downloading"
  | "building"
  | "warming"
  | "installing_tools"
  | "uninstalling"
  | "moving"
  | "importing"
  | "other"

export interface LogView {
  from: number
  lines: string[]
  omitted: number
}

export interface SaveOption {
  title: string
  detail: string
  kind: "continue" | "fresh" | "copy"
  path: string | null
  source: string | null
}

export interface SavesList {
  has_current: boolean
  this_build: SaveOption[]
  other_builds: SaveOption[]
  favorites: SaveOption[]
}

export interface PlayChoice {
  options: { title: string; detail: string }[]
  recommended: number | null
}

export type PlayNext = { next: "presets"; choice: PlayChoice } | { next: "saves" } | { next: "started" }

export type SaveChoice = { kind: "continue" } | { kind: "fresh" } | { kind: "copy"; path: string }

/** Everything the window can ask the launcher to do (src-tauri/src/launcher/actions.rs). */
/** A problem report: the GitHub issue links and the full text (src-tauri/src/report.rs). */
export interface ReportView {
  title: string
  text: string
  launcher_url: string
  build_url: string | null
}

/** Disk use of the workspace (src-tauri/src/storage.rs). Sizes are in bytes. */
export interface StorageView {
  total_bytes: number
  other_bytes: number
  free_bytes: number | null
  projects: {
    slug: string
    /** The repo to pass back to clear_cache / clear_previous. */
    repo: string
    label: string
    sources: number
    builds: number
    previous: number
    cache: number
    /** Saves and profiles: kept, never cleared. */
    saves: number
  }[]
}

/** The warnings a player must accept before a risky step. Mirrors the Rust whitelist. */
export type ResponsibilityId = "import-repo" | "agent-access" | "agent-patch"

export type Action =
  | { type: "select"; repo: string | null }
  | { type: "refresh_projects" }
  | { type: "reset_personal_fps"; repo: string }
  | { type: "set_include_exact60_fps"; include: boolean }
  | { type: "set_gpu_filter"; value: number }
  | { type: "set_list_options"; show_nuked: boolean; sort: number; tier: number }
  | { type: "report"; repo: string }
  | { type: "diagnostic_summary"; repo: string }
  | { type: "open_link"; url: string }
  | { type: "set_spec"; ram_gb: number | null; vram_gb: number | null }
  | { type: "check_repo"; repo: string }
  | { type: "install"; repo: string }
  | { type: "later"; repo: string; remote: string }
  | { type: "play"; repo: string }
  | { type: "confirm_preset"; repo: string; index: number | null }
  | { type: "start_with_save"; repo: string; choice: SaveChoice }
  | { type: "warm"; repo: string }
  | { type: "fix"; repo: string }
  | { type: "clear_shader_cache"; repo: string }
  | { type: "rollback"; repo: string }
  | { type: "set_update_warning"; show: boolean }
  | { type: "set_agent_access"; enabled: boolean }
  | { type: "set_controller_keys"; on: boolean }
  | { type: "set_controller_key"; control: string; key: number | null }
  | { type: "acknowledge"; id: ResponsibilityId }
  | { type: "uninstall"; repo: string }
  | { type: "stop" }
  | { type: "remove_custom"; repo: string }
  | { type: "add_repo_link"; link: string }
  | { type: "import_build_zip"; path: string }
  | { type: "dismiss_import_error" }
  | { type: "install_tools" }
  | { type: "set_game_dir"; path: string }
  | { type: "find_game_files" }
  | {
      type: "set_settings"
      patch: Partial<{ warm_before_play: boolean; warm_pipelines: boolean; launch_args: string; extra_env: string }>
    }
  | { type: "set_default_graphics"; graphics: Graphics }
  | { type: "set_project_graphics"; repo: string; graphics: Graphics | null }
  | { type: "set_project_keep_game_settings"; repo: string; value: boolean }
  | { type: "set_project_args"; repo: string; value: string | null }
  | { type: "set_project_switches"; repo: string; value: string | null }
  | { type: "apply_advice"; repo: string; id: string }
  | { type: "set_script_option"; repo: string; option: keyof ScriptOptions; enabled: boolean }
  | { type: "set_project_notes"; repo: string; notes: string }
  | { type: "save_preset"; repo: string; name: string; include_switches: boolean }
  | { type: "apply_preset"; repo: string; name: string | null }
  | { type: "delete_preset"; repo: string; name: string }
  | { type: "import_preset_file"; repo: string; path: string }
  | { type: "import_preset_text"; repo: string; text: string }
  | { type: "export_preset"; name: string; path: string }
  | { type: "preset_text"; name: string }
  | { type: "detected_preset"; repo: string }
  | { type: "choose_install_dir"; path: string }
  | { type: "use_default_install_dir" }
  | { type: "confirm_move"; answer: "move" | "keep" | "cancel" }
  | { type: "open"; what: "project_folder" | "backgrounds" | "workspace" | "update"; repo?: string }
  | { type: "export_log"; path: string; repo?: string }
  | { type: "check_update" }
  | { type: "download_update" }
  | { type: "restart_to_update" }
  | { type: "saves_list"; repo: string }
  | { type: "checkpoint"; name: string; source: string }
  | { type: "import_save_zip"; path: string }
  | { type: "export_save"; source: string; path: string }
  | { type: "delete_save"; path: string }
  | { type: "accept_notice" }
  | { type: "storage_scan" }
  | { type: "clear_cache"; repo: string }
  | { type: "get_nvidia_files" }
  | { type: "set_fsr_dll"; path: string | null }
  | { type: "clear_previous"; repo: string }
  | { type: "set_language"; language: string | null }
  | { type: "set_system_languages"; languages: string[] }
