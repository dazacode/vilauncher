// Shared labels and wording from src/lib (format.ts, compat.ts, theme.ts).
export default {
  /** Short job stages, by the launcher's stage_kind. */
  stage: {
    downloading: "Downloading",
    building: "Building",
    warming: "Warming up",
    installing_tools: "Installing tools",
    uninstalling: "Uninstalling",
    moving: "Moving",
    importing: "Importing",
    playing: "Playing",
  },
  /** Graphics modes (MODES in src/lib/format.ts). */
  mode: {
    fast: { label: "Fast", detail: "Half resolution, scaled up" },
    balanced: { label: "Balanced", detail: "Three quarters, scaled up" },
    best: { label: "Best quality", detail: "Full resolution" },
  },
  /** Output resolutions (OUTPUTS); the others are written as numbers ("1080p"). */
  output: {
    default: "Game default",
  },
  /** The launcher's looks (THEMES in src/lib/theme.ts). */
  theme: {
    dark: "Dark",
    light: "Light",
    calm: "Calm",
    paws: "Paws",
  },
  /** The project list's orders (SORTS in src/lib/compat.ts). */
  sort: {
    listing: "VIPortLive order",
    name: "Name",
    fps: "Frame rate",
    stars: "Stars",
    commits: "Recent commits",
  },
  /** How a build suits the player's PC (src/lib/compat.ts). {amount} is a size such as "8 GB". */
  compat: {
    gpu: "Graphics card",
    ram: "System memory",
    vram: "Video memory",
    madeForYours: "Made for {vendor} cards, like yours",
    madeFor: "Made for {cards} cards",
    noCards: "The author hasn't said which cards it runs on",
    noUsage: "No usage reported",
    reported: "The author reported about {amount}",
    above: "Above the {amount} the author reported",
    close: "Close to the {amount} the author reported",
    below: "Below the {amount} the author reported",
    headline: {
      card: "Made for {cards}: it may not start, or run slowly, on your card.",
      memory: "It may run slowly on your PC: it used more memory on the author's.",
      close: "Close to what your PC has: it may run a little slower than the author's.",
      good: "A good match for your PC, from what its author reported.",
      unknown: "Not enough is known yet to compare it with your PC.",
    },
    warning: {
      card: "Made for {cards} cards, not {vendor}",
      ram: "Used {used} of memory on the author's PC; yours has {have}",
      vram: "Used {used} of video memory on the author's PC; yours has {have}",
      close: "Close to your PC's limits: it may run a little slower",
    },
  },
} as const
