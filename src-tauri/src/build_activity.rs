use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
pub struct BuildActivity {
    pub updated: u64,
    pub ran: Option<bool>,
    #[serde(default)]
    pub last_run: Option<u64>,
    #[serde(default)]
    pub new_install: bool,
    commit: String,
}

impl BuildActivity {
    pub fn read(target: &Path) -> Option<Self> {
        let marker = target.join("built-from.txt");
        let commit = fs::read_to_string(&marker).ok()?.trim().to_owned();
        let history = Self::history(target);
        history
            .as_ref()
            .filter(|activity| activity.commit == commit)
            .cloned()
            .or_else(|| {
                let updated = seconds(fs::metadata(marker).ok()?.modified().ok()?);
                Some(Self {
                    updated,
                    ran: None,
                    last_run: history.and_then(|activity| activity.last_run),
                    new_install: false,
                    commit,
                })
            })
    }

    fn history(target: &Path) -> Option<Self> {
        let bytes = fs::read(target.join("build-activity.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn record_build(target: &Path, updating: bool) -> io::Result<()> {
        let commit = fs::read_to_string(target.join("built-from.txt"))?
            .trim()
            .to_owned();
        Self {
            updated: seconds(SystemTime::now()),
            ran: Some(false),
            last_run: Self::history(target).and_then(|activity| activity.last_run),
            new_install: !updating,
            commit,
        }
        .write(target)
    }

    pub fn record_run(target: &Path) -> io::Result<()> {
        if let Some(mut activity) = Self::read(target) {
            activity.ran = Some(true);
            activity.last_run = Some(seconds(SystemTime::now()));
            activity.write(target)?;
        }
        Ok(())
    }

    fn write(&self, target: &Path) -> io::Result<()> {
        let temporary = target.join(format!("build-activity-{}.tmp", std::process::id()));
        fs::write(&temporary, serde_json::to_vec(self)?)?;
        fs::rename(temporary, target.join("build-activity.json"))
    }

    /// When the build was made, as the Activity tab says it: "Updated 3 days
    /// ago", or "Installed …" for a first install.
    pub fn updated_ago(&self) -> String {
        let when = self.ago();
        if self.new_install {
            msg!("Installed {when}")
        } else {
            msg!("Updated {when}")
        }
    }

    /// How long ago the build was made: "3 days ago".
    pub fn ago(&self) -> String {
        let minutes = seconds(SystemTime::now()).saturating_sub(self.updated) / 60;
        match minutes {
            0 => msg!("just now"),
            1 => msg!("1 minute ago"),
            2..=59 => msg!("{minutes} minutes ago"),
            60..=119 => msg!("1 hour ago"),
            120..=1439 => {
                let hours = minutes / 60;
                msg!("{hours} hours ago")
            }
            1440..=2879 => msg!("1 day ago"),
            _ => {
                let days = minutes / 1440;
                msg!("{days} days ago")
            }
        }
    }

    /// Whether the first run of this build is still to come. Its first minutes compile
    /// about a thousand pipelines, so a slow first run is not representative.
    pub fn warm_up_pending(&self) -> bool {
        warm_up_pending(self.updated, self.last_run)
    }

    pub fn pending_label(&self) -> Option<&'static str> {
        (self.ran == Some(false)).then_some(if self.new_install {
            msg_str!("New install · Not run yet")
        } else {
            msg_str!("Updated · Not run yet")
        })
    }
}

#[cfg(windows)]
pub fn local_time(seconds: u64) -> Option<String> {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

    let ticks = seconds
        .checked_add(11_644_473_600)?
        .checked_mul(10_000_000)?;
    let file_time = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    if unsafe { FileTimeToSystemTime(&file_time, &mut utc) } == 0
        || unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) } == 0
    {
        return None;
    }
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute
    ))
}

/// The warm-up decision: a build made after its last recorded run (or never run) is still
/// to get its first run, and the notice says so. Equal times count as run.
pub fn warm_up_pending(built: u64, last_run: Option<u64>) -> bool {
    last_run.is_none_or(|run| run < built)
}

fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
#[test]
fn the_first_run_after_a_build_is_flagged_until_a_run_follows_it() {
    // Never run: pending. A run after the build: done. A build after the last run: pending.
    assert!(warm_up_pending(1_000, None));
    assert!(!warm_up_pending(1_000, Some(1_500)));
    assert!(
        !warm_up_pending(1_000, Some(1_000)),
        "a run in the same second as the build counts"
    );
    assert!(
        warm_up_pending(2_000, Some(1_500)),
        "an update after the last run is pending again"
    );
}

#[cfg(test)]
#[test]
fn tracks_runs_only_after_a_build_or_launch_is_recorded() {
    let target = std::env::temp_dir().join(format!("vilauncher-activity-{}", std::process::id()));
    fs::create_dir(&target).unwrap();
    assert!(BuildActivity::read(&target).is_none());
    fs::write(target.join("built-from.txt"), "existing").unwrap();
    assert_eq!(BuildActivity::read(&target).unwrap().ran, None);
    BuildActivity::record_build(&target, false).unwrap();
    let installed = BuildActivity::read(&target).unwrap();
    assert_eq!(installed.ran, Some(false));
    assert_eq!(installed.pending_label(), Some("New install · Not run yet"));
    BuildActivity::record_run(&target).unwrap();
    let played = BuildActivity::read(&target).unwrap();
    assert_eq!(played.ran, Some(true));
    assert_eq!(played.updated, installed.updated);
    fs::write(target.join("built-from.txt"), "updated").unwrap();
    BuildActivity::record_build(&target, true).unwrap();
    let updated = BuildActivity::read(&target).unwrap();
    assert_eq!(updated.pending_label(), Some("Updated · Not run yet"));
    assert_eq!(updated.last_run, played.last_run);
    fs::remove_dir_all(target).unwrap();
}
