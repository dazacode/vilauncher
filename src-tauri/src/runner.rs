//! Windows builds on a Mac. The builds in the list are Windows programs that draw with
//! Direct3D 12; on macOS a compatibility runner starts them, and its D3DMetal turns that
//! into Metal. The runner is CrossOver (a bottle) or a Wine build such as Apple's Game
//! Porting Toolkit (a prefix folder). Only a Windows program on macOS goes through it,
//! with a runner chosen in Settings: native programs, and every other system, are untouched.
//!
//! MetalFX: CrossOver's per-bottle DLSS setting runs a game's DLSS on Apple's MetalFX
//! (D3DMetal and DXMT only, and only while DLSS is on in the game). The launcher cannot set
//! that switch; it lets the game's own DLSS path run without an NVIDIA card, from a
//! Streamline folder the player chose (NVIDIA's files are only checked on Windows).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Which runner the player chose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    #[default]
    None,
    #[serde(rename = "crossover")]
    CrossOver,
    Wine,
}

/// The player's runner settings, as saved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Choice {
    pub(crate) kind: Kind,
    /// CrossOver: the bottle's name.
    pub(crate) bottle: String,
    /// Wine: its program (wine64 or wine).
    pub(crate) wine: String,
    /// Wine: the prefix folder (WINEPREFIX).
    pub(crate) prefix: String,
    /// Let the game's DLSS run through the runner's MetalFX.
    pub(crate) metalfx: bool,
    /// NVIDIA's Streamline files for that DLSS path, from the player's own download.
    pub(crate) streamline_dir: String,
}

/// A runner that is there and can start a program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Runner {
    CrossOver { wine: PathBuf, bottle: String },
    Wine { wine: PathBuf, prefix: PathBuf },
}

/// Where CrossOver's command-line `wine` is, inside the app.
const CROSSOVER_WINE: &str = "Contents/SharedSupport/CrossOver/bin/wine";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// CrossOver's `wine`, from /Applications or ~/Applications.
pub(crate) fn crossover_wine() -> Option<PathBuf> {
    let mut apps = vec![PathBuf::from("/Applications")];
    apps.extend(home().map(|h| h.join("Applications")));
    apps.into_iter()
        .map(|dir| dir.join("CrossOver.app").join(CROSSOVER_WINE))
        .find(|wine| wine.is_file())
}

/// Where CrossOver keeps its bottles.
fn bottles_dir() -> Option<PathBuf> {
    home().map(|h| h.join("Library/Application Support/CrossOver/Bottles"))
}

/// The names of the CrossOver bottles on this Mac, sorted.
pub(crate) fn bottles() -> Vec<String> {
    let Some(dir) = bottles_dir() else {
        return Vec::new();
    };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().join("cxbottle.conf").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// A Wine program in the usual places: the Game Porting Toolkit's Homebrew install first.
pub(crate) fn found_wine() -> Option<PathBuf> {
    [
        "/usr/local/opt/game-porting-toolkit/bin/wine64",
        "/opt/homebrew/opt/game-porting-toolkit/bin/wine64",
        "/usr/local/bin/wine64",
        "/opt/homebrew/bin/wine64",
        "/usr/local/bin/wine",
        "/opt/homebrew/bin/wine",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|wine| wine.is_file())
}

/// The runner the choice names, or why it cannot start anything. None: no runner chosen.
pub(crate) fn resolve(choice: &Choice) -> Result<Option<Runner>, String> {
    match choice.kind {
        Kind::None => Ok(None),
        Kind::CrossOver => {
            let wine = crossover_wine()
                .ok_or_else(|| msg_str!("CrossOver was not found in Applications.").to_string())?;
            let bottle = choice.bottle.trim();
            if bottle.is_empty() {
                return Err(msg_str!("Choose a CrossOver bottle in Settings.").into());
            }
            if !bottles_dir().is_some_and(|dir| dir.join(bottle).is_dir()) {
                return Err(msg!("The CrossOver bottle {bottle} was not found."));
            }
            Ok(Some(Runner::CrossOver {
                wine,
                bottle: bottle.to_string(),
            }))
        }
        Kind::Wine => {
            let wine = choice.wine.trim();
            if wine.is_empty() {
                return Err(msg_str!("Choose the Wine program in Settings.").into());
            }
            if !Path::new(wine).is_file() {
                return Err(msg!("The Wine program {wine} is not there."));
            }
            let prefix = choice.prefix.trim();
            if prefix.is_empty() {
                return Err(msg_str!("Choose a Wine prefix folder in Settings.").into());
            }
            Ok(Some(Runner::Wine {
                wine: PathBuf::from(wine),
                prefix: PathBuf::from(prefix),
            }))
        }
    }
}

/// A Mac path as the Windows program sees it: Wine shows the Mac's whole disk as drive Z:.
pub(crate) fn windows_path(path: &str) -> String {
    format!("Z:{}", path.replace('/', "\\"))
}

/// An argument with its Mac path, if it is one (`/a/b` or `--flag=/a/b`), as a Windows path.
fn translate_arg(arg: &str) -> String {
    if arg.starts_with('/') {
        return windows_path(arg);
    }
    match arg.split_once('=') {
        Some((name, value)) if name.starts_with('-') && value.starts_with('/') => {
            format!("{name}={}", windows_path(value))
        }
        _ => arg.to_string(),
    }
}

impl Runner {
    fn wine(&self) -> &Path {
        match self {
            Runner::CrossOver { wine, .. } | Runner::Wine { wine, .. } => wine,
        }
    }

    /// A command that starts `program` (a Windows program) through this runner.
    fn start(&self, program: &Path) -> Command {
        let mut command = Command::new(self.wine());
        match self {
            Runner::CrossOver { bottle, .. } => {
                command.args(["--bottle", bottle.as_str(), "--cx-app"]);
            }
            Runner::Wine { prefix, .. } => {
                command.env("WINEPREFIX", prefix);
            }
        }
        command.arg(program);
        command
    }

    /// `c` started through this runner: its program, its arguments and the `VICTORY_*`
    /// folders it is given, as the Windows program sees them, and its working folder.
    pub(crate) fn wrap(&self, c: &Command) -> Command {
        let mut out = self.start(Path::new(c.get_program()));
        for arg in c.get_args() {
            out.arg(translate_arg(&arg.to_string_lossy()));
        }
        for (name, value) in c.get_envs() {
            match value {
                Some(value) => {
                    let text = value.to_string_lossy();
                    if name.to_string_lossy().starts_with("VICTORY_") && text.starts_with('/') {
                        out.env(name, windows_path(&text));
                    } else {
                        out.env(name, value);
                    }
                }
                None => {
                    out.env_remove(name);
                }
            }
        }
        if let Some(dir) = c.get_current_dir() {
            out.current_dir(dir);
        }
        out
    }

    /// The Windows user's AppData\Local in the bottle or prefix, where the game keeps its
    /// F1 menu file. The user folder that already has the game's folder wins.
    pub(crate) fn local_app_data(&self) -> Option<PathBuf> {
        let root = match self {
            Runner::CrossOver { bottle, .. } => bottles_dir()?.join(bottle),
            Runner::Wine { prefix, .. } => prefix.clone(),
        };
        let users = root.join("drive_c").join("users");
        let local = |user: &str| users.join(user).join("AppData").join("Local");
        let names: Vec<String> = std::fs::read_dir(&users)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name != "Public")
            .collect();
        if let Some(name) = names
            .iter()
            .find(|name| local(name).join("ProjectNeptune").is_dir())
        {
            return Some(local(name));
        }
        let preferred = match self {
            Runner::CrossOver { .. } => "crossover".to_string(),
            Runner::Wine { .. } => std::env::var("USER").unwrap_or_default(),
        };
        names
            .iter()
            .find(|name| **name == preferred)
            .or(names.first())
            .map(|name| local(name))
    }

    /// Ends every Windows program in the bottle or prefix (Wine's own `wineboot --kill`).
    pub(crate) fn end_all(&self) {
        let mut command = self.start(Path::new("wineboot.exe"));
        command.arg("--kill");
        let _ = command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    /// For the log.
    pub(crate) fn describe(&self) -> String {
        match self {
            Runner::CrossOver { bottle, .. } => format!("CrossOver, bottle {bottle}"),
            Runner::Wine { wine, prefix } => {
                format!("Wine {}, prefix {}", wine.display(), prefix.display())
            }
        }
    }
}

/// The runner and MetalFX folder in use, kept from the settings (see [`apply`]).
struct Current {
    runner: Option<Runner>,
    metalfx: Option<PathBuf>,
}

static CURRENT: Mutex<Current> = Mutex::new(Current {
    runner: None,
    metalfx: None,
});

/// Takes the player's choice as the one in use; returns why the runner cannot start anything.
pub(crate) fn apply(choice: &Choice) -> Option<String> {
    let (runner, problem) = match resolve(choice) {
        Ok(runner) => (runner, None),
        Err(problem) => (None, Some(problem)),
    };
    let metalfx = (runner.is_some() && choice.metalfx && !choice.streamline_dir.trim().is_empty())
        .then(|| PathBuf::from(choice.streamline_dir.trim()));
    let mut current = CURRENT.lock().unwrap();
    current.runner = runner;
    current.metalfx = metalfx;
    problem
}

/// The runner in use, on macOS.
pub(crate) fn current() -> Option<Runner> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    CURRENT.lock().unwrap().runner.clone()
}

/// The Streamline folder for DLSS through MetalFX, while a runner is in use and MetalFX is on.
pub(crate) fn metalfx_dir() -> Option<PathBuf> {
    current()?;
    CURRENT.lock().unwrap().metalfx.clone()
}

/// The runner that starts `program`: on macOS, for a Windows program, with a runner chosen.
pub(crate) fn for_program(program: &Path) -> Option<Runner> {
    let runner = current()?;
    (crate::platform::binary_kind(program) == crate::platform::Binary::Windows).then_some(runner)
}

/// Rewrites `c` to start through the runner when its program needs one; the runner when it did.
pub(crate) fn adapt(c: &mut Command) -> Option<Runner> {
    let runner = for_program(Path::new(c.get_program()))?;
    *c = runner.wrap(c);
    Some(runner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wine() -> Runner {
        Runner::Wine {
            wine: PathBuf::from("/opt/gptk/bin/wine64"),
            prefix: PathBuf::from("/Users/me/gptk"),
        }
    }

    #[test]
    fn mac_paths_become_drive_z() {
        assert_eq!(
            windows_path("/Users/me/Game/game.exe"),
            "Z:\\Users\\me\\Game\\game.exe"
        );
        assert_eq!(translate_arg("/Users/me/a"), "Z:\\Users\\me\\a");
        assert_eq!(
            translate_arg("--cache=/Users/me/c"),
            "--cache=Z:\\Users\\me\\c"
        );
        assert_eq!(translate_arg("--console"), "--console");
        assert_eq!(translate_arg("series-s"), "series-s");
        assert_eq!(translate_arg("VICTORY_X=/a"), "VICTORY_X=/a");
    }

    #[test]
    fn a_crossover_run_names_its_bottle_and_the_program() {
        let runner = Runner::CrossOver {
            wine: PathBuf::from("/Applications/CrossOver.app/w"),
            bottle: "Games".into(),
        };
        let mut c = Command::new("/Users/me/build/victory.exe");
        c.args(["run", "/Users/me/Game/game.exe", "--execute"]);
        c.env("VICTORY_DATA_DIR", "/Users/me/data")
            .env("VICTORY_DLSS", "1")
            .env("PATH", "/usr/bin");
        c.current_dir("/Users/me/build");
        let out = runner.wrap(&c);
        assert_eq!(out.get_program(), "/Applications/CrossOver.app/w");
        let args: Vec<_> = out
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--bottle",
                "Games",
                "--cx-app",
                "/Users/me/build/victory.exe",
                "run",
                "Z:\\Users\\me\\Game\\game.exe",
                "--execute"
            ]
        );
        let env = |name: &str| {
            out.get_envs()
                .find(|(k, _)| *k == name)
                .and_then(|(_, v)| v)
                .map(|v| v.to_string_lossy().into_owned())
        };
        assert_eq!(
            env("VICTORY_DATA_DIR").as_deref(),
            Some("Z:\\Users\\me\\data")
        );
        assert_eq!(env("VICTORY_DLSS").as_deref(), Some("1"));
        // Only the game's own folders are rewritten.
        assert_eq!(env("PATH").as_deref(), Some("/usr/bin"));
        assert_eq!(out.get_current_dir(), Some(Path::new("/Users/me/build")));
    }

    #[test]
    fn a_wine_run_sets_its_prefix() {
        let out = wine().wrap(&Command::new("/b/victory.exe"));
        assert_eq!(out.get_program(), "/opt/gptk/bin/wine64");
        let args: Vec<_> = out.get_args().collect();
        assert_eq!(args, ["/b/victory.exe"]);
        assert!(out
            .get_envs()
            .any(|(k, v)| k == "WINEPREFIX" && v == Some("/Users/me/gptk".as_ref())));
    }

    #[test]
    fn a_removed_switch_stays_removed() {
        let mut c = Command::new("/b/victory.exe");
        c.env_remove("VICTORY_DLSS");
        let out = wine().wrap(&c);
        assert!(out
            .get_envs()
            .any(|(k, v)| k == "VICTORY_DLSS" && v.is_none()));
    }

    #[test]
    fn a_choice_says_what_is_missing() {
        assert_eq!(resolve(&Choice::default()), Ok(None));
        let no_program = Choice {
            kind: Kind::Wine,
            ..Default::default()
        };
        assert_eq!(
            resolve(&no_program),
            Err("Choose the Wine program in Settings.".to_string())
        );
        let gone = Choice {
            kind: Kind::Wine,
            wine: "/no/such/wine64".into(),
            prefix: "/p".into(),
            ..Default::default()
        };
        assert_eq!(
            resolve(&gone),
            Err("The Wine program /no/such/wine64 is not there.".to_string())
        );
        let me = std::env::current_exe().unwrap().display().to_string();
        let no_prefix = Choice {
            kind: Kind::Wine,
            wine: me.clone(),
            ..Default::default()
        };
        assert_eq!(
            resolve(&no_prefix),
            Err("Choose a Wine prefix folder in Settings.".to_string())
        );
        let ready = Choice {
            kind: Kind::Wine,
            wine: me.clone(),
            prefix: "/p".into(),
            ..Default::default()
        };
        assert_eq!(
            resolve(&ready),
            Ok(Some(Runner::Wine {
                wine: PathBuf::from(me),
                prefix: PathBuf::from("/p")
            }))
        );
    }

    #[test]
    fn the_game_folder_in_a_prefix_is_found() {
        let root = std::env::temp_dir().join(format!("vl-runner-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let users = root.join("drive_c").join("users");
        std::fs::create_dir_all(users.join("Public")).unwrap();
        std::fs::create_dir_all(users.join("someone").join("AppData").join("Local")).unwrap();
        std::fs::create_dir_all(
            users
                .join("player")
                .join("AppData")
                .join("Local")
                .join("ProjectNeptune"),
        )
        .unwrap();
        let runner = Runner::Wine {
            wine: PathBuf::from("/w"),
            prefix: root.clone(),
        };
        assert_eq!(
            runner.local_app_data(),
            Some(users.join("player").join("AppData").join("Local"))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
