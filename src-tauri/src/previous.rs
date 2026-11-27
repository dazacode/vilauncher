//! The build an update replaced, kept so the player can go back to it once.
//!
//! Everything lives in one folder (workspace\previous\<slug>\): the old exe under
//! its own file name, the D3D12 folder next to it when there was one,
//! `built-from.txt` (the old commit) and `exe-path.txt` (where the exe belongs).
//! The functions take paths only; they never touch the saves or the repos.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const COMMIT: &str = "built-from.txt";
const EXE_PATH: &str = "exe-path.txt";
const D3D12: &str = "D3D12";

/// Moves each (from, to) pair in order. If one fails, the moves already made are undone.
fn move_all(pairs: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    let mut done: Vec<&(PathBuf, PathBuf)> = Vec::new();
    for pair in pairs {
        if let Err(error) = fs::rename(&pair.0, &pair.1) {
            for (from, to) in done.iter().rev() {
                let _ = fs::rename(to, from);
            }
            return Err(format!(
                "could not move {} to {}: {error}",
                pair.0.display(),
                pair.1.display()
            ));
        }
        done.push(pair);
    }
    Ok(())
}

fn read_text(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|error| format!("could not write {}: {error}", path.display()))
}

/// The exe the kept build goes back to, from `exe-path.txt`.
fn kept_exe_path(previous: &Path) -> Option<PathBuf> {
    read_text(&previous.join(EXE_PATH))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

/// The kept exe's file in `previous`, named as the original exe is.
fn kept_file(previous: &Path, exe: &Path) -> Option<PathBuf> {
    Some(previous.join(exe.file_name()?))
}

fn suffixed(name: &OsString, suffix: &str) -> OsString {
    let mut owned = name.clone();
    owned.push(suffix);
    owned
}

/// Before an update's build starts: keeps the installed build at `exe` (with its
/// D3D12 folder and the commit in `built_from`) in `previous`, replacing any older
/// kept build. Returns false, and keeps nothing, when no build is installed.
pub(crate) fn keep_for_update(
    exe: &Path,
    built_from: &Path,
    previous: &Path,
) -> Result<bool, String> {
    clear(previous)?;
    if !exe.is_file() {
        return Ok(false);
    }
    let name = exe
        .file_name()
        .ok_or_else(|| format!("{} has no file name", exe.display()))?;
    fs::create_dir_all(previous)
        .map_err(|error| format!("could not make {}: {error}", previous.display()))?;
    let commit = read_text(built_from).unwrap_or_default();
    write_text(&previous.join(COMMIT), &commit)?;
    write_text(&previous.join(EXE_PATH), &exe.display().to_string())?;
    let mut pairs = vec![(exe.to_path_buf(), previous.join(name))];
    let d3d12 = exe.with_file_name(D3D12);
    if d3d12.is_dir() {
        pairs.push((d3d12, previous.join(D3D12)));
    }
    if let Err(error) = move_all(&pairs) {
        clear(previous)?;
        return Err(error);
    }
    Ok(true)
}

/// After a failed update build: removes what the failed build left at the exe's
/// place and puts the kept build back, so the old version is installed again.
/// Does nothing when no build was kept.
pub(crate) fn restore_failed(previous: &Path) -> Result<(), String> {
    let Some(exe) = kept_exe_path(previous) else {
        return clear(previous);
    };
    let Some(kept) = kept_file(previous, &exe) else {
        return clear(previous);
    };
    if exe.is_file() {
        fs::remove_file(&exe)
            .map_err(|error| format!("could not remove {}: {error}", exe.display()))?;
    }
    let d3d12 = exe.with_file_name(D3D12);
    if d3d12.is_dir() {
        fs::remove_dir_all(&d3d12)
            .map_err(|error| format!("could not remove {}: {error}", d3d12.display()))?;
    }
    let mut pairs = vec![(kept, exe.clone())];
    if previous.join(D3D12).is_dir() {
        pairs.push((previous.join(D3D12), d3d12));
    }
    move_all(&pairs)?;
    clear(previous)
}

/// Restore: swaps the installed build with the kept one, and their commits with
/// them, so calling it again goes back to the build that was installed before.
pub(crate) fn swap(previous: &Path, built_from: &Path) -> Result<(), String> {
    let exe = kept_exe_path(previous).ok_or("No previous build is kept for this project.")?;
    let kept = kept_file(previous, &exe).ok_or("The kept build has no file name.")?;
    if !kept.is_file() {
        return Err(format!(
            "The previous build is missing its program ({}). Update again to keep a new one.",
            kept.display()
        ));
    }
    let name = exe.file_name().unwrap_or_default().to_os_string();
    let d3d12 = exe.with_file_name(D3D12);
    let kept_d3d12 = previous.join(D3D12);
    let tmp_exe = previous.join(suffixed(&name, ".swap"));
    let tmp_d3d12 = previous.join(format!("{D3D12}.swap"));
    let (had_exe, had_d3d12, had_kept_d3d12) = (exe.is_file(), d3d12.is_dir(), kept_d3d12.is_dir());
    // Each step's target is free by the time it runs, so a failure can be undone.
    let mut pairs = Vec::new();
    if had_exe {
        pairs.push((exe.clone(), tmp_exe.clone()));
    }
    pairs.push((kept.clone(), exe.clone()));
    if had_d3d12 {
        pairs.push((d3d12.clone(), tmp_d3d12.clone()));
    }
    if had_kept_d3d12 {
        pairs.push((kept_d3d12.clone(), d3d12.clone()));
    }
    if had_exe {
        pairs.push((tmp_exe, kept));
    }
    if had_d3d12 {
        pairs.push((tmp_d3d12, kept_d3d12));
    }
    move_all(&pairs)?;
    let installed_commit = read_text(built_from).unwrap_or_default();
    let kept_commit = read_text(&previous.join(COMMIT)).unwrap_or_default();
    write_text(built_from, &kept_commit)?;
    write_text(&previous.join(COMMIT), &installed_commit)
}

/// The commit of the build an update replaced, when a build is kept to restore
/// (empty when that commit was not recorded). None: nothing to restore.
pub(crate) fn previous_commit(previous: &Path) -> Option<String> {
    let exe = kept_exe_path(previous)?;
    if !kept_file(previous, &exe)?.is_file() {
        return None;
    }
    Some(read_text(&previous.join(COMMIT)).unwrap_or_default())
}

fn clear(previous: &Path) -> Result<(), String> {
    match fs::remove_dir_all(previous) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not clear {}: {error}", previous.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir; never the real workspace.
    fn scratch(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!(
            "vilauncher-previous-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// An installed build: target\release\victory.exe, its D3D12 folder and the commit.
    fn installed(root: &Path, commit: &str) -> (PathBuf, PathBuf, PathBuf) {
        let target = root.join("target").join("release");
        fs::create_dir_all(target.join("D3D12")).unwrap();
        let exe = target.join("victory.exe");
        fs::write(&exe, format!("exe {commit}")).unwrap();
        fs::write(
            target.join("D3D12").join("core.dll"),
            format!("d3d {commit}"),
        )
        .unwrap();
        let built_from = root.join("target").join("built-from.txt");
        fs::write(&built_from, commit).unwrap();
        (exe, built_from, root.join("previous"))
    }

    fn text(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn a_failed_update_puts_the_old_build_back() {
        let root = scratch("failed");
        let (exe, built_from, previous) = installed(&root, "old111");
        assert!(keep_for_update(&exe, &built_from, &previous).unwrap());
        assert!(!exe.exists());
        assert_eq!(previous_commit(&previous).as_deref(), Some("old111"));
        // The failed build leaves its own exe behind.
        fs::write(&exe, "half built").unwrap();
        restore_failed(&previous).unwrap();
        assert_eq!(text(&exe), "exe old111");
        assert_eq!(
            text(&exe.with_file_name("D3D12").join("core.dll")),
            "d3d old111"
        );
        assert!(!previous.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_restore_swaps_the_two_builds_both_ways() {
        let root = scratch("swap");
        let (exe, built_from, previous) = installed(&root, "old111");
        keep_for_update(&exe, &built_from, &previous).unwrap();
        fs::write(&exe, "exe new222").unwrap();
        fs::write(&built_from, "new222").unwrap();
        fs::create_dir_all(exe.with_file_name("D3D12")).unwrap();
        fs::write(exe.with_file_name("D3D12").join("core.dll"), "d3d new222").unwrap();

        swap(&previous, &built_from).unwrap();
        assert_eq!(text(&exe), "exe old111");
        assert_eq!(text(&built_from), "old111");
        assert_eq!(
            text(&exe.with_file_name("D3D12").join("core.dll")),
            "d3d old111"
        );
        assert_eq!(previous_commit(&previous).as_deref(), Some("new222"));

        swap(&previous, &built_from).unwrap();
        assert_eq!(text(&exe), "exe new222");
        assert_eq!(text(&built_from), "new222");
        assert_eq!(
            text(&exe.with_file_name("D3D12").join("core.dll")),
            "d3d new222"
        );
        assert_eq!(previous_commit(&previous).as_deref(), Some("old111"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_restore_without_a_kept_build_says_so() {
        let root = scratch("none");
        let (exe, built_from, previous) = installed(&root, "old111");
        assert_eq!(previous_commit(&previous), None);
        let error = swap(&previous, &built_from).unwrap_err();
        assert!(error.contains("No previous build"), "{error}");
        assert_eq!(text(&exe), "exe old111");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn nothing_is_kept_when_no_build_is_installed() {
        let root = scratch("empty");
        let exe = root.join("target").join("release").join("victory.exe");
        let built_from = root.join("built-from.txt");
        let previous = root.join("previous");
        assert!(!keep_for_update(&exe, &built_from, &previous).unwrap());
        assert_eq!(previous_commit(&previous), None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_second_update_replaces_the_kept_build() {
        let root = scratch("second");
        let (exe, built_from, previous) = installed(&root, "old111");
        keep_for_update(&exe, &built_from, &previous).unwrap();
        fs::write(&exe, "exe new222").unwrap();
        fs::write(&built_from, "new222").unwrap();
        keep_for_update(&exe, &built_from, &previous).unwrap();
        assert_eq!(previous_commit(&previous).as_deref(), Some("new222"));
        assert_eq!(text(&previous.join("victory.exe")), "exe new222");
        let _ = fs::remove_dir_all(root);
    }
}
