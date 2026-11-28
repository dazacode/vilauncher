//! Linux (and the other Unix desktops the window builds for): `/proc` for
//! processes, the system C compiler for building, and the AppImage's
//! libraries kept away from the programs the launcher starts.

use std::path::Path;
use std::process::{Command, Stdio};

/// An AppImage puts its own libraries first in LD_LIBRARY_PATH. The programs the launcher
/// starts (git, cargo, the compiler) then load those copies, which do not match the
/// system's. The window's own libraries are already loaded, so the folders can go.
pub(super) fn keep_child_programs_out_of_the_appimage() {
    let Some(appdir) = std::env::var_os("APPDIR") else {
        return;
    };
    let appdir = appdir.to_string_lossy().into_owned();
    if let Some(path) = std::env::var_os("LD_LIBRARY_PATH") {
        let cleaned = without_folder(&path.to_string_lossy(), &appdir);
        if cleaned.is_empty() {
            std::env::remove_var("LD_LIBRARY_PATH");
        } else {
            std::env::set_var("LD_LIBRARY_PATH", cleaned);
        }
    }
}

/// `path` (a ':'-separated list) without the entries inside `folder`.
fn without_folder(path: &str, folder: &str) -> String {
    path.split(':')
        .filter(|entry| !entry.is_empty() && !entry.starts_with(folder))
        .collect::<Vec<_>>()
        .join(":")
}

/// Every process id in `/proc` but this process's.
fn other_pids(me: u32) -> Result<Vec<u32>, std::io::Error> {
    Ok(std::fs::read_dir("/proc")?
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| *pid != me)
        .collect())
}

/// Ends every process whose program lives under `dir`.
pub(super) fn stop_programs_under(dir: &Path) {
    for pid in other_pids(std::process::id()).unwrap_or_default() {
        let in_dir =
            std::fs::read_link(format!("/proc/{pid}/exe")).is_ok_and(|exe| exe.starts_with(dir));
        if in_dir {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
        }
    }
}

/// A polite close first (SIGTERM), so the game can save; forced only if it is
/// still there a few seconds later.
pub(super) fn stop_program(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(8));
        if Path::new(&format!("/proc/{pid}")).exists() {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    });
}

/// Whether a VILauncher other than `me` runs, by the processes' names.
pub(super) fn other_launcher_running(me: u32) -> Result<bool, String> {
    let pids = other_pids(me).map_err(|_| {
        "could not check whether VILauncher is running (/proc unreadable), so Fix was not started"
            .to_string()
    })?;
    Ok(pids.into_iter().any(|pid| {
        std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .is_ok_and(|name| name.trim() == "VILauncher")
    }))
}

/// Whether the system C compiler (`cc`) that Rust links with answers.
pub(super) fn build_tools_installed() -> bool {
    Command::new("cc")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// On some Linux graphics setups (NVIDIA on Wayland, AppImage bundles) WebKitGTK paints a
/// blank white window, because it draws through DMA-BUF buffers the driver cannot share.
/// The launcher's interface is light, so the renderer is turned off unless the player set it.
///
/// The renderer used instead loses whatever it does not repaint in a transparent window, so
/// only animated parts showed and the rest stayed white. That is why the Linux window is
/// opaque (`tauri.linux.conf.json`); keep it so while this renderer is off.
pub(super) fn avoid_blank_webview() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[cfg(test)]
mod tests {
    use super::without_folder;

    #[test]
    fn appimage_folders_are_taken_out_and_the_rest_kept() {
        let path = "/tmp/.mount_X/usr/lib:/usr/local/lib::/tmp/.mount_X/usr/lib64:/opt/x";
        assert_eq!(
            without_folder(path, "/tmp/.mount_X"),
            "/usr/local/lib:/opt/x"
        );
        assert_eq!(without_folder("/tmp/.mount_X/usr/lib", "/tmp/.mount_X"), "");
    }
}
