# VILauncher: notes for agents

## Checking a build

Run the same checks as CI, locally, before you say a change works:

- `pnpm ci:check` runs typecheck, vite build and `cargo test --locked`, stopping at the first failure.
- `node scripts/ci.mjs --only typecheck,vite,test --json` runs a subset and writes `.ci/last-run.json`.
- `node scripts/ci.mjs --tauri --keep-going` also builds the bundle (the installer on Windows, the `.app` and `.dmg` on macOS, the AppImage and `.deb` on Linux), and runs every step.

The tree must build and test on Windows, macOS and Linux; CI runs all three. What differs between them lives in `src-tauri/src/platform/` (`mod.rs` dispatches to `windows.rs`, `macos.rs` and `linux.rs`), so other modules need no `#[cfg]` for those jobs. `cfg!(windows)` is fine for a single branch; code that names a system's own symbol needs a real `#[cfg]`.

On a failure, each failing step prints its last 40 lines with colour codes removed. Read that output first; `.ci/last-run.json` has the same lines for the failing step. Fix the first error, then rerun only the failing step with `--only`.

## Working tree

The tree often has other people's uncommitted edits. Do not revert or reformat files you did not change in this task, and do not commit unless asked.

## Languages

The window speaks English, zh-TW and zh-CN (docs/I18N.md). UI text goes through `t` / `<Trans>` from `@/i18n` with its English in `src/i18n/messages/en/<namespace>.ts` and a translation in every other language; launcher messages that reach the window are marked `msg!` / `msg_str!` in Rust and translated in `src-tauri/locales/*/*.json`. `pnpm i18n:check` and `cargo test` fail on a missing translation or English left in UI code. Logs, problem reports and the agent tool stay English.

## Layout

- `src/` React UI. `src/dev/mock.ts` is the browser mock backend; keep it in step with `src/lib/types.ts`.
- `src-tauri/src/` Rust launcher. The project list comes from `brand::PROJECT_SOURCES`; the list's options and the PC-spec override are saved settings (`launcher.rs`, actions in `launcher/actions.rs`).
- `src/lib/compat.ts` judges a build against the PC (yes / close / no); `withSpec` applies the player's own RAM and video memory.
- The window knows the system from `snapshot.brand.platform` (Rust's `std::env::consts::OS`: `windows`, `macos`, `linux`); the mock's `?platform=macos` previews macOS. macOS specifics (Gatekeeper, the updater, entitlements, signing) are in `docs/MACOS.md`.

## For AI agents

`node scripts/vilauncher-agent.mjs <command> [--json]` (or `pnpm agent`) is the only tool to use on the launcher's projects. Run `help` for the list.

- Read-only, always allowed: `status` (folders, agent access, tools, disk, projects, builds), `projects`, `log <repo>`, `patch-status <repo>`, `patch-export <repo> --out <file>` (writes only the new file, never overwrites).
- Changing: `fix <repo> --yes` (runs the launcher's Fix and verify) and `patch-apply <repo> <file> --yes` (git apply to the working tree, never commits). Both need the player's **AI agent access** setting on (`agent_access: true` in the launcher's `settings.json`: `%APPDATA%\VILauncher` on Windows, `~/Library/Application Support/VILauncher` on macOS, `~/.local/share/VILauncher` on Linux; missing means off). Without it they exit 1 and change nothing.
- Flow: `status` -> `log <repo>` -> make the change on a branch in `repos\<slug>` -> `patch-export <repo> --out <file>` -> `patch-apply <repo> <file> --yes` -> `fix <repo> --yes` -> `status` again.
- Exit codes: 0 ok, 1 refused, 2 usage error, 3 failure. Errors go to stderr.
- Changes run on the player's PC and are their responsibility. Show the patch to the player before `fix`.
- `fix` refuses while the launcher runs (ask the player to close it). It runs the launcher's `--fix <repo>` command from `src-tauri/target/release/VILauncher` (`VILauncher.exe` on Windows) if that exists, else the debug build. The launcher's `--fix` refuses too (exit 1) unless `agent_access` is true and no other VILauncher window runs. Build it with `cargo build --release` in `src-tauri`.
- The tool never reads or writes `data\` (the saves). It prints no secrets, env values or home-folder paths.
- When a job (Install, Update, Play, Fix and verify) ends, the launcher writes its output to `<workspace>\logs\<slug>.log` (header line with the time and stage, then the last 2000 lines, overwritten by each job, scrubbed of the player's name and folders). `log <repo>` prints that file, or says none exists yet. Never `data\`.
