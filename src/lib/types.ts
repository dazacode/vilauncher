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
}

export interface Project {
  name: string
  repo: string
  owner: string | null
  gpu: string[]
  fps: number | null
  render: number | null
  verified: boolean
  summary: string
  /** What works, from the tracker. */
  highlights: string[]
  /** What still stops it, from the tracker. */
  blockers: string[]
  /** Memory the author reported it using, read from the summary; null when not said. */
  reported: { ram_gb: number | null; vram_gb: number | null }
  custom: boolean
  kind: "git" | "zip" | "local"
  web_url: string | null
  installed: boolean
  checking: boolean
  built: string | null
  update: string | null
  standalone: boolean
  runnable: boolean
  activity: {
    updated_ago: string
    pending: string | null
    new_install: boolean
    last_run: string | null
  } | null
  launch: {
    args: string | null
    switches: string | null
    script_options: ScriptOptions | null
    script_state: ScriptOptions
    selected_preset: string | null
    graphics: Graphics
    graphics_override: boolean
    notes: string
  }
}

export interface Snapshot {
  version: string
  notice_accepted: boolean
  brand: { name: string; team: string; site: string; site_label: string; source: string }
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
    gpu_filter: number
    backgrounds_dir: string
  }
  files: {
    chosen: boolean
    ok: boolean
    empty: boolean
    missing: string[]
    searching: boolean
    find_error: string | null
  }
  tools: { ready: boolean; missing: string[] } | null
  job: {
    running: boolean
    repo: string | null
    stage: string
    failed: boolean
    stoppable: boolean
    log_lines: number
    import_error: string | null
  }
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
export type Action =
  | { type: "select"; repo: string | null }
  | { type: "refresh_projects" }
  | { type: "set_gpu_filter"; value: number }
  | { type: "check_repo"; repo: string }
  | { type: "install"; repo: string }
  | { type: "later"; repo: string; remote: string }
  | { type: "play"; repo: string }
  | { type: "confirm_preset"; repo: string; index: number | null }
  | { type: "start_with_save"; repo: string; choice: SaveChoice }
  | { type: "warm"; repo: string }
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
  | { type: "set_project_args"; repo: string; value: string | null }
  | { type: "set_project_switches"; repo: string; value: string | null }
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
  | { type: "export_log"; path: string }
  | { type: "check_update" }
  | { type: "download_update" }
  | { type: "restart_to_update" }
  | { type: "saves_list"; repo: string }
  | { type: "checkpoint"; name: string; source: string }
  | { type: "import_save_zip"; path: string }
  | { type: "export_save"; source: string; path: string }
  | { type: "delete_save"; path: string }
  | { type: "accept_notice" }
