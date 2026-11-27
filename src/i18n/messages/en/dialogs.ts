// Dialogs (src/features/dialogs).
export default {
  import: {
    title: "Import a project",
    description: "Add a Rust project by its Git link, or a prebuilt Windows build from a ZIP.",
    what: "What to import",
    link: "Git link",
    /** The tab, and the file type in the file picker. */
    zip: "Build ZIP",
    add: "Add to library",
    added: "Added to your library",
    chooseZip: "Choose ZIP…",
    /** The file picker's title. */
    pickZip: "Import a build ZIP",
    hosts: "GitHub, GitLab, Codeberg, Gitea or your own server.",
    zipHint: "A ZIP with a Windows program and its files. Builds of VIctory (victory.exe) start with your game files; anything else starts on its own.",
    trust: "VILauncher builds and runs this code on your PC. Add only projects from people you trust: fake repos with malware exist.",
  },
  uninstall: {
    title: "Uninstall {name}?",
    titleUnknown: "Uninstall this project?",
    description: "This deletes its download, its build and its shader cache. Your saves stay, and you can install it again later.",
  },
  move: {
    title: "Move your installed projects?",
    description: "Your projects and their saves are in the old folder. Moving them to another drive can take a while.",
    move: "Move them",
    keep: "Don't move",
    from: "From",
    to: "To",
  },
  importError: "The build couldn't be imported",
  setup: {
    titlePlay: "Get ready to play",
    titleInstall: "Get ready to install",
    ready: "Everything is in place.",
    needed: "A few things are needed first. VILauncher can set most of them up for you.",
    notNow: "Not now",
    tools: {
      title: "Build tools",
      checking: "Checking for Git, Rust and the C++ build tools…",
      ready: "Git, Rust and the C++ build tools are ready.",
      missing: "Missing {tools}. Installed with winget; Windows asks for permission.",
      installing: "Installing",
    },
    files: {
      title: "Your game files",
      found: "Found in your game folder.",
      unplugged: "The drive with your game files is not connected. Plug it in, or choose the folder again.",
      missing: "Missing {files}",
      hint: "The folder with game_scarlett_master.exe from your own copy of the game.",
      /** The folder picker's title. */
      pick: "The folder with your game files",
    },
  },
  presets: {
    title: "How should it start?",
    description: "Choose once: VILauncher remembers it for this project.",
    current: "Current settings",
    currentDetail: "Keep this project's arguments and switches.",
  },
  saves: {
    title: "Choose a save",
    description: "New games and copies keep your earlier progress.",
    tabs: {
      label: "Saves",
      thisBuild: "This build",
      otherBuilds: "Other builds",
      favorites: "Favourites",
    },
    looking: "Looking for saves",
    emptyOther: "Saves from your other builds show here once you've saved in them.",
    emptyFavorites: "To keep a checkpoint, pick a save and choose Add to favourites.",
    /** A save's name in Screenshot mode. */
    example: "Example save {n}",
    fresh: "Start a new game",
    copy: "Play from a copy",
    favorite: "Add to favourites",
    favorited: "Added to favourites",
    checkpointName: "Checkpoint name",
    /** The launcher takes only these characters (ASCII letters and digits), so the example stays in English letters. */
    checkpointExample: "e.g. Intro complete",
    checkpointRule: "Letters, numbers, spaces, hyphens and underscores, up to 60.",
    saveCheckpoint: "Save checkpoint",
    confirmDelete: "Delete “{name}”? The files move to VILauncher's deleted-saves folder.",
    deleteSave: "Delete save",
    keep: "Keep it",
    deleted: "Moved to deleted-saves",
    /** The file picker's title and file type. */
    importZip: "Import a save ZIP",
    importType: "VIctory save ZIP",
    imported: "Imported “{name}” into favourites",
    /** The save dialog's title and file type. */
    exportTitle: "Export this save",
    exportType: "ZIP archive",
    exported: "Save exported",
    needFiles: "Choose your game files in Settings to play.",
  },
} as const
