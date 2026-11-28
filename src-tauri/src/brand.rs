//! Who VILauncher is and where it talks to: one place for the names, links
//! and folders, so a fork only has to change this file.

/// The launcher's name, in the title bar, the folders and the updater.
pub const NAME: &str = "VILauncher";
/// Who makes it.
pub const TEAM: &str = msg_str!("VIPortLive team");
/// The site the launcher belongs to.
pub const SITE: &str = "https://viportlive.com";
pub const SITE_LABEL: &str = "viportlive.com";
/// The launcher's own code and releases on GitHub (`owner/name`).
pub const REPOSITORY: &str = "dazacode/vilauncher";
pub const SOURCE_URL: &str = "https://github.com/dazacode/vilauncher";
/// The release download the updater looks for, for this system. The macOS
/// bundle is a `.app.tar.gz` of a universal binary; Windows ships a portable exe.
#[cfg(windows)]
pub const RELEASE_ASSET: &str = "VILauncher.exe";
#[cfg(target_os = "macos")]
pub const RELEASE_ASSET: &str = "VILauncher-macos.app.tar.gz";
#[cfg(all(unix, not(target_os = "macos")))]
pub const RELEASE_ASSET: &str = "VILauncher.AppImage";

/// Where the list of projects comes from, tried in order: VIPortLive's
/// launcher feed (PortWatch's tracker with VIPortLive's corrections and
/// hidden entries applied), then PortWatch's own file if VIPortLive is down.
pub const PROJECT_SOURCES: &[(&str, &str)] = &[
    ("VIPortLive", "https://viportlive.com/api/launcher"),
    ("PortWatch", "https://portwatch-vi.xyz/data/tracker.json"),
];

/// The folder under the system's data folder: projects, pictures and
/// settings. Windows: `%LOCALAPPDATA%` / `%APPDATA%`. macOS: both are
/// `~/Library/Application Support`, the app's own folder there.
pub const FOLDER: &str = "VILauncher";
/// The launcher's earlier names: settings are carried over from these and
/// already installed projects are found there (see `migrate_old_folders`).
pub const LEGACY_FOLDERS: &[&str] = &["Mandyhub", "VI Launcher"];

/// The data folder (projects, pictures): `%LOCALAPPDATA%` on Windows, the
/// platform's own data folder elsewhere.
pub fn local_base() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(Into::into)
            .unwrap_or_else(data_base)
    }
    #[cfg(not(windows))]
    {
        data_base()
    }
}

/// The settings folder: `%APPDATA%` on Windows, the platform's own data
/// folder elsewhere.
pub fn roaming_base() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(Into::into)
            .unwrap_or_else(data_base)
    }
    #[cfg(not(windows))]
    {
        data_base()
    }
}

/// `~/Library/Application Support` on macOS, `$XDG_DATA_HOME` (else
/// `~/.local/share`) elsewhere.
fn data_base() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "macos") {
        return home
            .map(|h| h.join("Library").join("Application Support"))
            .unwrap_or_else(|| ".".into());
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(Into::into)
        .or_else(|| home.map(|h| h.join(".local").join("share")))
        .unwrap_or_else(|| ".".into())
}

/// What the launcher calls itself to web servers.
pub const USER_AGENT: &str = concat!("VILauncher/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_and_projects_folders_are_under_one_data_folder() {
        if cfg!(windows) {
            return;
        }
        // On macOS the roaming and local folders are the same Application
        // Support folder, so settings and projects never split apart.
        assert_eq!(local_base(), roaming_base());
        if cfg!(target_os = "macos") {
            assert!(local_base().ends_with("Library/Application Support"));
        }
    }

    #[test]
    fn the_release_asset_matches_the_system() {
        if cfg!(windows) {
            assert_eq!(RELEASE_ASSET, "VILauncher.exe");
        }
        if cfg!(target_os = "macos") {
            assert!(RELEASE_ASSET.ends_with(".app.tar.gz"));
        }
    }
}
