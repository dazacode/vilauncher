//! NVIDIA's Streamline files, which DLSS upscaling and DLSS frame generation run on. They are not ours to
//! ship. The player asks for them with "Get NVIDIA's files" (Settings › Graphics): the launcher downloads
//! NVIDIA's Streamline SDK release from NVIDIA's GitHub, checks its size and SHA-256, keeps only the DLLs
//! a run needs (bin\x64, not the development ones) in `%LOCALAPPDATA%\Neptune\streamline`, and keeps each
//! one only when it has a valid Authenticode signature by "NVIDIA Corporation". The request is anonymous:
//! a plain GET of one fixed URL, and only when the player presses the button.
//!
//! The same folder is the one Neptune's own launcher uses, so files fetched by either are used by both.
//! Each kept file is recorded in `verified.txt` with its size and SHA-256 hash. A run checks that record
//! (a read and a hash of each file, no signature check), so a file that changed since it was checked is
//! not used until it is checked again.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::notify::Ctx;

pub(crate) const URL: &str = "https://github.com/NVIDIA-RTX/Streamline/releases/download/v2.14.16/streamline-sdk-v2.14.16.zip";
const ZIP_SIZE: u64 = 276_016_170;
const ZIP_SHA256: &str = "32041e51db0cab709a271b72a352281b690c2abbd669a827809401a5dd23956b";
const SIGNER: &str = "NVIDIA Corporation";
const MARKER: &str = "verified.txt";

/// Always: the interposer and its common plugin.
const BASE: [&str; 2] = ["sl.interposer.dll", "sl.common.dll"];
/// Frame generation (9.9 MB with the base).
const FRAMEGEN: [&str; 4] = ["sl.dlss_g.dll", "nvngx_dlssg.dll", "sl.reflex.dll", "sl.pcl.dll"];
/// DLSS upscaling (56.6 MB more).
const UPSCALE: [&str; 2] = ["sl.dlss.dll", "nvngx_dlss.dll"];

/// The files a run needs for these switches.
pub(crate) fn needed(framegen: bool, dlss: bool) -> Vec<&'static str> {
    let mut files: Vec<&'static str> = BASE.to_vec();
    if framegen {
        files.extend(FRAMEGEN);
    }
    if dlss {
        files.extend(UPSCALE);
    }
    files
}

/// Where the files are kept: `%LOCALAPPDATA%\Neptune\streamline`, the folder Neptune's launcher uses.
/// Empty when `LOCALAPPDATA` is not set, which means nothing can be kept.
pub(crate) fn default_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("Neptune").join("streamline"))
        .unwrap_or_default()
}

/// Why the player's own folder `dir` lacks one of `files`, or None when each is there. Nothing is
/// checked beyond that: on a Mac the files come from the player's own download (see runner.rs).
pub(crate) fn present(dir: &Path, files: &[&str]) -> Option<String> {
    if !dir.is_dir() {
        return Some(format!("no folder {}", dir.display()));
    }
    files
        .iter()
        .find(|file| !dir.join(file).is_file())
        .map(|file| format!("{file} is not in {}", dir.display()))
}

/// Why `dir` cannot serve `files` for a run, or None when it can: each is there and matches the record
/// written when it was checked (same size and SHA-256). It runs at each Play, so it reads and hashes the
/// files but does not run the signature check again.
pub(crate) fn missing(dir: &Path, files: &[&str]) -> Option<String> {
    if !dir.is_dir() {
        return Some(format!("no folder {}", dir.display()));
    }
    let record = read_record(dir);
    for file in files {
        let path = dir.join(file);
        let Ok(meta) = fs::metadata(&path) else {
            return Some(format!("{file} is not in {}", dir.display()));
        };
        match record.get(*file) {
            Some((size, hash)) if *size == meta.len() && file_sha256(&path).is_ok_and(|now| &now == hash) => {}
            _ => return Some(format!("{file} is not checked as NVIDIA's: use Get NVIDIA's files")),
        }
    }
    None
}

/// The record: `name<TAB>size<TAB>sha256` per file.
fn read_record(dir: &Path) -> HashMap<String, (u64, String)> {
    fs::read_to_string(dir.join(MARKER))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let name = parts.next()?;
            let size = parts.next()?.parse().ok()?;
            let hash = parts.next()?.to_string();
            Some((name.to_string(), (size, hash)))
        })
        .collect()
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    Ok(hex(context.finish().as_ref()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks each of `files` in `dir` (presence and NVIDIA's signature), and records them as checked.
pub(crate) fn verify_and_record(dir: &Path, files: &[&str]) -> Result<(), String> {
    let mut lines = String::new();
    for file in files {
        let path = dir.join(file);
        signed_by_nvidia(&path)?;
        let size = fs::metadata(&path)
            .map_err(|error| msg!("{file} could not be read: {error}"))?
            .len();
        let hash = file_sha256(&path).map_err(|error| msg!("{file} could not be read: {error}"))?;
        lines.push_str(&format!("{file}\t{size}\t{hash}\n"));
    }
    fs::write(dir.join(MARKER), lines).map_err(|error| msg!("could not record the checked files: {error}"))
}

/// Whether the file is validly signed by NVIDIA Corporation. Windows only: it asks the system's own
/// Authenticode check (Get-AuthenticodeSignature), with the path passed in the environment.
#[cfg(windows)]
fn signed_by_nvidia(path: &Path) -> Result<(), String> {
    const SCRIPT: &str = "$s = Get-AuthenticodeSignature -LiteralPath $env:VL_SIGNED_FILE; if ($s.Status -ne 'Valid') { 'status:' + $s.Status } else { 'signer:' + $s.SignerCertificate.GetNameInfo('SimpleName', $false) }";
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    // The full path to Windows PowerShell: a plain name would be looked up in the search path, where a
    // writable folder could hold a copy.
    let root = std::env::var_os("SystemRoot").ok_or_else(|| msg!("{name}: the system folder is not known"))?;
    let powershell = PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .env("VL_SIGNED_FILE", path)
        .stdin(std::process::Stdio::null());
    crate::launcher::hide_window(&mut command);
    let output = command
        .output()
        .map_err(|error| msg!("{name}: could not run the signature check ({error})"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.trim();
    if let Some(signer) = line.strip_prefix("signer:") {
        let signer = signer.trim();
        return if signer == SIGNER {
            Ok(())
        } else {
            Err(msg!("{name} is signed by \"{signer}\", not by NVIDIA Corporation"))
        };
    }
    let status = line.strip_prefix("status:").unwrap_or("unknown");
    Err(msg!("{name} is not validly signed ({status})"))
}

#[cfg(not(windows))]
fn signed_by_nvidia(_path: &Path) -> Result<(), String> {
    Err(msg_str!("NVIDIA's files are checked on Windows only.").to_string())
}

/// The download's state for the window. `busy` while a fetch runs; `status` is the last thing it said.
#[derive(Clone, Debug, Default)]
pub(crate) struct Fetch {
    pub busy: bool,
    pub status: Option<String>,
    pub got: u64,
    pub total: u64,
}

pub(crate) type Shared = Arc<Mutex<Fetch>>;

fn lock(state: &Shared) -> std::sync::MutexGuard<'_, Fetch> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Starts the fetch on its own thread, unless one runs already.
pub(crate) fn start(state: &Shared, ctx: &Ctx, dir: PathBuf) {
    {
        let mut fetch = lock(state);
        if fetch.busy {
            return;
        }
        *fetch = Fetch { busy: true, status: Some(msg_str!("Connecting to GitHub…").to_string()), got: 0, total: ZIP_SIZE };
    }
    let (state, ctx) = (state.clone(), ctx.clone());
    std::thread::spawn(move || {
        let set = |text: String| {
            lock(&state).status = Some(text);
            ctx.request_repaint();
        };
        let progress = |got: u64| {
            lock(&state).got = got;
            ctx.request_repaint();
        };
        // A panic inside the fetch must still clear `busy`, or the button stays disabled until restart.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            fetch_into(&dir, &progress, &set)
        }));
        let status = match outcome {
            Ok(Ok(text)) => text,
            Ok(Err(error)) => error,
            Err(_) => msg_str!("Getting NVIDIA's files stopped unexpectedly: nothing kept.").to_string(),
        };
        {
            let mut fetch = lock(&state);
            fetch.busy = false;
            fetch.status = Some(status);
        }
        ctx.request_repaint();
    });
}

/// The fetch itself: returns the closing status line, or the error to show. Keeps nothing on an error.
fn fetch_into(dir: &Path, progress: &dyn Fn(u64), say: &dyn Fn(String)) -> Result<String, String> {
    if dir.as_os_str().is_empty() {
        return Err(msg_str!("LOCALAPPDATA is not set, so there is no folder to keep NVIDIA's files in.").to_string());
    }
    let files = needed(true, true);
    let dir_text = dir.display().to_string();
    if dir.is_dir() && files.iter().all(|file| dir.join(file).is_file()) {
        // Files already there (fetched by Neptune's launcher, perhaps): checked, not downloaded again.
        say(msg!("Checking the files already in {dir_text}…"));
        if verify_and_record(dir, &files).is_ok() {
            return Ok(msg!("NVIDIA's files are ready in {dir_text}, signed by NVIDIA Corporation."));
        }
    }
    fs::create_dir_all(dir).map_err(|error| msg!("Could not create {dir_text}: {error}. Nothing was kept."))?;
    let work = dir
        .parent()
        .unwrap_or(dir)
        .join(format!("streamline-download-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|error| msg!("Could not create a download folder: {error}. Nothing was kept."))?;
    let result = download_and_check(&work, dir, &files, progress, say);
    let _ = fs::remove_dir_all(&work);
    result.map(|()| msg!("NVIDIA's files are ready in {dir_text}, signed by NVIDIA Corporation."))
}

fn download_and_check(
    work: &Path,
    dir: &Path,
    files: &[&str],
    progress: &dyn Fn(u64),
    say: &dyn Fn(String),
) -> Result<(), String> {
    let zip_path = work.join("streamline-sdk.zip");
    let response = ureq::get(URL)
        .set("User-Agent", "VILauncher")
        .timeout(Duration::from_secs(3600))
        .call()
        .map_err(|error| match error {
            ureq::Error::Status(code, _) => msg!("GitHub answered {code}: nothing was downloaded."),
            other => msg!("Could not reach GitHub: {other}. Nothing was kept."),
        })?;
    let mut body = response.into_reader();
    let mut out = File::create(&zip_path).map_err(|error| msg!("Could not write the download: {error}. Nothing was kept."))?;
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = vec![0u8; 1 << 20];
    let mut got: u64 = 0;
    let mut shown = Instant::now();
    loop {
        let read = body
            .read(&mut buffer)
            .map_err(|error| msg!("The download stopped at {got} bytes ({error}). Nothing was kept."))?;
        if read == 0 {
            break;
        }
        got += read as u64;
        if got > ZIP_SIZE {
            return Err(msg_str!("The download is larger than NVIDIA's release: nothing kept.").to_string());
        }
        digest.update(&buffer[..read]);
        out.write_all(&buffer[..read])
            .map_err(|error| msg!("Could not write the download: {error}. Nothing was kept."))?;
        if shown.elapsed() > Duration::from_millis(400) {
            shown = Instant::now();
            progress(got);
            let (done, total) = (got >> 20, ZIP_SIZE >> 20);
            say(msg!("Downloading NVIDIA's Streamline SDK: {done} of {total} MB"));
        }
    }
    drop(out);
    if got != ZIP_SIZE {
        return Err(msg!("The download ended early at {got} bytes: nothing kept."));
    }
    let hash = hex(digest.finish().as_ref());
    if hash != ZIP_SHA256 {
        return Err(msg_str!("The download's SHA-256 is not NVIDIA's release's: nothing kept.").to_string());
    }
    say(msg_str!("Checking NVIDIA's signatures…").to_string());
    let mut archive = zip::ZipArchive::new(File::open(&zip_path).map_err(|error| error.to_string())?)
        .map_err(|error| msg!("Could not read the SDK: {error}. Nothing was kept."))?;
    let mut extracted = Vec::new();
    for file in files {
        let wanted = format!("bin/x64/{file}");
        let mut found = None;
        for index in 0..archive.len() {
            let name = archive
                .by_index(index)
                .ok()
                .and_then(|entry| entry.name().ok().map(|name| name.replace('\\', "/")))
                .unwrap_or_default();
            if name.eq_ignore_ascii_case(&wanted) || name.to_ascii_lowercase().ends_with(&format!("/{wanted}")) {
                found = Some(index);
                break;
            }
        }
        let index = found.ok_or_else(|| msg!("{file} is not in the SDK: nothing kept."))?;
        // The output name is the constant name from our own list, never the zip's entry name.
        let fresh = work.join(file);
        let mut entry = archive
            .by_index(index)
            .map_err(|error| msg!("Could not read {file} from the SDK: {error}. Nothing was kept."))?;
        let mut out = File::create(&fresh).map_err(|error| msg!("Could not write {file}: {error}. Nothing was kept."))?;
        io::copy(&mut entry, &mut out).map_err(|error| msg!("Could not write {file}: {error}. Nothing was kept."))?;
        drop(out);
        extracted.push(*file);
    }
    // Every file is checked before any is kept, so the folder never holds a mix of new and old.
    for file in &extracted {
        signed_by_nvidia(&work.join(file))?;
    }
    // Replacing the files one at a time can fail part way (a DLL in use, say). If it does, the record is
    // removed first, so the folder is not used until a later fetch checks it again.
    let _ = fs::remove_file(dir.join(MARKER));
    for file in &extracted {
        fs::rename(work.join(file), dir.join(file))
            .or_else(|_| fs::copy(work.join(file), dir.join(file)).map(|_| ()))
            .map_err(|error| msg!("Could not keep {file}: {error}. The files are not usable until Get NVIDIA's files runs again."))?;
    }
    verify_and_record(dir, files)
}

/// Writes the record for files the caller has already checked (tests only: not a way to skip the check).
#[cfg(test)]
pub(crate) fn record_for_tests(dir: &Path, files: &[&str]) {
    let mut lines = String::new();
    for file in files {
        let path = dir.join(file);
        if let (Ok(meta), Ok(hash)) = (fs::metadata(&path), file_sha256(&path)) {
            lines.push_str(&format!("{file}\t{}\t{hash}\n", meta.len()));
        }
    }
    fs::write(dir.join(MARKER), lines).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vl-streamline-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_files_for_each_switch_are_the_ones_neptune_checks() {
        assert_eq!(needed(false, false), ["sl.interposer.dll", "sl.common.dll"]);
        assert_eq!(needed(true, false).len(), 6);
        assert_eq!(needed(true, true).len(), 8);
        assert!(needed(false, true).contains(&"nvngx_dlss.dll"));
    }

    #[test]
    fn a_file_is_used_only_once_its_record_matches_its_contents() {
        let dir = scratch("record");
        let files = needed(false, false);
        for file in &files {
            fs::write(dir.join(file), b"not really a dll").unwrap();
        }
        // Present but never checked: not used.
        assert!(missing(&dir, &files).is_some_and(|why| why.contains("not checked as NVIDIA's")));
        // Recorded as checked: used.
        record_for_tests(&dir, &files);
        assert_eq!(missing(&dir, &files), None);
        // Same size, different bytes: not used, even though the size still matches.
        fs::write(dir.join(files[0]), b"NOT really a dll").unwrap();
        assert!(missing(&dir, &files).is_some_and(|why| why.contains(files[0])));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_is_named() {
        let dir = scratch("missing");
        let why = missing(&dir, &needed(false, false)).unwrap();
        assert!(why.contains("sl.interposer.dll is not in"), "{why}");
        fs::remove_dir_all(&dir).ok();
    }

    /// The real thing: downloads NVIDIA's SDK from GitHub (276 MB), checks it and keeps the files in the
    /// default folder. Run on purpose with `cargo test -- --ignored real_download`.
    #[test]
    #[ignore]
    fn real_download_keeps_nvidia_signed_files() {
        let dir = default_dir();
        let result = fetch_into(&dir, &|_| {}, &|line| eprintln!("{line}"));
        let files = needed(true, true);
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(missing(&dir, &files), None);
    }
}
