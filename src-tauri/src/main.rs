//! VILauncher by the VIPortLive team: a community launcher for VIctory-based
//! projects that run an Xbox Series game on PC. Forked from Mandyhub by
//! Zettrox. This is the window: a Tauri shell around the web UI in ../src,
//! with the launcher (src/launcher.rs) behind three commands.
//!
//! No game files are included or downloaded: the player supplies their own.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// First, so its `msg!` and `msg_str!` are there for every module below.
#[macro_use]
mod i18n;
mod addons;
mod backgrounds;
mod brand;
mod build_activity;
mod build_zip;
mod controller_keys;
mod diagnose;
mod fullscreen;
mod job_log;
mod known_problems;
mod launcher;
mod notify;
mod patches;
mod platform;
mod play_stats;
mod previous;
mod report;
mod runner;
mod rust_project;
mod save_zip;
mod saves;
mod script_preset;
mod settings_store;
mod shader_cache;
mod streamline;
mod storage;
mod system;
mod tools;
mod updater;

pub(crate) use launcher::{default_workspace, hide_window};
use launcher::{Action, App};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

struct Shared {
    app: Mutex<App>,
    /// The settings as last written, so unchanged ones are not written again.
    written: Mutex<String>,
}

impl Shared {
    fn persist(&self, app: &App) {
        settings_store::save(&app.saved, &mut self.written.lock().unwrap());
    }
}

#[tauri::command]
fn snapshot(state: tauri::State<'_, Shared>) -> launcher::Snapshot {
    let mut app = state.app.lock().unwrap();
    let snapshot = app.snapshot();
    state.persist(&app);
    snapshot
}

#[tauri::command]
fn act(
    action: Action,
    state: tauri::State<'_, Shared>,
    handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let mut app = state.app.lock().unwrap();
    let outcome = app.act(action);
    state.persist(&app);
    // The launcher's messages are English; the window shows them in the player's language.
    match outcome.map_err(|error| i18n::localize(&error))? {
        launcher::Outcome::Value(value) => Ok(value),
        launcher::Outcome::Exit => {
            handle.exit(0);
            Ok(serde_json::Value::Null)
        }
    }
}

#[tauri::command]
fn log(from: usize, repo: Option<String>, state: tauri::State<'_, Shared>) -> launcher::LogView {
    state
        .app
        .lock()
        .unwrap()
        .project_log_from(repo.as_deref(), from)
}

fn main() {
    // Before anything else: Finder and the Dock give an app a bare PATH, and an
    // AppImage's libraries must not reach the programs the launcher starts.
    platform::init_environment();
    // Fix and verify without a window, for the agent tool (scripts\vilauncher-agent.mjs fix):
    // `--fix <repo>` prints the job's log and a STAGE line. Debug and release builds alike.
    // It refuses (exit 1) unless AI agent access is on, and while another launcher window runs.
    if let Some(at) = std::env::args().position(|a| a == "--fix") {
        let Some(repo) = std::env::args().nth(at + 1) else {
            eprintln!("--fix needs a repo");
            return;
        };
        if let Err(reason) = fix_allowed() {
            eprintln!("refused: {reason}");
            std::process::exit(1);
        }
        let mut app = App::new(notify::Ctx::new(|| {}), settings_store::load());
        for _ in 0..100 {
            if app.list_ready() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        if let Err(error) = app.act(Action::Fix { repo }) {
            eprintln!("could not start the fix: {error}");
            return;
        }
        loop {
            let _ = app.snapshot();
            let (running, stage) = app.job_running();
            if !running {
                let _ = app.snapshot();
                settings_store::save(&app.saved, &mut String::new());
                println!("STAGE: {stage}");
                println!("{}", app.log_from(0).lines.join("\n"));
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    // Check without a window: the launcher's first snapshot, as JSON.
    if std::env::args().any(|a| a == "--dump-snapshot") {
        let mut app = App::new(notify::Ctx::new(|| {}), settings_store::load());
        for tries in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let snapshot = serde_json::to_value(app.snapshot()).unwrap();
            // The list, and the PC's look unless it takes almost all the time.
            if snapshot["list"]["status"] != "loading"
                && (!snapshot["system"].is_null() || tries >= 36)
            {
                println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
                return;
            }
        }
        eprintln!("the project list did not arrive in 10 s");
        return;
    }
    launcher::migrate_old_folders();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|tauri_app| {
            // WKWebView has no edit shortcuts of its own: Cmd+C/V/X/A/Z and
            // Cmd+Q/H/W only work through a native menu with these items.
            #[cfg(target_os = "macos")]
            install_macos_menu(&tauri_app.handle().clone())?;
            let handle = tauri_app.handle().clone();
            let ctx = notify::Ctx::new(move || {
                let _ = handle.emit("launcher://changed", ());
            });
            let app = App::new(ctx, settings_store::load());
            tauri_app.manage(Shared {
                app: Mutex::new(app),
                written: Mutex::new(String::new()),
            });
            if let Some(window) = tauri_app.get_webview_window("main") {
                glass(&window);
            }
            // The launcher looks for updates of installed projects on its own, every few minutes.
            let handle = tauri_app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let state = handle.state::<Shared>();
                let mut app = state.app.lock().unwrap();
                let ctx = app.ctx.clone();
                app.auto_check(&ctx);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![snapshot, act, log])
        .run(tauri::generate_context!())
        .expect("VILauncher could not start");
}

/// Why `--fix` may not run now, or `Ok` when it may: AI agent access must be on in
/// settings.json, and no other VILauncher.exe (the window) may be running.
fn fix_allowed() -> Result<(), String> {
    let text = std::fs::read_to_string(settings_store::file()).unwrap_or_default();
    let settings: serde_json::Value =
        serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if settings.get("agent_access") != Some(&serde_json::Value::Bool(true)) {
        return Err("AI agent access is off. Ask the player to turn on 'AI agent access' in VILauncher Settings. Nothing was changed.".into());
    }
    if other_launcher_running()? {
        return Err(
            "VILauncher is running. Close it first: Fix may need the files it holds.".into(),
        );
    }
    Ok(())
}

/// The macOS application menu: App (about, services, hide, quit), Edit
/// (undo, redo, cut, copy, paste, select all) and Window (minimise, zoom,
/// close). Without it, the standard keyboard shortcuts do nothing.
#[cfg(target_os = "macos")]
fn install_macos_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{AboutMetadata, Menu, PredefinedMenuItem, Submenu};

    let name = brand::NAME;
    let app_menu = Submenu::with_items(
        app,
        name,
        true,
        &[
            &PredefinedMenuItem::about(
                app,
                Some(&format!("About {name}")),
                Some(AboutMetadata::default()),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some(&format!("Hide {name}")))?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some(&format!("Quit {name}")))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, Some("Zoom"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let menu = Menu::with_items(app, &[&app_menu, &edit, &window])?;
    app.set_menu(menu)?;
    Ok(())
}

/// Whether another launcher runs besides this process (tasklist on Windows, ps on macOS,
/// /proc on Linux).
/// A `--dump-snapshot` run is a check, not a window, so it never counts as one.
fn other_launcher_running() -> Result<bool, String> {
    if std::env::args().any(|a| a == "--dump-snapshot") {
        return Ok(false);
    }
    platform::other_launcher_running(std::process::id())
}

/// The system's own glass behind the window: Mica on Windows 11 (Acrylic on
/// Windows 10), the sidebar material on macOS. The UI keeps its own dark
/// ground where none is available.
fn glass(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        if window_vibrancy::apply_mica(window, Some(true)).is_err() {
            let _ = window_vibrancy::apply_acrylic(window, Some((12, 10, 18, 200)));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window_vibrancy::apply_vibrancy(
            window,
            window_vibrancy::NSVisualEffectMaterial::HudWindow,
            None,
            Some(12.0),
        );
    }
    let _ = window;
}
