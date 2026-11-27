//! What differs between the systems the launcher runs on: hiding consoles,
//! finding the tools on a GUI's short `PATH`, opening links and folders,
//! ending programs, reading free space, and finding the game on other drives
//! or volumes.
//!
//! Everything the rest of the launcher calls lives here, so no other module
//! needs a `#[cfg]` for these jobs. The Windows behaviour is unchanged.

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Whether a spawned program gets a console window. Windows: never (a
/// detached console would flash); elsewhere a console is not a thing.
pub(crate) fn configure_background_command(c: &mut Command) {
    #[cfg(windows)]
    windows::hide_window(c);
    #[cfg(not(windows))]
    let _ = c;
}

/// The folders the build tools are installed to. Empty on Windows (its
/// installers write the `PATH` themselves).
pub(crate) fn tool_dirs() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        macos::tool_dirs()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// `PATH` with the tools' folders first, then what was already there.
/// Used by every spawned command, so a `PATH` changed after the launcher
/// started still reaches `git`, `cargo` and the game.
pub(crate) fn with_tool_dirs(existing: Option<std::ffi::OsString>) -> Option<std::ffi::OsString> {
    let mut dirs = tool_dirs();
    if let Some(path) = existing.as_ref() {
        dirs.extend(std::env::split_paths(path).filter(|d| !d.as_os_str().is_empty()));
    }
    if dirs.is_empty() {
        return existing;
    }
    dirs.dedup();
    std::env::join_paths(dirs).ok().or(existing)
}

/// Gives this process (and so every program it starts) the folders the
/// tools live in. macOS apps started from Finder, Spotlight or the Dock get
/// `/usr/bin:/bin:/usr/sbin:/sbin` only and would not find cargo, rustc or a
/// Homebrew git; the user's login shell knows better.
pub(crate) fn init_environment() {
    #[cfg(target_os = "macos")]
    {
        let current = std::env::var_os("PATH");
        if let Some(path) = macos::resolved_path(current.clone()) {
            std::env::set_var("PATH", path);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = std::env::var_os("PATH");
    }
}

/// Opens an `https` link in the player's browser.
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Explorer takes the link as one argument; no shell, so nothing in it is re-parsed.
        Command::new("explorer.exe")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open the link: {error}"))
    }
    #[cfg(target_os = "macos")]
    {
        macos::open_url(url)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open the link: {error}"))
    }
}

/// Opens a folder in the system's file manager.
pub(crate) fn open_folder(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open {}: {error}", path.display()))
    }
    #[cfg(target_os = "macos")]
    {
        macos::open_folder(path)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open {}: {error}", path.display()))
    }
}

/// Shows a file or folder selected in the file manager (Finder: "Reveal").
pub(crate) fn reveal(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::reveal(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows' Explorer selects with `/select,`; the folder itself is opened otherwise.
        if path.is_dir() {
            open_folder(path)
        } else {
            Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("Could not show {}: {error}", path.display()))
        }
    }
}

/// Free bytes on the disk holding `dir`. `None` when it cannot be read.
pub(crate) fn free_bytes(dir: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        windows::free_bytes(dir)
    }
    #[cfg(target_os = "macos")]
    {
        macos::free_bytes(dir)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = dir;
        None
    }
}

/// Ends every program running from a folder (the game and the runner), so an
/// uninstall or a rollback can delete it.
pub(crate) fn stop_programs_under(dir: &Path) {
    #[cfg(windows)]
    {
        windows::stop_programs_under(dir);
    }
    #[cfg(target_os = "macos")]
    {
        macos::stop_programs_under(dir);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = dir;
    }
}

/// Ends one program: a polite close first so the game can save, a forced one
/// seconds later if it is still there. Never blocks the caller.
pub(crate) fn stop_program(pid: u32) {
    #[cfg(windows)]
    {
        windows::stop_program(pid);
    }
    #[cfg(target_os = "macos")]
    {
        macos::stop_program(pid);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = pid;
    }
}

/// Whether another launcher window runs besides this process.
pub(crate) fn other_launcher_running(me: u32) -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows::other_launcher_running(me)
    }
    #[cfg(target_os = "macos")]
    {
        macos::other_launcher_running(me)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = me;
        Ok(false)
    }
}

/// A time as the player's clock shows it. `None` where it cannot be read.
pub(crate) fn local_time(seconds: u64) -> Option<String> {
    #[cfg(windows)]
    {
        crate::build_activity::local_time(seconds)
    }
    #[cfg(target_os = "macos")]
    {
        macos::local_time(seconds)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = seconds;
        None
    }
}

/// The roots a search for the game walks: drive letters on Windows, mounted
/// volumes and the home folder on macOS.
pub(crate) fn game_search_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows::game_search_roots()
    }
    #[cfg(target_os = "macos")]
    {
        macos::game_search_roots()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        vec![PathBuf::from("/")]
    }
}

/// Where a saved folder may be if its drive letter or volume name changed.
pub(crate) fn moved_folder_candidates(saved: &Path, tail: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows::moved_folder_candidates(saved, tail)
    }
    #[cfg(target_os = "macos")]
    {
        macos::moved_folder_candidates(saved, tail)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = saved;
        vec![PathBuf::from("/").join(tail)]
    }
}

/// Whether the disk a folder is on is connected. A relative folder counts as
/// connected.
pub(crate) fn drive_connected(dir: &Path) -> bool {
    #[cfg(windows)]
    {
        windows::drive_connected(dir)
    }
    #[cfg(target_os = "macos")]
    {
        macos::drive_connected(dir)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = dir;
        true
    }
}

/// The name a Cargo binary target gets on this system.
pub(crate) fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

/// What kind of program a file is: a Windows PE, a native Mach-O, a script, or
/// something else. Used to tell a player that a prebuilt ZIP is Windows-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binary {
    Windows,
    Native,
    Script,
    Unknown,
}

/// Reads the first bytes of `path`. Never panics; an unreadable file is `Unknown`.
pub(crate) fn binary_kind(path: &Path) -> Binary {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return Binary::Unknown;
    };
    let mut magic = [0u8; 4];
    let read = file.read(&mut magic).unwrap_or(0);
    if read >= 2 && &magic[..2] == b"MZ" {
        return Binary::Windows;
    }
    if read >= 2 && &magic[..2] == b"#!" {
        return Binary::Script;
    }
    if read == 4 {
        let m = u32::from_be_bytes(magic);
        // Mach-O thin (both byte orders, 32 and 64 bit) and universal ("fat") images.
        if matches!(
            m,
            0xfeed_face | 0xfeed_facf | 0xcefa_edfe | 0xcffa_edfe | 0xcafe_babe | 0xbeba_feca
        ) {
            return Binary::Native;
        }
    }
    if read == 4 {
        let m = u32::from_le_bytes(magic);
        if matches!(m, 0xfeed_face | 0xfeed_facf) {
            return Binary::Native;
        }
    }
    Binary::Unknown
}

/// Whether `path` is the program, a bundle, or a folder holding it: the thing
/// to run. A macOS `.app` runs its own program with the arguments and
/// environment the caller set, so its output is still captured.
pub(crate) fn program_for(path: &Path) -> (PathBuf, Option<PathBuf>) {
    if path.extension().is_some_and(|ext| ext == "app") && path.is_dir() {
        let macos = path.join("Contents").join("MacOS");
        if let Ok(entries) = std::fs::read_dir(&macos) {
            if let Some(found) = entries.flatten().map(|e| e.path()).find(|p| p.is_file()) {
                return (found, Some(macos));
            }
        }
    }
    (path.to_path_buf(), path.parent().map(Path::to_path_buf))
}

/// Whether the C++ build tools (Windows) or Xcode Command Line Tools
/// (macOS) are installed: Rust needs their linker.
pub(crate) fn build_tools_installed() -> bool {
    #[cfg(windows)]
    {
        windows::build_tools_installed()
    }
    #[cfg(target_os = "macos")]
    {
        macos::build_tools_installed()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // A linker from the system's packages is assumed present.
        true
    }
}

/// Whether `path` is something that can be started: a file, or a `.app` bundle.
pub(crate) fn is_program(path: &Path) -> bool {
    path.is_file() || (path.extension().is_some_and(|ext| ext == "app") && path.is_dir())
}

/// Adds the execute bit after unpacking, where the system has one.
pub(crate) fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut permissions = meta.permissions();
            permissions.set_mode(permissions.mode() | 0o755);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Clears macOS's quarantine flag from a freshly unpacked folder, where it is
/// set. Gatekeeper would otherwise refuse the game the first time.
pub(crate) fn clear_quarantine(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::clear_quarantine(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

/// Whether `path` still carries the quarantine flag.
pub(crate) fn is_quarantined(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_quarantined(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vilauncher-platform-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn binaries_are_recognised_by_their_first_bytes() {
        let dir = temp("kind");
        let pe = dir.join("a.exe");
        std::fs::write(&pe, b"MZ\x90\x00rest").unwrap();
        assert_eq!(binary_kind(&pe), Binary::Windows);
        let macho = dir.join("a");
        std::fs::write(&macho, [0xcf, 0xfa, 0xed, 0xfe, 0, 0, 0, 0]).unwrap();
        assert_eq!(binary_kind(&macho), Binary::Native);
        let fat = dir.join("b");
        std::fs::write(&fat, [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 0]).unwrap();
        assert_eq!(binary_kind(&fat), Binary::Native);
        let script = dir.join("c.sh");
        std::fs::write(&script, b"#!/bin/sh\n").unwrap();
        assert_eq!(binary_kind(&script), Binary::Script);
        let other = dir.join("d.txt");
        std::fs::write(&other, b"hello").unwrap();
        assert_eq!(binary_kind(&other), Binary::Unknown);
        assert_eq!(binary_kind(&dir.join("missing")), Binary::Unknown);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_bundle_runs_the_program_inside_it() {
        let dir = temp("bundle");
        let app = dir.join("Game.app");
        let macos = app.join("Contents").join("MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::write(macos.join("game"), b"x").unwrap();
        let (program, cwd) = program_for(&app);
        assert_eq!(program, macos.join("game"));
        assert_eq!(cwd, Some(macos));
        let plain = dir.join("victory");
        std::fs::write(&plain, b"x").unwrap();
        assert_eq!(program_for(&plain), (plain.clone(), Some(dir.clone())));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_exe_suffix_follows_the_system() {
        let name = exe_name("victory");
        if cfg!(windows) {
            assert_eq!(name, "victory.exe");
        } else {
            assert_eq!(name, "victory");
        }
    }
}
