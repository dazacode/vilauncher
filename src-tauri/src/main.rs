//! VILauncher by the VIPortLive team: a community launcher for VIctory-based
//! projects that run an Xbox Series game on PC. Forked from Mandyhub by
//! Zettrox. This is the window: a Tauri shell around the web UI in ../src,
//! with the launcher (src/launcher.rs) behind three commands.
//!
//! No game files are included or downloaded: the player supplies their own.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backgrounds;
mod brand;
mod build_activity;
mod build_zip;
mod fullscreen;
mod launcher;
mod notify;
mod rust_project;
mod save_zip;
mod saves;
mod script_preset;
mod settings_store;
mod system;
mod tools;
mod updater;

use launcher::{Action, App};
pub(crate) use launcher::{default_workspace, hide_window};
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
fn act(action: Action, state: tauri::State<'_, Shared>, handle: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let mut app = state.app.lock().unwrap();
    let outcome = app.act(action);
    state.persist(&app);
    match outcome? {
        launcher::Outcome::Value(value) => Ok(value),
        launcher::Outcome::Exit => {
            handle.exit(0);
            Ok(serde_json::Value::Null)
        }
    }
}

#[tauri::command]
fn log(from: usize, state: tauri::State<'_, Shared>) -> launcher::LogView {
    state.app.lock().unwrap().log_from(from)
}

fn main() {
    // Development check without a window: the launcher's first snapshot, as JSON.
    #[cfg(debug_assertions)]
    if std::env::args().any(|a| a == "--dump-snapshot") {
        let mut app = App::new(notify::Ctx::new(|| {}), settings_store::load());
        for tries in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let snapshot = serde_json::to_value(app.snapshot()).unwrap();
            // The list, and the PC's look unless it takes almost all the time.
            if snapshot["list"]["status"] != "loading" && (!snapshot["system"].is_null() || tries >= 36) {
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
            let handle = tauri_app.handle().clone();
            let ctx = notify::Ctx::new(move || {
                let _ = handle.emit("launcher://changed", ());
            });
            let app = App::new(ctx, settings_store::load());
            tauri_app.manage(Shared { app: Mutex::new(app), written: Mutex::new(String::new()) });
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
        let _ = window_vibrancy::apply_vibrancy(window, window_vibrancy::NSVisualEffectMaterial::HudWindow, None, Some(12.0));
    }
    let _ = window;
}
