# VILauncher

A Windows launcher for community VIctory projects, by the VIPortLive team.

VIctory projects are Rust recompilation runtimes that run the game from your own copy of its files. VILauncher lists the public ones from [viportlive.com](https://viportlive.com), downloads and builds the one you pick with Git and Cargo, and starts it with your game files.

No game files are included or downloaded. You need your own copy of the game.

## Download

Get `VILauncher.exe` (portable) or the installer from [Releases](https://github.com/dazacode/vilauncher/releases/latest). The launcher checks for its own updates at start and replaces itself after verifying the download's SHA-256 digest.

## Features

- **Library.** Every listed project with its reported frame rate, the graphics cards it was made for, whether it has been verified, and a check of whether it suits your PC's graphics card. Add any Rust project by its Git URL, or import a prebuilt build ZIP.
- **Tool setup.** Finds Git, Rust and the C++ build tools where their installers put them, and installs the missing ones with winget in one click (Windows asks for permission).
- **Install, update, play.** A project is downloaded and built once, then stays installed. Installed projects are checked for new commits at start and every 15 minutes. Uninstall removes the download, build and shader cache; saves stay.
- **Save picker.** Continue a save, start fresh, copy a save from another build, or start from a named checkpoint (each launch uses a copy). Import a save ZIP as a checkpoint, or export any save as a ZIP to share.
- **Launch presets.** Arguments and environment switches per project, saved as named presets you can export, import or paste as text. PowerShell launch scripts such as `play-scarlett.ps1` are read for their `VICTORY_*` defaults and offered as presets.
- **Graphics per project.** A global quality preset, and per-project overrides for console profile, render scale, output resolution and fixes.
- **Shader warm-up.** Builds the shader cache before the first launch on projects that support it.
- **Activity log.** The full Git and Cargo output of every install and build, selectable and exportable.
- **Screenshot mode.** Replaces repository names and links with examples and hides paths, notes and logs.
- **Glass window.** Mica on Windows 11, Acrylic on Windows 10.

## Requirements

- Windows 10 or 11, 64-bit
- WebView2, which ships with Windows 10 and 11 (the installer fetches it if missing)
- Your own copy of the game
- To build projects: Git, Rust and the C++ build tools (VILauncher can install them)

## Data folders

| Path | Holds |
| --- | --- |
| `%LOCALAPPDATA%\VILauncher` | Projects, builds and shader caches (the install folder can be changed in Settings), `backgrounds` |
| `%APPDATA%\VILauncher\settings.json` | Settings |

On first start VILauncher copies your Mandyhub settings and uses Mandyhub's installed projects where they are, so nothing is moved or built again.

Pictures (`.jpg`, `.png`) in `%LOCALAPPDATA%\VILauncher\backgrounds` are shown as a slow banner on each project's page.

## Build from source

Needs Node.js 22, pnpm 10 and Rust stable. The UI also runs in a plain browser with a mock backend during development.

```powershell
pnpm install
pnpm tauri dev     # run with hot reload
pnpm tauri build   # release build
```

Output:

- `src-tauri\target\release\VILauncher.exe`
- `src-tauri\target\release\bundle\nsis\VILauncher_<version>_x64-setup.exe`

Checks: `pnpm exec tsc -b` and `cargo test` in `src-tauri`. CI builds every push and pull request; a `v*` tag publishes a release with both files.

## Project layout

| Path | Contents |
| --- | --- |
| `src/` | React UI (TypeScript, Tailwind CSS) |
| `src/dev/mock.ts` | Mock backend for the browser |
| `src-tauri/` | Rust launcher (Tauri 2): project list, Git and Cargo jobs, saves, presets, updater |
| `src-tauri/src/brand.rs` | Names, links, folders and project list sources |

The project list comes from `https://viportlive.com/api/launcher`, with PortWatch's `tracker.json` as the fallback.

## Credits

- Forked from [Mandyhub](https://github.com/TheGameOnAddict/mandyhub-sinkfinger) by Zettrox
- Launcher redesign by ellion369
- Save picker, checkpoints, presets and build tracking by TheGameOnAddict
- VIctory by Marvisak and contributors
- Project list from PortWatch, via VIPortLive
- Fonts: Inter and Schibsted Grotesk (SIL Open Font License), Yellowtail (Apache License 2.0), via Fontsource

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
