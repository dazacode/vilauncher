//! Git, Rust (cargo) and the C++ build tools that building a project needs:
//! found on the PATH or in the folders their installers use (a PATH changed
//! after Windows started does not reach programs started before), and put
//! on this PC with winget when the player asks.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[derive(Clone, Default, Debug)]
pub struct Tools {
    pub git: Option<PathBuf>,
    pub cargo: Option<PathBuf>,
    /// The C++ build tools (MSVC): Rust needs their linker.
    pub msvc: bool,
}

impl Tools {
    pub fn ready(&self) -> bool {
        self.git.is_some() && self.cargo.is_some() && self.msvc
    }

    /// What is missing, for people.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut m = Vec::new();
        if self.git.is_none() {
            m.push("Git");
        }
        if self.cargo.is_none() {
            m.push("Rust");
        }
        if !self.msvc {
            m.push("C++ build tools");
        }
        m
    }
}

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
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::hide_window(&mut c);
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

fn msvc_installed() -> bool {
    // Every Visual Studio install (any edition, Build Tools too), from the
    // installer's own list, else the usual folders.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(pf86) = env_path("ProgramFiles(x86)") {
        let vswhere = pf86
            .join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe");
        if vswhere.is_file() {
            let mut c = Command::new(vswhere);
            c.args(["-all", "-products", "*", "-property", "installationPath"]);
            crate::hide_window(&mut c);
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
    // The C++ tools are there if a version of them has its 64-bit linker.
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

/// Looks for the tools now.
pub fn detect() -> Tools {
    let pf = env_path("ProgramFiles");
    let pf86 = env_path("ProgramFiles(x86)");
    let local = env_path("LOCALAPPDATA");
    let home = env_path("USERPROFILE");
    let git = find(
        "git",
        &[
            pf.as_ref()
                .map(|p| p.join("Git").join("cmd").join("git.exe")),
            pf86.as_ref()
                .map(|p| p.join("Git").join("cmd").join("git.exe")),
            local
                .as_ref()
                .map(|p| p.join("Programs").join("Git").join("cmd").join("git.exe")),
        ],
    );
    let cargo = find(
        "cargo",
        &[
            env_path("CARGO_HOME").map(|p| p.join("bin").join("cargo.exe")),
            home.as_ref()
                .map(|p| p.join(".cargo").join("bin").join("cargo.exe")),
        ],
    );
    let t = Tools {
        git,
        cargo,
        msvc: msvc_installed(),
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
    if !dirs.is_empty() {
        if let Some(path) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&path));
        }
        if let Ok(joined) = std::env::join_paths(dirs) {
            c.env("PATH", joined);
        }
    }
    c
}

pub fn git() -> Command {
    command(current().and_then(|t| t.git), "git")
}

pub fn cargo() -> Command {
    command(current().and_then(|t| t.cargo), "cargo")
}

/// Whether winget (the Windows package manager) is there.
pub fn has_winget() -> bool {
    answers(Path::new("winget"))
}

/// The winget installs for what `t` lacks: (what, arguments).
pub fn install_steps(t: &Tools) -> Vec<(&'static str, Vec<String>)> {
    let common = [
        "--exact",
        "--source",
        "winget",
        "--accept-package-agreements",
        "--accept-source-agreements",
    ];
    let mut steps = Vec::new();
    let mut step = |what: &'static str, id: &str, extra: &[&str]| {
        let mut a: Vec<String> = vec!["install".into(), "--id".into(), id.into()];
        a.extend(common.iter().map(|s| s.to_string()));
        a.extend(extra.iter().map(|s| s.to_string()));
        steps.push((what, a));
    };
    if t.git.is_none() {
        step("Git", "Git.Git", &["--silent"]);
    }
    if !t.msvc {
        step(
            "C++ build tools",
            "Microsoft.VisualStudio.2022.BuildTools",
            &["--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"],
        );
    }
    if t.cargo.is_none() {
        step("Rust", "Rustlang.Rustup", &["--silent"]);
    }
    steps
}
