# VILauncher on macOS

VILauncher runs natively on macOS, on Apple silicon and Intel. This page covers what differs from Windows: the games, the build tools, Gatekeeper, the updater and the folders.

## Requirements

- macOS 13 (Ventura) or later. Releases ship one universal app for both processors.
- To build Rust projects: Xcode Command Line Tools, Git and Rust. **Settings → Install missing tools** starts Apple's installer (`xcode-select --install`), installs Git with Homebrew when Homebrew is there, and installs Rust with the official rustup script.

An app started from Finder, Spotlight or the Dock gets only `/usr/bin:/bin:/usr/sbin:/sbin` as its `PATH`. At start VILauncher reads your login shell's `PATH` and adds `~/.cargo/bin`, `/opt/homebrew/bin` and `/usr/local/bin`, so tools installed with rustup or Homebrew are found.

## The games

Prebuilt builds in the list are Windows programs. On macOS you can install them and keep their files, but running one needs a compatibility runner such as CrossOver or Whisky; the launcher says so before Play. Rust projects are built from source and run natively. The D3D12 Agility SDK checks and fixes are Windows ones and are skipped on macOS.

## Gatekeeper

macOS stops an app that was downloaded and is not notarized.

**A notarized release** (built with the Apple secrets set in CI, see below) opens normally. `spctl --assess --type execute --verbose=4 VILauncher.app` reports `source=Notarized Developer ID`.

**An ad-hoc signed release, first start.** macOS says the developer cannot be verified. Either:

1. System Settings → Privacy & Security → **Open Anyway**, then confirm, or
2. in Terminal: `xattr -dr com.apple.quarantine /Applications/VILauncher.app`

**A game build.** The launcher clears the quarantine flag on programs it unpacks, sets their execute bit, and offers **Fix and verify** when one is still blocked. `Bad CPU type in executable` means the build was made for the other processor only; it is not a quarantine problem.

## Verifying a download

Each release lists a SHA-256 for the `.dmg` and for `VILauncher-macos.app.tar.gz`. Compare before opening:

```sh
shasum -a 256 ~/Downloads/VILauncher_<version>_universal.dmg
codesign --verify --deep --strict /Applications/VILauncher.app
```

## The updater

- The release download is `VILauncher-macos.app.tar.gz`, the universal app. A release without it is not offered. Its SHA-256 is checked against the digest GitHub records when it is downloaded, and again before the swap.
- The archive is unpacked into a staging folder next to the running app and its program is checked. Only then is the old app moved aside and the new one put in its place; a failure puts the old app back.
- In a folder you can write to, no prompt is needed. In a read-only place (a mounted `.dmg`, an `/Applications` owned by another user) macOS asks for an administrator. If that is refused, the new app stays where it was staged and the launcher names the folder.
- Saves in `data` are never touched by an update.

## Folders

| Path | Holds |
| --- | --- |
| `~/Library/Application Support/VILauncher` | Repos, builds, shader caches, saves and profiles (`data`), previous builds, patches, logs. The install folder can be changed in Settings. |
| `~/Library/Application Support/VILauncher/settings.json` | Settings |

## Entitlements

`src-tauri/Entitlements.plist` is applied with the hardened runtime (`src-tauri/tauri.macos.conf.json`):

- `com.apple.security.cs.disable-library-validation`: the launcher starts programs it did not build (game builds, a Homebrew `git`), which are not signed by the same team.
- `com.apple.security.cs.allow-jit` and `com.apple.security.cs.allow-unsigned-executable-memory`: game builds and JIT-based tools map executable memory.
- `com.apple.security.network.client`: cloning projects and checking for updates.

## Signing and notarizing in CI

The **Build for macOS** job signs and notarizes when these repository secrets are set. Tauri reads them by these names:

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | base64 of a Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | its password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | the Apple ID used to notarize |
| `APPLE_PASSWORD` | an app-specific password for it |
| `APPLE_TEAM_ID` | the Team ID |

Without them the app is ad-hoc signed and not notarized, so forks and pull requests still build.

## Building on a Mac

```sh
pnpm install
pnpm tauri dev                                   # run with hot reload
pnpm tauri build                                 # this Mac's processor
pnpm tauri build --target universal-apple-darwin # both (needs both Rust targets)
```

The universal build needs `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. Output is in `src-tauri/target/<target>/release/bundle/` (`macos/VILauncher.app` and `dmg/`).
