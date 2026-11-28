//! Windows: console hiding, `taskkill`/`tasklist`, drive letters, `fsutil`.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

/// Runs a program without its console window flashing up.
pub(super) fn hide_window(c: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    c.creation_flags(CREATE_NO_WINDOW);
}

/// The commit limit and the commit in use, from GetPerformanceInfo (psapi.dll).
pub(super) fn commit_charge() -> Option<super::CommitCharge> {
    use windows_sys::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
    let mut info: PERFORMANCE_INFORMATION = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32;
    info.cb = size;
    if unsafe { GetPerformanceInfo(&mut info, size) } == 0 {
        return None;
    }
    super::commit_from_pages(info.CommitLimit, info.CommitTotal, info.PageSize)
}

/// Free bytes on the volume holding `dir`, read with `fsutil`.
pub(super) fn free_bytes(dir: &Path) -> Option<u64> {
    let drive: String = dir
        .components()
        .take(2)
        .collect::<PathBuf>()
        .display()
        .to_string();
    let mut c = Command::new("fsutil");
    c.args(["volume", "diskfree", &drive]);
    hide_window(&mut c);
    let output = c.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let after = text.lines().next()?.split(':').nth(1)?;
    let digits: String = after
        .split('(')
        .next()?
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Ends every process whose program lives under `dir` (the game and the runner).
pub(super) fn stop_programs_under(dir: &Path) {
    let folder = dir.display().to_string().replace('\'', "''");
    let script = format!(
        "Get-Process | Where-Object {{ $_.Path -and $_.Path.StartsWith('{folder}', [StringComparison]::OrdinalIgnoreCase) }} | Stop-Process -Force -ErrorAction SilentlyContinue"
    );
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    hide_window(&mut c);
    let _ = c.status();
}

/// A window close first, so the game can save; forced only if it is still
/// running a few seconds later.
pub(super) fn stop_program(pid: u32) {
    std::thread::spawn(move || {
        let pid = pid.to_string();
        let mut close = Command::new("taskkill");
        close.args(["/PID", &pid, "/T"]);
        hide_window(&mut close);
        let _ = close.status();
        std::thread::sleep(std::time::Duration::from_secs(8));
        let mut force = Command::new("taskkill");
        force.args(["/PID", &pid, "/T", "/F"]);
        hide_window(&mut force);
        let _ = force.status();
    });
}

/// Whether a VILauncher other than `me` runs, from tasklist's CSV output.
pub(super) fn other_launcher_running(me: u32) -> Result<bool, String> {
    let mut command = Command::new("tasklist");
    command.args(["/FI", "IMAGENAME eq VILauncher.exe", "/FO", "CSV", "/NH"]);
    hide_window(&mut command);
    let output = command.output().map_err(|_| {
        "could not check whether VILauncher is running (tasklist failed), so Fix was not started"
            .to_string()
    })?;
    if !output.status.success() {
        return Err("could not check whether VILauncher is running (tasklist failed), so Fix was not started".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter_map(|line| line.split(',').nth(1))
        .filter_map(|pid| pid.trim().trim_matches('"').parse::<u32>().ok())
        .any(|pid| pid != me))
}

/// Every connected drive, C: to Z:.
pub(super) fn game_search_roots() -> Vec<PathBuf> {
    (b'C'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|root| root.is_dir())
        .collect()
}

/// The same folder on another drive (an external SSD that came back as F:
/// instead of E:), for a folder whose drive letter changed.
pub(super) fn moved_folder_candidates(_saved: &Path, tail: &Path) -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .map(|root| root.join(tail))
        .collect()
}

/// Whether the drive a folder is on is connected. A folder without a drive
/// letter counts as connected.
pub(super) fn drive_connected(dir: &Path) -> bool {
    match dir.components().next() {
        Some(Component::Prefix(_)) => dir.components().take(2).collect::<PathBuf>().exists(),
        _ => true,
    }
}

/// Whether a Visual Studio install (any edition, Build Tools too) has the
/// 64-bit C++ linker.
pub(super) fn build_tools_installed() -> bool {
    fn env_path(var: &str) -> Option<PathBuf> {
        std::env::var_os(var).map(PathBuf::from)
    }
    // Every Visual Studio install, from the installer's own list, else the usual folders.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(pf86) = env_path("ProgramFiles(x86)") {
        let vswhere = pf86
            .join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe");
        if vswhere.is_file() {
            let mut c = Command::new(vswhere);
            c.args(["-all", "-products", "*", "-property", "installationPath"]);
            hide_window(&mut c);
            if let Ok(o) = c.output() {
                roots.extend(
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(PathBuf::from),
                );
            }
        }
    }
    for base in [env_path("ProgramFiles"), env_path("ProgramFiles(x86)")]
        .into_iter()
        .flatten()
    {
        let vs = base.join("Microsoft Visual Studio");
        for year in std::fs::read_dir(&vs).into_iter().flatten().flatten() {
            for edition in std::fs::read_dir(year.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                roots.push(edition.path());
            }
        }
    }
    roots.iter().any(|root| {
        std::fs::read_dir(root.join("VC").join("Tools").join("MSVC"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|v| {
                v.path()
                    .join("bin")
                    .join("Hostx64")
                    .join("x64")
                    .join("link.exe")
                    .is_file()
            })
    })
}

/// Unused on Windows but kept so both modules have the same helpers.
#[allow(dead_code)]
pub(super) fn silence(c: &mut Command) {
    c.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
}
