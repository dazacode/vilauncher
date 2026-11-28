//! Git, Rust (cargo) and the build tools that building a project needs:
//! found on the `PATH` or in the folders their installers use (a `PATH`
//! changed after the launcher started does not reach programs started before
//! it), and put on this PC with the system's own installer when the player
//! asks: winget on Windows, Xcode Command Line Tools and rustup on macOS.
//! Elsewhere the player installs them with the system's package manager.

use crate::platform;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[derive(Clone, Default, Debug)]
pub struct Tools {
    pub git: Option<PathBuf>,
    pub cargo: Option<PathBuf>,
    /// The C++ build tools (MSVC) on Windows, Xcode Command Line Tools on
    /// macOS, the system C compiler elsewhere. Rust needs their linker.
    pub clt: bool,
}

impl Tools {
    pub fn ready(&self) -> bool {
        self.git.is_some() && self.cargo.is_some() && self.clt
    }

    /// What is missing, for people.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut m = Vec::new();
        if !self.clt {
            m.push(BUILD_TOOLS);
        }
        if self.git.is_none() {
            m.push("Git");
        }
        if self.cargo.is_none() {
            m.push("Rust");
        }
        m
    }
}

/// What the build tools are called on this system: what Rust links with.
#[cfg(windows)]
pub const BUILD_TOOLS: &str = msg_str!("C++ build tools");
#[cfg(target_os = "macos")]
pub const BUILD_TOOLS: &str = msg_str!("Xcode Command Line Tools");
#[cfg(all(unix, not(target_os = "macos")))]
pub const BUILD_TOOLS: &str = msg_str!("C compiler");

/// The last look, shared with the build commands.
static FOUND: Mutex<Option<Tools>> = Mutex::new(None);

pub fn current() -> Option<Tools> {
    FOUND.lock().unwrap().clone()
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

fn answers(program: &Path) -> bool {
    let mut c = Command::new(program);
    c.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    platform::configure_background_command(&mut c);
    c.status().map(|s| s.success()).unwrap_or(false)
}

/// `name` on the PATH, else the first of `places` that exists and answers.
fn find(name: &str, places: &[Option<PathBuf>]) -> Option<PathBuf> {
    if answers(Path::new(name)) {
        return Some(PathBuf::from(name));
    }
    places
        .iter()
        .flatten()
        .find(|p| p.is_file() && answers(p))
        .cloned()
}

/// Where Git's installers put it, for when it is not on the `PATH`.
fn git_places() -> Vec<Option<PathBuf>> {
    #[cfg(windows)]
    {
        let pf = env_path("ProgramFiles");
        let pf86 = env_path("ProgramFiles(x86)");
        let local = env_path("LOCALAPPDATA");
        vec![
            pf.map(|p| p.join("Git").join("cmd").join("git.exe")),
            pf86.map(|p| p.join("Git").join("cmd").join("git.exe")),
            local.map(|p| p.join("Programs").join("Git").join("cmd").join("git.exe")),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![
            Some(PathBuf::from("/opt/homebrew/bin/git")),
            Some(PathBuf::from("/usr/local/bin/git")),
            Some(PathBuf::from("/usr/bin/git")),
        ]
    }
}

/// rustup's folder (`CARGO_HOME`, else `~/.cargo`), which a desktop session's
/// `PATH` often lacks, then Homebrew's on macOS.
fn cargo_places(program: &str) -> Vec<Option<PathBuf>> {
    let name = platform::exe_name(program);
    let home = env_path(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    let mut places = vec![
        env_path("CARGO_HOME").map(|p| p.join("bin").join(&name)),
        home.map(|p| p.join(".cargo").join("bin").join(&name)),
    ];
    if cfg!(target_os = "macos") {
        places.push(Some(PathBuf::from("/opt/homebrew/bin").join(&name)));
    }
    places
}

/// Looks for the tools now.
pub fn detect() -> Tools {
    let t = Tools {
        git: find("git", &git_places()),
        cargo: find("cargo", &cargo_places("cargo")),
        clt: platform::build_tools_installed(),
    };
    *FOUND.lock().unwrap() = Some(t.clone());
    t
}

/// A command for a tool, by the path it was found at, with the tools'
/// folders first on its PATH (cargo runs rustc and git from there).
fn command(path: Option<PathBuf>, name: &str) -> Command {
    let program = path.unwrap_or_else(|| PathBuf::from(name));
    let mut c = Command::new(&program);
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(t) = current() {
        for p in [t.git, t.cargo].into_iter().flatten() {
            if let Some(d) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                dirs.push(d.to_path_buf());
            }
        }
    }
    // The found tools first, then whatever the system had, then the standard
    // tool folders (macOS needs the last: a GUI app inherits a short `PATH`).
    let mut value = if dirs.is_empty() {
        std::env::var_os("PATH")
    } else {
        if let Some(path) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&path));
        }
        std::env::join_paths(dirs).ok()
    };
    value = platform::with_tool_dirs(value);
    if let Some(value) = value {
        c.env("PATH", value);
    }
    c
}

pub fn git() -> Command {
    command(current().and_then(|t| t.git), "git")
}

pub fn cargo() -> Command {
    command(current().and_then(|t| t.cargo), "cargo")
}

/// Whether the launcher can install the tools here: winget on Windows,
/// Apple's installer and rustup on macOS. Elsewhere the player does it.
pub fn has_manager() -> bool {
    #[cfg(windows)]
    {
        answers(Path::new("winget"))
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        false
    }
}

/// The rustup program, when it is on the `PATH` or in its usual folder.
pub fn rustup() -> Option<PathBuf> {
    find("rustup", &cargo_places("rustup"))
}

/// What the player does for the tools still missing after Fix: each one's install page.
pub fn by_hand_hint(missing: &[&str]) -> String {
    let steps: Vec<String> = missing
        .iter()
        .map(|what| format!("{what} from {}", manual_install_page(what)))
        .collect();
    format!(
        "Install {} by hand, then press Fix again. A winget install can report success while the tool does not run.",
        steps.join("; ")
    )
}

/// Where to get a tool by hand when winget cannot install it.
pub fn manual_install_page(what: &str) -> &'static str {
    match what {
        "Git" => "https://git-scm.com/download/win",
        "Rust" => "https://rustup.rs",
        _ => "https://visualstudio.microsoft.com/visual-cpp-build-tools/",
    }
}

/// Whether a winget error is its "no newer version" result, APPINSTALLER_CLI_ERROR_UPDATE_NOT_APPLICABLE
/// (0x8A15002B, "No applicable update found"). That means the package is already installed, so the
/// tool is there and the look-again decides.
pub fn winget_already_installed(error: &str) -> bool {
    error.to_ascii_lowercase().contains("0x8a15002b")
}

/// One install step: what it is for, the program, and its arguments.
pub struct Step {
    pub what: &'static str,
    pub program: &'static str,
    pub args: Vec<String>,
    /// The program only opens an installer window and returns; the caller waits
    /// for the tool to answer instead (Apple's Command Line Tools).
    pub opens_installer: bool,
}

/// The installs for what `t` lacks, run in this order.
pub fn install_steps(t: &Tools) -> Vec<Step> {
    #[cfg(windows)]
    {
        let common = [
            "--exact",
            "--source",
            "winget",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ];
        let mut steps = Vec::new();
        let mut step = |what: &'static str, id: &str, extra: &[&str]| {
            let mut args: Vec<String> = vec!["install".into(), "--id".into(), id.into()];
            args.extend(common.iter().map(|s| s.to_string()));
            args.extend(extra.iter().map(|s| s.to_string()));
            steps.push(Step {
                what,
                program: "winget",
                args,
                opens_installer: false,
            });
        };
        if t.git.is_none() {
            step("Git", "Git.Git", &["--silent"]);
        }
        if !t.clt {
            step(
                BUILD_TOOLS,
                "Microsoft.VisualStudio.2022.BuildTools",
                &["--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"],
            );
        }
        if t.cargo.is_none() {
            step("Rust", "Rustlang.Rustup", &["--silent"]);
        }
        steps
    }
    #[cfg(target_os = "macos")]
    {
        let mut steps = Vec::new();
        if !t.clt {
            // Apple's installer window opens; the caller waits for the tools to answer.
            steps.push(Step {
                what: BUILD_TOOLS,
                program: "/usr/bin/xcode-select",
                args: vec!["--install".into()],
                opens_installer: true,
            });
        }
        if t.git.is_none() && answers(Path::new("/opt/homebrew/bin/brew")) {
            steps.push(Step {
                what: "Git",
                program: "/opt/homebrew/bin/brew",
                args: vec!["install".into(), "git".into()],
                opens_installer: false,
            });
        }
        if t.cargo.is_none() {
            // The official installer, downloaded first so it is visible in the log.
            let script = std::env::temp_dir().join("vilauncher-rustup-init.sh");
            steps.push(Step {
                what: "Rust",
                program: "/usr/bin/curl",
                args: vec![
                    "--proto".into(),
                    "=https".into(),
                    "--tlsv1.2".into(),
                    "-sSfL".into(),
                    "https://sh.rustup.rs".into(),
                    "-o".into(),
                    script.display().to_string(),
                ],
                opens_installer: false,
            });
            steps.push(Step {
                what: "Rust",
                program: "/bin/sh",
                args: vec![script.display().to_string(), "-y".into()],
                opens_installer: false,
            });
        }
        steps
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // `has_manager` is false here, so the player is told what to install.
        let _ = t;
        Vec::new()
    }
}

/// Waits (up to `seconds`) for a just-started install to answer.
pub fn wait_until_installed(what: &str, seconds: u64) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        let now = detect();
        let done = match what {
            "Rust" => now.cargo.is_some(),
            "Git" => now.git.is_some(),
            _ => now.clt,
        };
        if done {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// The version of the found git, for the log.
pub fn git_version() -> Option<String> {
    let t = current()?;
    let mut c = command(t.git, "git");
    c.arg("--version");
    let out = c.output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_tool_is_named_with_its_install_page() {
        let hint = by_hand_hint(&["Rust"]);
        assert!(hint.contains("Rust from https://rustup.rs"), "{hint}");
        assert!(hint.contains("press Fix again"));
        assert!(by_hand_hint(&["Git", "Rust"]).contains("git-scm.com"));
    }

    #[test]
    fn winget_nothing_to_upgrade_counts_as_installed() {
        // The exact text from issues #6 and #8.
        assert!(winget_already_installed(
            "\"winget\" ended with exit code: 0x8a15002b"
        ));
        assert!(winget_already_installed("ended with exit code: 0x8A15002B"));
        // Other winget failures still stop the install.
        assert!(!winget_already_installed(
            "\"winget\" ended with exit code: 0x8a150011"
        ));
        assert!(!winget_already_installed("could not start winget"));
    }
}
