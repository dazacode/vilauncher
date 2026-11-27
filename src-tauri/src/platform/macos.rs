//! macOS: the `PATH` a GUI app does not get, Finder (`open`), `kill`,
//! `df`, `/Volumes`, and the quarantine flag.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The folders the build tools are installed to. A window started from
/// Finder gets `/usr/bin:/bin:/usr/sbin:/sbin` only, so these are added to
/// every spawned command's `PATH` (see `platform::with_tool_dirs`).
pub(super) fn tool_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = home {
        dirs.push(home.join(".cargo").join("bin"));
        dirs.push(home.join(".local").join("bin"));
        dirs.push(
            home.join(".rustup")
                .join("toolchains")
                .join("stable-aarch64-apple-darwin")
                .join("bin"),
        );
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/opt/homebrew/sbin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/Library/Developer/CommandLineTools/usr/bin"));
    dirs.push(PathBuf::from("/usr/bin"));
    dirs.push(PathBuf::from("/bin"));
    dirs.retain(|dir| dir.is_dir());
    dirs
}

/// The user's login shell.
fn user_shell() -> PathBuf {
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|shell| shell.is_absolute() && shell.is_file())
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

/// Runs a short command and gives up after `limit`: a heavy shell start-up
/// must never hold the launcher's start.
fn output_within(mut command: Command, limit: Duration) -> Option<Output> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let output = command.output();
        let _ = send.send(output.ok());
    });
    receive.recv_timeout(limit).ok()??.into()
}

/// The `PATH` the user's login shell sets, as `-lc` (non-interactive: an
/// interactive shell waits for a TTY) with a hard two-second limit, then
/// `/usr/libexec/path_helper` when the shell printed nothing usable.
fn login_path() -> Option<OsString> {
    let mut shell = Command::new(user_shell());
    shell.args(["-lc", "printf %s \"$PATH\""]);
    if let Some(output) = output_within(shell, Duration::from_secs(2)) {
        if let Some(path) = path_line(&String::from_utf8_lossy(&output.stdout)) {
            return Some(OsString::from(path));
        }
    }
    // Apple's own tool: prints `PATH="..."; export PATH;`.
    let mut helper = Command::new("/usr/libexec/path_helper");
    helper.arg("-s");
    let output = output_within(helper, Duration::from_secs(2))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("PATH=\""))?
        .split('"')
        .next()?;
    path_line(line).map(OsString::from)
}

/// The `PATH` in a shell's output: start-up files sometimes print, so take
/// the last line that looks like a path list.
fn path_line(text: &str) -> Option<String> {
    let line = text
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.contains('/') && !line.contains(' '))?;
    let valid = line.split(':').filter(|part| part.starts_with('/')).count();
    (valid > 0).then(|| line.to_owned())
}

/// `PATH` with the tools' folders and the login shell's first, then the one
/// the app was started with.
pub(super) fn resolved_path(current: Option<OsString>) -> Option<OsString> {
    let mut dirs = tool_dirs();
    if let Some(login) = login_path() {
        dirs.extend(std::env::split_paths(&login));
    }
    if let Some(path) = current {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs.retain(|dir| dir.is_absolute());
    dirs.dedup();
    std::env::join_paths(dirs).ok()
}

/// Opens a link in the default browser.
pub(super) fn open_url(url: &str) -> Result<(), String> {
    Command::new("/usr/bin/open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the link: {error}"))
}

/// Opens a folder in Finder.
pub(super) fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("/usr/bin/open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open {}: {error}", path.display()))
}

/// Reveals a file or folder, selected, in Finder.
pub(super) fn reveal(path: &Path) -> Result<(), String> {
    Command::new("/usr/bin/open")
        .args(["-R"])
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not show {}: {error}", path.display()))
}

/// Free bytes on the volume holding `dir`, from `df`.
pub(super) fn free_bytes(dir: &Path) -> Option<u64> {
    let output = Command::new("/bin/df")
        .args(["-Pk"])
        .arg(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let available: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
    available.checked_mul(1024)
}

/// Every process whose program lives under `dir`.
fn pids_under(dir: &Path) -> Vec<u32> {
    let mut command = Command::new("/bin/ps");
    command.args(["-axo", "pid=,comm="]);
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    let folder = dir.to_string_lossy();
    let folder = folder.trim_end_matches('/');
    let me = std::process::id();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid: u32 = pid.parse().ok()?;
            let command = command.trim();
            (pid != me && (command == folder || command.starts_with(&format!("{folder}/"))))
                .then_some(pid)
        })
        .collect()
}

/// Ends every process running from `dir`, so it can be deleted.
pub(super) fn stop_programs_under(dir: &Path) {
    for pid in pids_under(dir) {
        signal(pid, "-TERM");
    }
    std::thread::sleep(Duration::from_millis(400));
    for pid in pids_under(dir) {
        signal(pid, "-KILL");
    }
}

fn signal(pid: u32, name: &str) {
    let _ = Command::new("/bin/kill")
        .args([name, &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// A polite close first, so the game can save; forced only if it is still
/// there a few seconds later.
pub(super) fn stop_program(pid: u32) {
    std::thread::spawn(move || {
        signal(pid, "-TERM");
        std::thread::sleep(Duration::from_secs(8));
        signal(pid, "-KILL");
    });
}

/// Whether another launcher window runs besides `me`.
pub(super) fn other_launcher_running(me: u32) -> Result<bool, String> {
    // `comm` is the program's path, so compare the file name only, and leave
    // this process out: a dump or a fix run must not count itself.
    let name = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_name().map(|name| name.to_owned()))
        .unwrap_or_default();
    let output = Command::new("/bin/ps")
        .args(["-A", "-o", "pid=,comm="])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| {
            "could not check whether VILauncher is running (ps failed), so Fix was not started"
                .to_string()
        })?;
    if !output.status.success() {
        return Err(
            "could not check whether VILauncher is running (ps failed), so Fix was not started"
                .into(),
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(|line| {
        let line = line.trim();
        let (pid, command) = match line.split_once(char::is_whitespace) {
            Some((pid, command)) => (pid, command.trim()),
            None => return false,
        };
        let same_name = Path::new(command)
            .file_name()
            .is_some_and(|found| found == name);
        same_name && pid.parse::<u32>().ok().is_some_and(|pid| pid != me)
    }))
}

/// A time as the player's clock shows it.
pub(super) fn local_time(seconds: u64) -> Option<String> {
    let output = Command::new("/bin/date")
        .args(["-r", &seconds.to_string(), "+%Y-%m-%d %H:%M"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The game is searched under the mounted volumes and the home folder.
pub(super) fn game_search_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/Volumes") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                roots.push(path);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home));
    }
    roots.push(PathBuf::from("/Users/Shared"));
    roots.retain(|root| root.is_dir());
    roots
}

/// The saved folder under every mounted volume, for a volume that came back
/// under another name (its first component is the volume's name).
pub(super) fn moved_folder_candidates(saved: &Path, tail: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/Volumes") {
        for entry in entries.flatten() {
            candidates.push(entry.path().join(tail));
        }
    }
    // The home folder, if the game lived outside a volume mount.
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if let Ok(rest) = saved.strip_prefix(&home) {
            candidates.push(home.join(rest));
        }
    }
    candidates
}

/// Whether the volume a folder is on is mounted. `/Volumes/<name>` must
/// exist; anything outside `/Volumes` is on the boot volume.
pub(super) fn drive_connected(dir: &Path) -> bool {
    let mut parts = dir.components();
    if parts.next().is_some_and(|c| c == Component::RootDir)
        && parts.next().is_some_and(|c| c.as_os_str() == "Volumes")
    {
        if let Some(volume) = parts.next() {
            return Path::new("/Volumes").join(volume).exists();
        }
    }
    true
}

/// Whether Xcode Command Line Tools are installed: `xcode-select -p` answers
/// with a path that holds the SDKs, and `clang` answers.
pub(super) fn build_tools_installed() -> bool {
    let output = Command::new("/usr/bin/xcode-select")
        .arg("-p")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // A path left behind by a removed install is not an install.
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !PathBuf::from(&path).is_dir() {
        return false;
    }
    // The full Xcode and the Command Line Tools keep the compiler in different
    // places, so ask the system for it rather than guess a folder.
    Command::new("/usr/bin/xcrun")
        .args(["--find", "clang"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|found| {
            found.status.success()
                && PathBuf::from(String::from_utf8_lossy(&found.stdout).trim()).is_file()
        })
        .unwrap_or(false)
}

/// Whether the quarantine flag is still on `path`.
pub(super) fn is_quarantined(path: &Path) -> bool {
    Command::new("/usr/bin/xattr")
        .args(["-p", "com.apple.quarantine"])
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Clears the quarantine flag from `path` and everything under it.
pub(super) fn clear_quarantine(path: &Path) -> bool {
    if !is_quarantined(path) {
        return false;
    }
    Command::new("/usr/bin/xattr")
        .args(["-d", "-r", "com.apple.quarantine"])
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_folders_are_all_absolute_and_existing() {
        for dir in tool_dirs() {
            assert!(dir.is_absolute(), "{}", dir.display());
            assert!(dir.is_dir(), "{}", dir.display());
        }
    }

    #[test]
    fn the_resolved_path_starts_with_the_tools_and_keeps_what_was_there() {
        let path = resolved_path(Some(OsString::from("/custom/bin"))).unwrap();
        let parts: Vec<PathBuf> = std::env::split_paths(&path).collect();
        assert!(parts.contains(&PathBuf::from("/usr/bin")));
        assert_eq!(parts.last(), Some(&PathBuf::from("/custom/bin")));
        assert!(parts.iter().all(|p| p.is_absolute()));
    }

    #[test]
    fn a_path_line_is_taken_from_noisy_shell_output() {
        let text = "Restored session: Nov 27\n/Users/someone/.cargo/bin:/usr/bin:/bin";
        assert_eq!(
            path_line(text).as_deref(),
            Some("/Users/someone/.cargo/bin:/usr/bin:/bin")
        );
        assert!(path_line("Welcome back!").is_none());
        assert!(path_line("").is_none());
    }

    #[test]
    fn a_folder_outside_volumes_is_on_the_boot_disk() {
        assert!(drive_connected(Path::new("/Users/someone/Games")));
        assert!(!drive_connected(Path::new(
            "/Volumes/No Such Volume Here/Games"
        )));
        assert!(drive_connected(Path::new("/Volumes")));
    }
}
