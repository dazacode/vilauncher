use crate::notify::Ctx;
use serde::Deserialize;
use std::fs;
use std::io::Read;
#[cfg(windows)]
use std::io::Write;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::brand::{RELEASE_ASSET as ASSET, REPOSITORY};

pub const RELEASES: &str = "https://github.com/dazacode/vilauncher/releases";
/// The largest download accepted: the portable exe on Windows, the app
/// bundle's tarball on macOS.
#[cfg(windows)]
const MAX_BYTES: u64 = 64 * 1024 * 1024;
#[cfg(not(windows))]
const MAX_BYTES: u64 = 400 * 1024 * 1024;

#[derive(Deserialize)]
pub struct Release {
    pub tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    digest: Option<String>,
}

// Linux never downloads an update (see `check_release`), so it never reads the checks.
#[cfg_attr(all(unix, not(target_os = "macos")), allow(dead_code))]
pub struct DownloadedUpdate {
    pub path: PathBuf,
    size: u64,
    digest: String,
}

#[derive(Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    Current,
    Available(Release),
    Downloading,
    Downloaded(DownloadedUpdate),
    Failed(String),
}

#[derive(Default)]
pub struct Updater {
    pub state: State,
    pub restart_requested: bool,
    pub restart_error: Option<String>,
    result: Option<Receiver<Result<State, String>>>,
}

impl Updater {
    pub fn poll(&mut self) {
        let Some(result) = &self.result else { return };
        self.state = match result.try_recv() {
            Ok(Ok(state)) => state,
            Ok(Err(error)) => State::Failed(error),
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                State::Failed(msg_str!("Update task stopped unexpectedly.").into())
            }
        };
        self.result = None;
    }

    pub fn check(&mut self, ctx: &Ctx) {
        self.restart_requested = false;
        self.restart_error = None;
        self.start(ctx, State::Checking, check_release);
    }

    pub fn download(&mut self, ctx: &Ctx) {
        let State::Available(release) = std::mem::take(&mut self.state) else {
            return;
        };
        self.restart_requested = true;
        self.restart_error = None;
        self.start(ctx, State::Downloading, move || {
            download_release(&release).map(State::Downloaded)
        });
    }

    fn start(
        &mut self,
        ctx: &Ctx,
        state: State,
        task: impl FnOnce() -> Result<State, String> + Send + 'static,
    ) {
        self.state = state;
        let (send, receive) = mpsc::channel();
        self.result = Some(receive);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(task());
            ctx.request_repaint();
        });
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .user_agent(crate::brand::USER_AGENT)
        .build()
}

fn version(tag: &str) -> Option<[u32; 3]> {
    let mut parts = tag.strip_prefix('v').unwrap_or(tag).split('.');
    let mut version = [0; 3];
    for number in &mut version {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        *number = part.parse().ok()?;
    }
    parts.next().is_none().then_some(version)
}

fn check_release() -> Result<State, String> {
    // The in-app update replaces VILauncher.exe on Windows and the .app on macOS. Linux
    // updates from the Releases page, so it is never offered another system's download.
    if !cfg!(any(windows, target_os = "macos")) {
        return Ok(State::Current);
    }
    let url = format!("https://api.github.com/repos/{REPOSITORY}/releases/latest");
    let response = agent()
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call();
    let release: Release = match response {
        Ok(response) => response.into_json().map_err(|error| error.to_string())?,
        Err(ureq::Error::Status(404, _)) => return Ok(State::Current),
        Err(error) => return Err(error.to_string()),
    };
    let available = version(&release.tag_name)
        .ok_or(msg_str!("The release tag is not a supported version."))?;
    let current =
        version(env!("CARGO_PKG_VERSION")).ok_or(msg_str!("The installed version is invalid."))?;
    // A release without this system's download is not offered (a macOS build may follow later).
    if available > current && pick_asset(&release, ASSET).is_ok() {
        Ok(State::Available(release))
    } else {
        Ok(State::Current)
    }
}

/// The asset for this system in a release, or a message saying there is none.
fn pick_asset<'a>(release: &'a Release, asset_name: &str) -> Result<&'a Asset, String> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| msg!("This release has no {file} download.", file = asset_name))
}

fn download_release(release: &Release) -> Result<DownloadedUpdate, String> {
    let asset = pick_asset(release, ASSET)?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .ok_or(msg_str!(
            "GitHub did not provide a SHA-256 checksum for the download."
        ))?;
    if asset.size == 0 || asset.size > MAX_BYTES {
        return Err(msg_str!("The download size is outside the supported limit.").into());
    }
    let url = format!("{RELEASES}/download/{}/{ASSET}", release.tag_name);
    let response = agent()
        .get(&url)
        .call()
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(asset.size + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if !matches_asset(&bytes, asset.size, digest) {
        return Err(
            msg_str!("The download did not match GitHub’s size and SHA-256 checksum.").into(),
        );
    }
    let folder = crate::default_workspace()
        .join("updates")
        .join(&release.tag_name);
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let temporary = folder.join(format!("{ASSET}.{}.tmp", std::process::id()));
    let executable = folder.join(ASSET);
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(&temporary, &executable) {
        let _ = fs::remove_file(temporary);
        return Err(error.to_string());
    }
    Ok(DownloadedUpdate {
        path: executable,
        size: asset.size,
        digest: digest.to_owned(),
    })
}

/// Replaces the running launcher and starts the new one. Windows: the
/// portable exe beside it. macOS: the whole `.app` bundle.
#[cfg(windows)]
pub fn restart_with(update: &DownloadedUpdate) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let staged = executable.with_extension(format!("vilauncher-{}.tmp", std::process::id()));
    let backup = executable.with_extension("vilauncher-previous.exe");
    let source = fs::File::open(&update.path)
        .map_err(|error| msg!("Could not open the downloaded update: {error}"))?;
    let mut bytes = Vec::new();
    source
        .take(update.size + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if !matches_asset(&bytes, update.size, &update.digest) {
        return Err(msg_str!(
            "The downloaded update changed on disk. Check for updates and download it again."
        )
        .into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)
        .map_err(|error| msg!("Could not stage the update beside VILauncher: {error}"))?;
    let write_result = file.write_all(&bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = (|| -> Result<(), String> {
        write_result.map_err(|error| msg!("Could not save the staged update: {error}"))?;
        match fs::remove_file(&backup) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(msg!("Could not remove the previous update backup: {error}")),
        }
        fs::rename(&executable, &backup)
            .map_err(|error| msg!("Could not move the running launcher aside: {error}"))?;
        let restore = |message: String| match fs::rename(&backup, &executable) {
            Ok(()) => message,
            Err(error) => msg!(
                "{message} Could not restore the previous version: {error}. Backup: {path}",
                path = backup.display()
            ),
        };
        fs::rename(&staged, &executable)
            .map_err(|error| restore(msg!("Could not put the update in place: {error}")))?;
        if let Err(error) = std::process::Command::new(&executable).spawn() {
            return Err(restore(msg!("Could not start the new version: {error}")));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    result
}

/// Copies an `.app` bundle (or a folder) with `ditto`: it keeps symlinks,
/// permission bits, extended attributes and the code signature intact, which
/// `fs::copy` per file does not.
#[cfg(target_os = "macos")]
fn copy_bundle(from: &PathBuf, to: &Path) -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/ditto")
        .arg(from)
        .arg(to)
        .status()
        .map_err(|error| format!("ditto did not run: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "ditto could not copy the new app bundle".to_string())
}

/// Drops Gatekeeper's quarantine flag so the replaced app starts without a
/// prompt. Best effort: a copy made by `ditto` from a local file may carry none.
#[cfg(target_os = "macos")]
fn clear_quarantine(path: &Path) {
    let _ = std::process::Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(path)
        .status();
}

/// Replaces the running `.app` bundle with the downloaded one and starts it.
///
/// The archive's SHA-256 is checked again, it is unpacked into a staging
/// folder on the same volume, its program is checked, and only then is the
/// old bundle moved aside. Any failure puts the old bundle back; a bundle on
/// a read-only volume (a mounted DMG, `/Applications` for another user) is
/// replaced through `osascript` with an administrator prompt instead.
#[cfg(target_os = "macos")]
pub fn restart_with(update: &DownloadedUpdate) -> Result<(), String> {
    fn quoted(path: &std::path::Path) -> String {
        format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
    }

    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let bundle = exe
        .ancestors()
        .find(|part| part.extension().is_some_and(|ext| ext == "app"))
        .ok_or("The launcher is not running from an .app bundle, so it cannot replace itself.")?
        .to_path_buf();
    let parent = bundle
        .parent()
        .ok_or("The app bundle has no folder.")?
        .to_path_buf();
    let name = bundle
        .file_name()
        .ok_or("The app bundle has no name.")?
        .to_owned();
    let backup = parent.join(format!(
        ".{}.previous",
        name.to_string_lossy().trim_end_matches(".app")
    ));
    let source = fs::File::open(&update.path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    source
        .take(update.size + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if !matches_asset(&bytes, update.size, &update.digest) {
        return Err(
            "The downloaded update changed on disk. Check for updates and download it again."
                .into(),
        );
    }
    // A fresh staging folder next to the bundle, so the last steps are renames.
    let staging = update
        .path
        .with_file_name(format!("staged-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let unpack = std::process::Command::new("/usr/bin/tar")
        .arg("-xzf")
        .arg(&update.path)
        .arg("-C")
        .arg(&staging)
        .status()
        .map_err(|error| format!("Could not unpack the update: {error}"))?;
    if !unpack.success() {
        return Err("The downloaded update could not be unpacked.".into());
    }
    let staged_app = fs::read_dir(&staging)
        .map_err(|error| error.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "app") && path.is_dir())
        .ok_or("The update archive holds no .app bundle.")?;
    let program = staged_app.join("Contents").join("MacOS");
    let ok = fs::read_dir(&program)
        .ok()
        .map(|mut entries| entries.any(|e| e.is_ok_and(|e| e.path().is_file())))
        .unwrap_or(false);
    if !ok {
        return Err("The update's app bundle holds no program.".into());
    }

    // Into place beside the bundle, then swap with renames.
    let target = parent.join(&name);
    if let Err(error) = copy_bundle(&staged_app, &target) {
        let _ = fs::remove_dir_all(&target);
        return Err(format!("Could not put the update in place: {error}"));
    }
    clear_quarantine(&target);
    let swap = |target: &PathBuf| -> Result<(), String> {
        let _ = fs::remove_dir_all(&backup);
        fs::rename(&bundle, &backup)
            .map_err(|error| format!("Could not move the running launcher aside: {error}"))?;
        if let Err(error) = fs::rename(target, &bundle) {
            let _ = fs::rename(&backup, &bundle);
            return Err(format!("Could not put the update in place: {error}"));
        }
        Ok(())
    };
    match swap(&target) {
        Ok(()) => {}
        Err(_error) if parent_is_read_only(&parent) => {
            // A mounted DMG or a bundle owned by another user: ask for an administrator.
            let script = format!(
                "rm -rf {} && mv {} {} && rm -rf {}",
                quoted(&bundle),
                quoted(&target),
                quoted(&bundle),
                quoted(&backup)
            );
            let script = script.replace('\\', "\\\\").replace('"', "\\\"");
            let status = std::process::Command::new("/usr/bin/osascript")
                .args([
                    "-e",
                    &format!("do shell script \"{script}\" with administrator privileges"),
                ])
                .status()
                .map_err(|error| {
                    format!("{error} (the update is ready in {})", target.display())
                })?;
            if !status.success() {
                return Err(format!(
                    "macOS did not allow replacing the app. Move it to a folder you own, or install the update from {} yourself.",
                    target.display()
                ));
            }
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&target);
            return Err(error);
        }
    }
    let started = std::process::Command::new("/usr/bin/open")
        .args(["-n"])
        .arg(&bundle)
        .spawn();
    let _ = fs::remove_dir_all(&staging);
    match started {
        Ok(_) => Ok(()),
        Err(error) => Err(format!(
            "The update is in place but could not be started: {error}. Open VILauncher from Applications."
        )),
    }
}

/// Whether the folder refuses a new entry (a mounted DMG, another user's
/// folder). Probed, because permission bits alone mislead on ACLs.
#[cfg(target_os = "macos")]
fn parent_is_read_only(parent: &std::path::Path) -> bool {
    let probe = parent.join(format!(".vilauncher-write-{}", std::process::id()));
    match fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            false
        }
        Err(_) => true,
    }
}

/// Linux updates from the Releases page: `check_release` never offers an
/// update here, so there is nothing to put in place.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn restart_with(_update: &DownloadedUpdate) -> Result<(), String> {
    Err(format!("Download the new version from {RELEASES}."))
}

fn matches_asset(bytes: &[u8], size: u64, sha256: &str) -> bool {
    let actual = ring::digest::digest(&ring::digest::SHA256, bytes);
    let actual: String = actual
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    bytes.len() as u64 == size && actual.eq_ignore_ascii_case(sha256)
}

#[cfg(test)]
#[test]
fn release_links_follow_the_brand() {
    assert_eq!(
        RELEASES,
        format!("https://github.com/{REPOSITORY}/releases")
    );
    assert_eq!(RELEASES, format!("{}/releases", crate::brand::SOURCE_URL));
}

#[cfg(test)]
#[test]
fn compares_stable_versions_numerically() {
    assert!(version("v0.10.0") > version("0.2.0"));
    assert_eq!(version("v0.2.0"), version("0.2.0"));
    assert!(version("v0.3.0-beta").is_none());
    assert!(version("v0.3.0/other").is_none());
}

#[cfg(test)]
#[test]
fn the_system_s_asset_is_picked_from_a_release() {
    let release = Release {
        tag_name: "v1.2.3".into(),
        assets: vec![
            Asset {
                name: "VILauncher.exe".into(),
                size: 1,
                digest: None,
            },
            Asset {
                name: "VILauncher-macos.app.tar.gz".into(),
                size: 2,
                digest: None,
            },
        ],
    };
    // Windows updates with VILauncher.exe, macOS with its archive; Linux updates from the
    // Releases page, so no asset is picked there.
    let picked = pick_asset(&release, ASSET);
    if cfg!(windows) {
        assert_eq!(picked.unwrap().size, 1);
    } else if cfg!(target_os = "macos") {
        assert_eq!(picked.unwrap().size, 2);
    } else {
        assert!(picked.is_err());
    }
    assert!(pick_asset(&release, "VILauncher-linux").is_err());
}

#[cfg(test)]
#[test]
fn rejects_changed_or_truncated_downloads() {
    let known = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(matches_asset(b"abc", 3, known));
    assert!(!matches_asset(b"abd", 3, known));
    assert!(!matches_asset(b"abc", 4, known));
}
