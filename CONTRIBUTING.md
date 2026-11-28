# Contributing to VILauncher

Thank you for helping. VILauncher is free software under the GPL-3.0-or-later (see [LICENSE](LICENSE) and [LEGAL.md](LEGAL.md)). By contributing you agree that your contribution is released under the same licence.

## Before you start

- **Small fixes** (typos, a clear bug, a small UI fix): open a pull request directly.
- **Anything larger** (a new feature, a new screen, a change to how builds are run or updated): open an issue first and agree the approach. This saves you time.
- **Security problems**: do not open a public issue. See [SECURITY.md](SECURITY.md).

## Where things belong

- A **project** (its game, code, results or bugs) belongs to that project's authors. Report it to them.
- The **project list** (which projects appear, their details, takedowns) belongs to VIPortLive.
- **VILauncher itself** (the app, its builds, its warnings, its docs) belongs in this repository.

## Setting up

The launcher builds and runs on Windows, macOS and Linux, and CI checks all three. On every system you need Node.js 22, pnpm 10 and Rust stable, plus:

- **Windows 10 or 11:** Rust's MSVC toolchain and the Visual Studio C++ build tools. WebView2 ships with Windows.
- **macOS 13 or later:** Xcode Command Line Tools (`xcode-select --install`). For the universal app, also run `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. Building and signing on a Mac: [docs/MACOS.md](docs/MACOS.md#building-on-a-mac).
- **Linux:** a C compiler and the WebKitGTK 4.1 libraries Tauri links against. On Debian or Ubuntu, CI installs:

  ```sh
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev patchelf libssl-dev
  ```

Then:

```sh
git clone https://github.com/<your-account>/vilauncher
cd vilauncher
pnpm install
pnpm tauri dev
```

The UI also runs in a browser with a mock backend (`src/dev/mock.ts`), which is useful for layout work: run `pnpm dev` and open `http://localhost:1420`. Add `?platform=macos` or `?platform=windows` to the address to preview that system's labels and window.

## Before you open a pull request

Run the same checks CI runs:

```powershell
pnpm ci:check
```

For Rust changes also run `cargo fmt --check` and `cargo clippy --all-targets` in `src-tauri`. Both should report nothing. Add a test for any logic you change when a test is practical.

If you change a Tauri command or a settings field, update `src/lib/types.ts` and `src/dev/mock.ts` in the same pull request so the browser mock keeps working.

## Rules for the code

- **No personal data.** Do not commit names, paths, tokens, logs with private information, or game files. Do not add telemetry.
- **Respect the safety model.** The launcher runs other people's code, so new features must warn before they run code, change files outside the project's own folder, or act on the player's behalf. Saves must never be read, moved or deleted by update, rollback or cleanup code.
- **Keep warnings honest.** If a feature can cause harm, say what could happen in the UI and in the docs, and require the player to accept it.
- **Keep system differences in one place.** Code that differs between Windows, macOS and Linux goes in `src-tauri/src/platform/` (`mod.rs` dispatches to `windows.rs`, `macos.rs` and `linux.rs`), so other modules need no `#[cfg]`. A change must still build and test on all three.
- **Translate UI text.** The window speaks English, zh-TW and zh-CN. New UI text goes through `t` / `<Trans>` from `@/i18n`, and launcher messages that reach the window are marked `msg!` / `msg_str!` in Rust. Every language needs a translation, and `pnpm i18n:check` and `cargo test` fail if one is missing. See [docs/I18N.md](docs/I18N.md).
- **Dependencies** need a reason and a licence compatible with GPL-3.0-or-later. Say which in the pull request.
- **Generated files** (`tsconfig.tsbuildinfo`, `.ci/`, build output) do not belong in commits.

## Pull request process

1. Fork the repository and create a branch from `main`.
2. Make a focused change. One topic per pull request.
3. Fill in the pull request template.
4. Make sure `pnpm ci:check` passes.
5. A maintainer reviews. Expect questions; they are about the change, not about you.
6. Maintainers merge when the change is reviewed, checked and fits the project. We may ask for changes or decline a change that does not fit.

## Forks

You may fork VILauncher and build something of your own under the same licence. A fork must keep the licence and notices, credit the authors, and say what it changed (see LEGAL.md, section 6). If you publish a fork, please name it differently so players can tell the two apart.

## Conduct

Be kind and specific. Read the [Code of Conduct](CODE_OF_CONDUCT.md).
