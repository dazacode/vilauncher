// A stand-in launcher for working on the UI in a plain browser (pnpm dev).
// Never part of the app: api.ts loads it only outside the Tauri window.
// ?state=fresh (nothing set up) · ready (set up, one installed) · job (a build running).

import { isLocale, systemLocale } from "@/i18n/locales"
import type { Action, Graphics, LogView, Project, ResponsibilityId, Snapshot, StageKind, StorageView } from "@/lib/types"

// The controls VIctory reads from the keyboard, as src-tauri/src/controller_keys.rs lists them.
const CONTROLLER_CONTROLS: { id: string; label: string; default_key: number }[] = [
  { id: "menu", label: "Menu", default_key: 0x1b }, { id: "view", label: "View", default_key: 0x09 },
  { id: "a", label: "A", default_key: 0x0d }, { id: "b", label: "B", default_key: 0x08 },
  { id: "x", label: "X", default_key: 0x58 }, { id: "y", label: "Y", default_key: 0x59 },
  { id: "up", label: "D-pad up", default_key: 0x26 }, { id: "down", label: "D-pad down", default_key: 0x28 },
  { id: "left", label: "D-pad left", default_key: 0x25 }, { id: "right", label: "D-pad right", default_key: 0x27 },
  { id: "left_shoulder", label: "Left shoulder", default_key: 0x51 }, { id: "right_shoulder", label: "Right shoulder", default_key: 0x45 },
  { id: "left_stick_click", label: "Left stick click", default_key: 0x46 }, { id: "right_stick_click", label: "Right stick click", default_key: 0x48 },
  { id: "left_stick_up", label: "Left stick up", default_key: 0x57 }, { id: "left_stick_left", label: "Left stick left", default_key: 0x41 },
  { id: "left_stick_down", label: "Left stick down", default_key: 0x53 }, { id: "left_stick_right", label: "Left stick right", default_key: 0x44 },
  { id: "right_stick_up", label: "Right stick up", default_key: 0x49 }, { id: "right_stick_left", label: "Right stick left", default_key: 0x4a },
  { id: "right_stick_down", label: "Right stick down", default_key: 0x4b }, { id: "right_stick_right", label: "Right stick right", default_key: 0x4c },
  { id: "left_trigger", label: "Left trigger", default_key: 0x5a }, { id: "right_trigger", label: "Right trigger", default_key: 0x43 },
]

const SAMPLE: { name: string; repo: string; gpu: string[]; fps: number | null; render: number | null; verified: boolean; summary: string }[] = [{"name":"Kelvaroq","repo":"stellarr-studio/kelvaroq","gpu":["NVIDIA"],"fps":10,"render":0.25,"verified":false,"summary":"Kelvaroq is a public Rust runtime built on VIctory and victory-fork. It adds rendering patches and measured submission improvements, and runs the game through the prologue and first mission."},{"name":"Belobug","repo":"Belobug/VIctory-Belobug-AMD","gpu":["AMD"],"fps":22,"render":0.8,"verified":true,"summary":"Belobug's VIctory fork runs on AMD Radeon and is now very playable: about 22 fps on average with no crashes, using about 30 GB of RAM and 12 GB of video memory. Its lit open world plays through missions, with story progress rated 80%, level with CGBJ, and its picture is rated just below DexNom's. Its README hasn't caught up yet: it still reports 5-7 fps."},{"name":"DexNom","repo":"DexNom/VIctory","gpu":["AMD"],"fps":3,"render":0.85,"verified":true,"summary":"DexNom Runtime, a fork of VIctory for AMD Radeon, tested on a Radeon RX 7900 XTX. It builds on ZSlayerHQ's victory-world-runtime with its own fixes for shaders, texture streaming and GPU memory. PortWatch has seen it running with ray-traced lighting working, the first AMD build on our board to get it. Its picture is the best-rated on our board, though lighting and materials still have problems, and it plays through missions at about 3-4 fps (about 1 fps with ray tracing on), rarely crashing."},{"name":"Uilian","repo":"uiliannn/VItoriaBr","gpu":["NVIDIA"],"fps":4,"render":0.2,"verified":false,"summary":"Uilian's VItoriaBr, built on hyphennn's victory-fork with his own NVIDIA work and streamed live on YouTube. On stream it reaches Jason's first mission, \"Gone Fishing\", with poor rendering, up to 4 fps and few crashes."},{"name":"AvatarManz","repo":"avatarmanz/VIctory-7900XTX","gpu":["AMD"],"fps":16,"render":0.7,"verified":true,"summary":"AvatarManz's fork of VIctory, tuned for speed on an AMD Radeon RX 7900 XTX. It brings victory-fork's, DexNom's and Belobug's work together on VIctory and adds its own speed-ups: the game's draw calls translated on eight threads of its own, frames prepared on a separate thread, and textures uploaded on their own queue. PortWatch has verified it: the game runs at about 16 fps on average in game, with sound, as far into the story as the leading builds. Its README measures 13-22 fps indoors and on the first boat mission, and a 41-minute session without a crash."},{"name":"hyphennn","repo":"fxceless/victory-fork","gpu":["NVIDIA"],"fps":3,"render":0.7,"verified":false,"summary":"hyphennn's fork of VIctory (formerly victory-gta6), with his own rendering and speed fixes. On his Rumble streams the prologue plays with ray-traced lighting, detailed textures and hair at about 3 fps; the README still lists shadows as black. Self-reported."},{"name":"VIctory","repo":"Marvisak/VIctory","gpu":["AMD"],"fps":1,"render":0,"verified":false,"summary":"Marvis's Rust layer runs the game natively. The owner reports controlled gameplay is now reachable, though the build remains unstable; the README still says it is not playable and reports about 1 fps in the world."},{"name":"xbox-emulator","repo":"omonkeyz/xbox-emulator","gpu":[],"fps":0,"render":0,"verified":false,"summary":"Rust runtime that runs dumped Xbox Series builds as normal Windows processes, continuing the TomBox project. It reports Trackmania races driving at about 60 fps and Terraria reaching its world, plus Fortnite's lobby. Self-reported."},{"name":"XCL","repo":"xcl-dev/xcl","gpu":["AMD","NVIDIA"],"fps":0,"render":0,"verified":false,"summary":"Wine-style layer running game code natively. On its main branch the game gets past its menus into the world for several testers, but the screen stays on a loading spinner and a GPU watchdog ends the run. Unverified."},{"name":"ScarlettX","repo":"Freminet/ScarlettX","gpu":[],"fps":0,"render":0,"verified":false,"summary":"Clean-room high-level emulator of the Scarlett OS and GDK runtime. It reports two retail Xbox Series games, Concept Destruction and Unicorn Overlord, playable on Windows."},{"name":"TuxBox","repo":"brcly/TuxBox","gpu":["NVIDIA"],"fps":0,"render":0,"verified":false,"summary":"Native-first layer for unpacked Xbox Series games, tested mostly on Linux through Proton. the game reported its own state as \"Game Running\" in the story intro before crashing after about ten minutes, but no picture past the player-selection prompt is shown. Self-reported."},{"name":"OpenScarlett","repo":"californiaAlmonds/OpenScarlett","gpu":["NVIDIA","Intel"],"fps":0,"render":0,"verified":false,"summary":"MIT-licensed C++20 translation layer targeting D3D12 and Vulkan. It now claims 17 boot stops cleared on a retail title it doesn't name, with no run record or frame shown, so no the game milestone yet."},{"name":"scarlett-layer","repo":"jadeowaowa/scarlett-layer","gpu":["NVIDIA"],"fps":0,"render":0,"verified":false,"summary":"Loader plus injected hook DLL that redirects console-only calls to Windows shims, with its own d3d12_xs wrapper and shader lifter. It reports Terraria's GDK build rendering its menus with audio; no gameplay yet."},{"name":"Xbox2Windows","repo":"R0Y00/Xbox2Windows","gpu":["NVIDIA","AMD","Intel"],"fps":0,"render":0,"verified":false,"summary":"Static recompilation research turning Xbox Series code and shaders into Windows C++ and DXIL, with over 50 contributors. Footage the project shared shows the game's menus, unstable. Its README now reports a modified diagnostic exe playing the intro and setup text; the original exe and the menus remain unverified."}]

const params = new URLSearchParams(location.search)
const mode = params.get("state") ?? "ready"
const listeners = new Set<() => void>()
const changed = () => listeners.forEach((l) => l())

const graphics: Graphics = { series_s: true, render_scale: 0.5, fix_8gb: true, bright_shadows: true, fullscreen: false, output: "Default" }
const state = {
  selected: (params.get("select") ?? SAMPLE[1]?.repo ?? null) as string | null,
  gpu: 0,
  showNuked: false,
  listSort: 0,
  includeExact60Fps: false,
  resetPersonalFps: new Set<string>(),
  listTier: 0,
  specRam: null as number | null,
  specVram: null as number | null,
  installed: new Set(mode === "fresh" ? [] : [SAMPLE[1]?.repo, SAMPLE[4]?.repo]),
  gameDir: mode === "fresh" ? "" : "D:\\Games\\Scarlett",
  toolsReady: mode !== "fresh",
  job: (mode === "job" ? { repo: SAMPLE[0]?.repo ?? "", stage: "Building (the first time takes several minutes)" } : null) as null | { repo: string; stage: string },
  log: mode === "job" ? Array.from({ length: 40 }, (_, i) => `   Compiling crate-${i} v0.${i}.0`) : [] as string[],
  graphics,
  overrides: new Map<string, Graphics>(),
  keepGame: new Set<string>(),
  notes: new Map<string, string>(),
  warm: false,
  notice: params.get("notice") !== "0",
  skipUpdateWarning: false,
  agentAccess: false,
  controllerKeysOn: false,
  controllerBindings: {} as Record<string, number>,
  acknowledged: [] as ResponsibilityId[],
  // ?lang=zh-TW starts in that language, as if the player had chosen it.
  language: (isLocale(params.get("lang")) ? params.get("lang") : null) as string | null,
  systemLanguages: [] as string[],
}

/** As the launcher's `stage_kind` reads its (English) stages. */
function stageKind(stage: string): StageKind {
  if (!stage) return "none"
  if (stage === "Playing") return "playing"
  for (const [prefix, kind] of [["Downloading", "downloading"], ["Building", "building"], ["Warming", "warming"], ["Installing tools", "installing_tools"], ["Uninstalling", "uninstalling"], ["Moving", "moving"], ["Extracting", "importing"]] as const) {
    if (stage.startsWith(prefix)) return kind
  }
  return "other"
}

function project(s: (typeof SAMPLE)[number], index: number): Project {
  const installed = state.installed.has(s.repo)
  return {
    ...s,
    personal_fps: installed && !state.resetPersonalFps.has(s.repo) ? (state.includeExact60Fps ? 38.7 : 37.2) : null,
    playtime: installed ? "1h 18m" : null,
    build_job: null,
    highlights: index === 1 ? ["Open world renders lit, with textures, people and vehicles", "Plays through story missions without crashing"] : [],
    blockers: index === 1 ? ["About 3 fps in busy streets", "Needs 30 GB of system memory"] : [],
    listing: {
      id: `mock-${index}`,
      nuked: index === 2 ? "Taken down (mock)." : null,
      tier: index % 2 ? "tracked" : "watchlist",
      stars: 100 - index,
      commits7d: index * 3,
    },
    reported: index === 1 ? { ram_gb: 30, vram_gb: 12 } : { ram_gb: null, vram_gb: null },
    owner: s.repo.split("/")[0],
    custom: false,
    kind: "git",
    web_url: `https://github.com/${s.repo}`,
    installed,
    checking: false,
    built: installed ? "a1b2c3d4e5f6" : null,
    update: installed && index === 4 ? "f00dbabe1234" : null,
    previous_build: installed && index === 4 ? "9a8b7c6d5e4f" : null,
    standalone: false,
    runnable: true,
    activity: installed ? { updated_ago: "Updated 2 days ago", updated_when: "2 days ago", pending: null, new_install: false, last_run: "Yesterday, 21:14" } : null,
    launch: {
      args: null,
      switches: null,
      script_options: null,
      script_state: { ray_tracing: false, probe_rays: false, hooks: false },
      selected_preset: null,
      graphics: state.overrides.get(s.repo) ?? state.graphics,
      graphics_override: state.overrides.has(s.repo),
      keep_game_settings: state.keepGame.has(s.repo),
      notes: state.notes.get(s.repo) ?? "",
    },
  }
}

/** The team's name as the launcher sends it: translated, from each language's core.json in src-tauri/locales. */
const TEAM: Record<string, string> = { en: "VIPortLive team", "zh-TW": "VIPortLive 團隊", "zh-CN": "VIPortLive 团队" }

function snapshot(): Snapshot {
  const locale = state.language ?? systemLocale(state.systemLanguages)
  return {
    version: "0.1.0",
    locale,
    notice_accepted: state.notice,
    brand: { name: "VILauncher", team: TEAM[locale] ?? TEAM.en, site: "https://viportlive.com", site_label: "viportlive.com", source: "https://github.com/dazacode/vilauncher" },
    list: { status: "ready", error: null, items: SAMPLE.map(project) },
    custom: [],
    selected: state.selected,
    settings: {
      game_dir: state.gameDir,
      workspace: "C:\\Users\\you\\AppData\\Local\\VILauncher",
      install_dir_custom: false,
      warm_before_play: state.warm,
      warm_pipelines: false,
      launch_args: "",
      extra_env: "",
      graphics: state.graphics,
      gpu_filter: state.gpu,
      show_nuked: state.showNuked ?? false,
      list_sort: state.listSort ?? 0,
      include_exact_60_fps: state.includeExact60Fps,
      list_tier: state.listTier ?? 0,
      spec_ram_gb: state.specRam ?? null,
      spec_vram_gb: state.specVram ?? null,
      backgrounds_dir: "C:\\Users\\you\\AppData\\Local\\VILauncher\\backgrounds",
      skip_update_warning: state.skipUpdateWarning,
      agent_access: state.agentAccess,
      controller_keys: {
        on: state.controllerKeysOn,
        controls: CONTROLLER_CONTROLS.map((c) => ({ ...c, key: state.controllerBindings[c.id] ?? c.default_key })),
      },
      acknowledged: state.acknowledged,
      language: state.language,
    },
    files: { chosen: !!state.gameDir, ok: !!state.gameDir, empty: false, unplugged: false, missing: state.gameDir ? [] : [], searching: false, find_error: null },
    tools: { ready: state.toolsReady, missing: state.toolsReady ? [] : ["C++ build tools"] },
    active_builds: 0,
    job: {
      running: !!state.job,
      generation: 0,
      repo: state.job?.repo ?? null,
      stage: state.job?.stage ?? "",
      stage_kind: stageKind(state.job?.stage ?? ""),
      failed: false,
      stoppable: false,
      log_lines: state.log.length,
      import_error: null,
    },
    updater: { state: "current", tag: null, error: null, restart_error: null, releases: "https://github.com/dazacode/vilauncher/releases" },
    pending_move: null,
    presets: [],
    backgrounds: [],
    system: { gpus: [{ name: "NVIDIA GeForce RTX 4060", vendor: "NVIDIA", vram_gb: 8 }], ram_gb: 16, cpu: "AMD Ryzen 7 5800X 8-Core Processor", cpu_cores: 16 },
  }
}

async function act(a: Action): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 120))
  switch (a.type) {
    case "select":
      state.selected = a.repo
      break
    case "reset_personal_fps":
      state.resetPersonalFps.add(a.repo)
      break
    case "set_include_exact60_fps":
      state.includeExact60Fps = a.include
      break
    case "set_gpu_filter":
      state.gpu = a.value
      break
    case "set_list_options":
      state.showNuked = a.show_nuked
      state.listSort = a.sort
      state.listTier = a.tier
      break
    case "set_spec":
      state.specRam = a.ram_gb
      state.specVram = a.vram_gb
      break
    case "open_link":
      window.open(a.url, "_blank", "noopener")
      break
    case "report":
      return {
        title: `${a.repo}: mock report`,
        text: "## What happened\n\n## Log (mock)\n",
        launcher_url: "https://github.com/dazacode/vilauncher/issues/new",
        build_url: a.repo.split("/").length === 2 ? `https://github.com/${a.repo}/issues/new` : null,
      }
    case "install_tools":
      state.toolsReady = true
      break
    case "find_game_files":
    case "set_game_dir":
      state.gameDir = "D:\\Games\\Scarlett"
      break
    case "install":
      state.job = { repo: a.repo, stage: "Downloading the project" }
      state.log = ["== Downloading the project", "Cloning into 'repo'..."]
      setTimeout(() => {
        state.job = { repo: a.repo, stage: "Building (the first time takes several minutes)" }
        state.log.push("== Building", "   Compiling victory v0.1.0")
        changed()
      }, 1200)
      setTimeout(() => {
        state.job = null
        state.installed.add(a.repo)
        state.log.push("Finished release profile")
        changed()
      }, 3200)
      break
    case "play":
      return { next: "saves" }
    case "fix":
      state.job = { repo: a.repo, stage: "Fix and verify" }
      state.log = [
        "== Checking every known problem",
        "Fix and verify: **Known problem: about 30-40 s after the builder starts...**",
        "repo README.md:61 documents this problem and recommends VICTORY_BVH_BUILDER=0.",
        "Set VICTORY_BVH_BUILDER=0 from the project's own notes and read it back; saved for the next Play.",
        "Fix and verify: Some required game files are missing from the chosen folder.",
        "Searching for the game files from D:\\Games\\Scarlett onwards.",
        "Every required file is in D:\\Games\\Scarlett.",
        "Fix and verify: The D3D12 Agility SDK files are missing or not the expected version.",
        "Trying Microsoft's 1.620.2 package.",
        "checked D3D12\\D3D12Core.dll (70a6d47d…)",
        "Verified: 3 problem(s) fixed or checked; every check passed.",
      ]
      setTimeout(() => {
        state.job = null
        changed()
      }, 1600)
      break
    case "saves_list":
      return {
        has_current: true,
        this_build: [
          { title: "Continue", detail: "Use this build's active save.", kind: "continue", path: null, source: "x" },
          { title: "Start fresh", detail: "Begin a new game in a separate save profile.", kind: "fresh", path: null, source: null },
          { title: "Previous save 1", detail: "Starts from a copy of an earlier save.", kind: "copy", path: "p1", source: "p1" },
        ],
        other_builds: [],
        favorites: [{ title: "Intro complete", detail: "Starts from a copy of this checkpoint.", kind: "copy", path: "c1", source: null }],
      }
    case "set_default_graphics":
      state.graphics = a.graphics
      break
    case "set_project_keep_game_settings":
      if (a.value) state.keepGame.add(a.repo)
      else state.keepGame.delete(a.repo)
      break
    case "set_project_graphics":
      if (a.graphics) state.overrides.set(a.repo, a.graphics)
      else state.overrides.delete(a.repo)
      break
    case "set_project_notes":
      state.notes.set(a.repo, a.notes)
      break
    case "set_settings":
      if (a.patch.warm_before_play != null) state.warm = a.patch.warm_before_play
      break
    case "accept_notice":
      state.notice = true
      break
    case "set_language":
      state.language = a.language
      break
    case "set_system_languages":
      state.systemLanguages = a.languages
      break
    case "uninstall":
      state.installed.delete(a.repo)
      break
    case "rollback":
      break
    case "set_update_warning":
      state.skipUpdateWarning = !a.show
      break
    case "set_agent_access":
      state.agentAccess = a.enabled
      break
    case "set_controller_keys":
      state.controllerKeysOn = a.on
      break
    case "set_controller_key": {
      const control = CONTROLLER_CONTROLS.find((c) => c.id === a.control)
      if (!control) throw new Error(`There is no controller control ${a.control}.`)
      if (a.key === null) {
        delete state.controllerBindings[a.control]
        break
      }
      if (a.key === 0 || a.key === 0xff) throw new Error("That key cannot be used for a controller control.")
      const other = CONTROLLER_CONTROLS.find((c) => c.id !== a.control && (state.controllerBindings[c.id] ?? c.default_key) === a.key)
      if (other) throw new Error(`${other.label} already uses that key.`)
      state.controllerBindings[a.control] = a.key
      break
    }
    case "acknowledge":
      if (!state.acknowledged.includes(a.id)) state.acknowledged = [...state.acknowledged, a.id]
      break
    case "storage_scan": {
      const MB = 1024 * 1024
      const view: StorageView = {
        total_bytes: 9.4 * 1024 * MB,
        other_bytes: 0.2 * 1024 * MB,
        free_bytes: 212 * 1024 * MB,
        projects: [
          { slug: "Kelvaroq", repo: "stellarr-studio/kelvaroq", label: "Kelvaroq", sources: 0.6 * 1024 * MB, builds: 2.1 * 1024 * MB, previous: 1.9 * 1024 * MB, cache: 3.2 * 1024 * MB, saves: 14 * MB },
          { slug: "Belobug", repo: "Belobug/VIctory-Belobug-AMD", label: "Belobug", sources: 0.4 * 1024 * MB, builds: 0.9 * 1024 * MB, previous: 0, cache: 0, saves: 3 * MB },
        ],
      }
      return view
    }
    case "clear_cache":
    case "clear_previous":
      break
  }
  changed()
  return null
}

export const mockBackend = {
  snapshot: async () => snapshot(),
  act,
  log: async (from: number): Promise<LogView> => ({ from, lines: state.log.slice(from), omitted: 0 }),
  onChange: async (listener: () => void) => {
    listeners.add(listener)
    return () => listeners.delete(listener)
  },
}
