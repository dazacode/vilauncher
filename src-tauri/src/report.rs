//! A report for a problem the launcher could not fix: a GitHub issue opened
//! with the log filled in, and the same text to copy. The launcher sends
//! nothing itself; the player reads the issue on GitHub and submits it there.

use serde::Serialize;

/// Where VILauncher's own problems are reported.
pub(crate) const LAUNCHER_REPO: &str = "dazacode/vilauncher";
/// The longest body put in the link; GitHub refuses links that are too long.
const LINK_BODY_LIMIT: usize = 6000;
/// The last lines of the log that go in the report.
const LOG_LINES: usize = 150;

#[derive(Serialize)]
pub(crate) struct Report {
    pub title: String,
    /// The whole report, for Copy (an AI agent can rewrite it from here).
    pub text: String,
    pub launcher_url: String,
    /// The project's own repo, when it is on GitHub.
    pub build_url: Option<String>,
}

pub(crate) struct Facts<'a> {
    pub project: &'a str,
    pub repo: &'a str,
    pub stage: &'a str,
    pub log: &'a [String],
    pub gpus: Vec<String>,
    pub cpu: Option<String>,
    pub ram_gb: Option<f64>,
    /// Text that must not leave the PC: the user's name, folders on this PC.
    pub private: Vec<String>,
}

/// Names and folders that must not leave the PC: the user's name, their home
/// folder, mounted volumes, the host name and `workspace`, on Windows
/// (`C:\Users\alice`) and macOS (`/Users/alice`, `/Volumes/Backup`) alike.
pub(crate) fn private_paths(workspace: &std::path::Path) -> Vec<String> {
    let mut private: Vec<String> = Vec::new();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        private.push(home.to_string_lossy().into_owned());
    }
    for var in ["USER", "USERNAME", "LOGNAME"] {
        if let Ok(value) = std::env::var(var) {
            private.push(value);
        }
    }
    for var in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(var) {
            private.push(value);
        }
    }
    if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("/usr/sbin/scutil")
            .args(["--get", "ComputerName"])
            .output()
        {
            private.push(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            private.extend(
                entries
                    .flatten()
                    .map(|entry| entry.path().to_string_lossy().into_owned()),
            );
        }
    }
    private.push(workspace.to_string_lossy().into_owned());
    private.retain(|s| !s.trim().is_empty());
    private
}

/// Removes every private string from `text`, longest first, and with either slash.
pub(crate) fn scrub(text: &str, private: &[String]) -> String {
    let mut secrets: Vec<String> = private
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() >= 3)
        .flat_map(|s| [s.replace('/', "\\"), s.replace('\\', "/")])
        .collect();
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    secrets.dedup();
    let mut out = text.to_string();
    for secret in secrets {
        out = out.replace(&secret, "<private>");
    }
    out
}

/// The GitHub issue link for `repo` (owner/name) with the title and body filled in.
fn issue_url(repo: &str, title: &str, body: &str) -> Option<String> {
    let mut url = url::Url::parse(&format!("https://github.com/{repo}/issues/new")).ok()?;
    url.query_pairs_mut()
        .append_pair("title", title)
        .append_pair("body", body);
    Some(url.into())
}

/// `owner/name` on GitHub, from a project's repo field (not a link to another site).
fn github_repo(repo: &str) -> Option<&str> {
    let mut parts = repo.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let plain = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    (parts.next().is_none() && !repo.contains("://") && plain(owner) && plain(name)).then_some(repo)
}

/// The longest prefix of `text` within `limit` bytes, cut on a character boundary.
fn cut(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

pub(crate) fn make(f: &Facts) -> Report {
    let scrubbed: Vec<String> = f.log.iter().map(|line| scrub(line, &f.private)).collect();
    let tail = &scrubbed[scrubbed.len().saturating_sub(LOG_LINES)..];
    let stage = scrub(f.stage, &f.private);
    let title = format!("{}: {}", f.project, cut(stage.trim(), 120));

    let gpus = if f.gpus.is_empty() {
        "not found".to_string()
    } else {
        f.gpus.join(", ")
    };
    let cpu = f.cpu.clone().unwrap_or_else(|| "not found".to_string());
    let ram = f
        .ram_gb
        .map(|g| format!("{g} GB"))
        .unwrap_or_else(|| "not found".to_string());
    let text = format!(
        "## What happened\n<!-- What did you do, and what did you expect? -->\n\n\
         ## Launcher\n- VILauncher {}\n- CPU: {cpu}\n- GPU: {gpus}\n- Memory: {ram}\n\n\
         ## Project\n- {} (`{}`)\n- Last stage: {stage}\n\n\
         ## Log (last {} lines)\n```text\n{}\n```\n",
        env!("CARGO_PKG_VERSION"),
        scrub(f.project, &f.private),
        scrub(f.repo, &f.private),
        tail.len(),
        tail.join("\n"),
    );
    let link_body = {
        let short = cut(&text, LINK_BODY_LIMIT);
        if short.len() < text.len() {
            format!("{short}\n\n_The log was cut for this link. Paste the full report from Copy report if it helps._")
        } else {
            text.clone()
        }
    };
    Report {
        launcher_url: issue_url(LAUNCHER_REPO, &title, &link_body).unwrap_or_default(),
        build_url: github_repo(f.repo).and_then(|r| issue_url(r, &title, &link_body)),
        title,
        text,
    }
}

/// The last lines of the project's job log that go in the diagnostic summary.
const SUMMARY_LOG_LINES: usize = 60;

/// What the diagnostic summary is made of. Everything here is already known to the
/// launcher; nothing is probed for the summary. The driver version and the OS build are
/// not recorded anywhere yet, so the summary says so.
pub(crate) struct SummaryFacts<'a> {
    pub launcher_version: &'a str,
    pub project: &'a str,
    /// The project's folder name under `repos\`, which names its log file.
    pub slug: &'a str,
    /// The commit the installed build was made from, when recorded.
    pub commit: Option<String>,
    /// The launch's `NAME=value` lines; only the `VICTORY_*` ones are shown.
    pub switches: &'a str,
    pub gpus: Vec<crate::system::Gpu>,
    pub ram_gb: Option<f64>,
    /// The project's last job log lines (the header is not included), when a log exists.
    pub log_tail: Option<Vec<String>>,
    /// Text that must not leave the PC (see `private_paths`).
    pub private: Vec<String>,
}

/// The `VICTORY_NAME=value` lines of a launch, names and values only. A value that is not
/// a plain token (anything with a slash, a colon or a space) is replaced by a note, so a
/// path never reaches the summary.
fn switch_lines(extra: &str) -> Vec<String> {
    extra
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("VICTORY_"))
        .filter_map(|line| {
            let (name, value) = line.split_once('=').unwrap_or((line, ""));
            let rest = name.strip_prefix("VICTORY_")?;
            let name_ok = !rest.is_empty()
                && rest
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
            if !name_ok {
                return None;
            }
            if value.is_empty() {
                return Some(name.to_string());
            }
            let plain = value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
            Some(if plain {
                format!("{name}={value}")
            } else {
                format!("{name}=(value not shown)")
            })
        })
        .collect()
}

/// The plain-text summary for a bug report, in a fixed order, English throughout.
pub(crate) fn diagnostic_summary(f: &SummaryFacts) -> String {
    let mut lines: Vec<String> = vec![
        "VILauncher diagnostic summary".to_string(),
        format!("Launcher: VILauncher {}", f.launcher_version),
        format!("Project: {}", f.project),
        format!("Project folder: {}", f.slug),
        format!("Commit: {}", f.commit.as_deref().unwrap_or("not recorded")),
        "Switches:".to_string(),
    ];
    let switches = switch_lines(f.switches);
    if switches.is_empty() {
        lines.push("  none set".to_string());
    }
    lines.extend(switches.into_iter().map(|s| format!("  {s}")));
    lines.push("Graphics:".to_string());
    if f.gpus.is_empty() {
        lines.push("  not found".to_string());
    }
    for gpu in &f.gpus {
        let vram = gpu
            .vram_gb
            .map(|gb| format!("{gb} GB"))
            .unwrap_or_else(|| "not found".to_string());
        lines.push(format!(
            "  {}; driver: not recorded; video memory: {vram}",
            gpu.name
        ));
    }
    lines.push(format!(
        "Memory: {}",
        f.ram_gb
            .map(|gb| format!("{gb} GB"))
            .unwrap_or_else(|| "not found".to_string())
    ));
    lines.push("OS build: not recorded".to_string());
    match &f.log_tail {
        Some(tail) => {
            let start = tail.len().saturating_sub(SUMMARY_LOG_LINES);
            lines.push(format!(
                "Log (last {} lines of {}.log):",
                tail.len() - start,
                f.slug
            ));
            lines.extend(tail[start..].iter().cloned());
        }
        None => lines.push("Log: no log yet".to_string()),
    }
    let text = lines.join("\n");
    scrub(&text, &f.private) + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts<'a>(log: &'a [String], repo: &'a str) -> Facts<'a> {
        Facts {
            project: "Victory",
            repo,
            stage: "Fix and verify failed: the game files are missing",
            log,
            gpus: vec!["NVIDIA GeForce RTX 4060".into()],
            cpu: Some("AMD Ryzen 7".into()),
            ram_gb: Some(16.0),
            private: vec!["alice".into(), "C:\\Users\\alice\\Games".into()],
        }
    }

    #[test]
    fn private_names_and_folders_never_reach_the_report() {
        let log = vec![
            "Looking in C:\\Users\\alice\\Games\\GTA".to_string(),
            "Looking in C:/Users/alice/Games/GTA".to_string(),
            "Hello alice".to_string(),
        ];
        let report = make(&facts(&log, "owner/victory"));
        assert!(!report.text.contains("alice"), "{}", report.text);
        assert!(!report.launcher_url.contains("alice"));
        assert!(report.text.contains("<private>"));
    }

    #[test]
    fn links_point_at_the_launcher_and_the_build_repo() {
        let log = vec!["ERROR: missing".to_string()];
        let report = make(&facts(&log, "owner/victory"));
        assert!(report
            .launcher_url
            .starts_with("https://github.com/dazacode/vilauncher/issues/new?"));
        assert!(report
            .build_url
            .as_deref()
            .unwrap()
            .starts_with("https://github.com/owner/victory/issues/new?"));
        // A repo given as a link is not on GitHub's issue form for this owner.
        let link = make(&facts(&log, "https://example.com/x.git"));
        assert!(link.build_url.is_none());
    }

    #[test]
    fn long_logs_are_cut_for_the_link_but_kept_in_the_text() {
        let log: Vec<String> = (0..2000).map(|i| format!("line {i} ünïcode")).collect();
        let report = make(&facts(&log, "owner/victory"));
        assert!(report.text.contains("line 1999"));
        assert!(!report.launcher_url.contains("line 1999"));
        assert!(report.launcher_url.len() < 12_000);
    }

    #[test]
    fn macos_paths_and_names_are_scrubbed() {
        let private = vec![
            "/Users/esperance".to_string(),
            "/Volumes/Backup".to_string(),
            "esperance".to_string(),
            "Esperances-MacBook-Pro".to_string(),
        ];
        let text = "warning in /Users/esperance/Library/Application Support/VILauncher (Esperances-MacBook-Pro)";
        let scrubbed = scrub(text, &private);
        assert!(!scrubbed.contains("esperance"), "{scrubbed}");
        assert!(!scrubbed.contains("/Volumes/Backup"));
        assert!(scrubbed.contains("<private>"));
        // The home folder is cut whole, before the bare name can split it.
        assert!(scrubbed.starts_with("warning in <private>/Library"));
    }

    #[test]
    fn scrub_handles_both_slashes_and_longest_first() {
        let private = vec!["C:\\Users\\alice".to_string(), "alice".to_string()];
        assert_eq!(
            scrub("C:/Users/alice/x alice", &private),
            "<private>/x <private>"
        );
    }

    #[test]
    fn the_summary_has_its_fields_in_a_fixed_order() {
        let log = vec!["Playing".to_string(), "frame 1".to_string()];
        let text = diagnostic_summary(&SummaryFacts {
            launcher_version: "1.1.0",
            project: "Victory",
            slug: "owner_victory",
            commit: Some("abc1234".into()),
            switches: "VICTORY_A=1\nVICTORY_B=2",
            gpus: vec![crate::system::Gpu {
                name: "NVIDIA GeForce RTX 4060".into(),
                vendor: crate::system::Vendor::Nvidia,
                vram_gb: Some(8.0),
            }],
            ram_gb: Some(16.0),
            log_tail: Some(log),
            private: vec![],
        });
        let order = [
            "VILauncher diagnostic summary",
            "Launcher: VILauncher 1.1.0",
            "Project: Victory",
            "Project folder: owner_victory",
            "Commit: abc1234",
            "Switches:",
            "Graphics:",
            "Memory: 16 GB",
            "OS build: not recorded",
            "Log (last 2 lines of owner_victory.log):",
        ];
        let mut at = 0;
        for marker in order {
            let found = text[at..]
                .find(marker)
                .unwrap_or_else(|| panic!("{marker} out of order:\n{text}"));
            at += found + marker.len();
        }
        assert!(text.contains("driver: not recorded; video memory: 8 GB"), "{text}");
        assert!(text.contains("  VICTORY_A=1\n  VICTORY_B=2"), "{text}");
        assert!(text.ends_with("frame 1\n"));
    }

    #[test]
    fn unknown_facts_say_not_recorded_or_not_found() {
        let text = diagnostic_summary(&SummaryFacts {
            launcher_version: "1.1.0",
            project: "Victory",
            slug: "owner_victory",
            commit: None,
            switches: "PATH=C:\\x\nVICTORY_ONLY",
            gpus: vec![],
            ram_gb: None,
            log_tail: None,
            private: vec![],
        });
        assert!(text.contains("Commit: not recorded"));
        assert!(text.contains("Graphics:\n  not found"));
        assert!(text.contains("Memory: not found"));
        assert!(text.contains("Log: no log yet"));
        // Only the VICTORY_ line is shown, and without a value it shows the name alone.
        assert!(text.contains("Switches:\n  VICTORY_ONLY\nGraphics:"), "{text}");
        assert!(!text.contains("PATH"));
    }

    #[test]
    fn switch_values_that_are_paths_and_home_folders_are_never_shown() {
        let private = vec!["C:\\Users\\alice".to_string(), "alice".to_string()];
        let text = diagnostic_summary(&SummaryFacts {
            launcher_version: "1.1.0",
            project: "Victory",
            slug: "owner_victory",
            commit: None,
            switches: "VICTORY_DIR=C:\\Users\\alice\\Games\nVICTORY_X=1 2",
            gpus: vec![],
            ram_gb: None,
            log_tail: Some(vec!["Looking in C:/Users/alice/Games".to_string()]),
            private,
        });
        assert!(!text.contains("alice"), "{text}");
        assert!(text.contains("Looking in <private>/Games"), "{text}");
        assert!(text.contains("VICTORY_DIR=(value not shown)"));
        assert!(text.contains("VICTORY_X=(value not shown)"));
        assert!(text.contains("<private>"));
    }
}
