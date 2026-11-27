// The shell: sidebar, job card, window controls, launcher update notice (src/components, src/features/shared/launcher-update.tsx).
export default {
  /** The hidden look that seven clicks on the logo unlock. */
  unlocked: "A new look is unlocked. Find it in Settings, under Appearance.",
  /** The toasts' region, for screen readers. */
  notifications: "Notifications",
  nav: {
    games: "Games",
    workshop: "Workshop",
    label: "Pages",
    library: "Library",
    settings: "Settings",
  },
  search: "Search library",
  list: {
    title: "My library",
    fold: "Fold the library",
    unfold: "Show the library",
    label: "Projects",
    loadFailed: "The project list could not load.",
    noMatch: "No project matches “{query}”.",
    noneForCard: "No projects for this card yet.",
    clearSearch: "Clear search",
    showAllCards: "Show all cards",
    import: "Import a repo or build",
    sections: {
      custom: "Your repos",
      installed: "Installed",
      all: "All projects",
    },
  },
  filter: {
    button: "Filter and sort",
    buttonOn: { one: "Filter and sort, {count} on", other: "Filter and sort, {count} on" },
    showTakenDown: "Show taken-down projects",
    sort: "Sort",
    tier: "Tier",
    tiers: {
      all: "All",
      tracked: "Tracked",
      watchlist: "Watchlist",
    },
    cards: "Graphics cards",
    allCards: "All",
    advanced: "Advanced: by PC spec",
    reset: "Reset filters",
    specHint: "Builds are judged against these numbers. Leave a box empty to use what the launcher found.",
    ram: "System memory (GB)",
    vram: "Video memory (GB)",
  },
  /** The second line of a project's row in the sidebar. */
  row: {
    mayNotRun: "May not run well on this PC",
    badgeDown: "DOWN",
    badgeUpdate: "UPDATE",
    takenDown: "Taken down · open on VIPortLive",
    update: "Update available",
    imported: "Imported build",
    installedByLink: "Installed · added by link",
    byLink: "Added by link",
    notMeasured: "Not measured yet",
    verified: "Verified",
  },
  job: {
    log: "Build log",
    dismiss: "Dismiss",
  },
  window: {
    minimise: "Minimise",
    maximise: "Maximise",
    restore: "Restore",
  },
  /** A new release of VILauncher itself. */
  update: {
    out: "VILauncher {tag} is out",
    outHint: "Update now, or later from the sidebar.",
    notes: "Notes",
    downloading: "Downloading {tag}",
    ready: "{tag} is ready",
    restart: "Restart now",
  },
} as const
