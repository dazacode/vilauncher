//! Launch presets: saved launch arguments and switches, shared as JSON or
//! read from a project's PowerShell launch scripts (scripts/play*.ps1).

use super::{project_key, repo_dir, LaunchPreset, Project, ProjectLaunch};
use crate::i18n::localize;
use crate::script_preset;
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_PRESET_BYTES: usize = 128 * 1024;

/// What Play offers before it starts, the first time: the project's own
/// launch scripts and the presets saved for it. `recommended` is preselected.
#[derive(Serialize)]
pub(crate) struct PlayChoice {
    pub(crate) options: Vec<PlayOption>,
    pub(crate) recommended: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct PlayOption {
    pub(crate) title: String,
    pub(crate) detail: String,
}

pub(crate) struct PlayPresetPicker {
    pub(crate) options: Vec<LaunchPreset>,
    pub(crate) recommended: Option<usize>,
}

impl PlayPresetPicker {
    pub(crate) fn for_project(
        project: &Project,
        saved: &[LaunchPreset],
        launch: Option<&ProjectLaunch>,
    ) -> Option<Self> {
        if launch.is_some_and(ProjectLaunch::has_confirmed_preset) {
            return None;
        }
        let standalone = project.runs_standalone();
        let repo = project_key(&project.repo);
        let mut options = if standalone {
            Vec::new()
        } else {
            scripts(&project_scripts(project), &project.name)
        };
        let recommended = options
            .first()
            .filter(|preset| is_custom_script(&preset.name))
            .map(|_| 0);
        options.extend(
            saved
                .iter()
                .filter(|preset| {
                    preset.projects.contains(&repo)
                        && (!standalone || preset.switches.is_none())
                        && preset.repo_script.is_none()
                })
                .cloned(),
        );
        if recommended.is_none() && options.len() < 2 {
            return None;
        }
        Some(Self {
            options,
            recommended,
        })
    }

    /// The choices as the window shows them, in the player's language (a
    /// saved preset's own name stays as the player wrote it).
    pub(crate) fn describe(&self) -> PlayChoice {
        let options = self
            .options
            .iter()
            .enumerate()
            .map(|(index, preset)| {
                let (title, detail) = match preset.repo_script.as_deref() {
                    Some(script) if is_custom_script(script) => (
                        localize(msg_str!("Detected custom script")),
                        if self.recommended == Some(index) {
                            msg!("{script}.ps1 · Preselected for this build")
                        } else {
                            msg!("{script}.ps1 · Found in the project")
                        },
                    ),
                    Some(script) => (
                        localize(msg_str!("Standard repo launch")),
                        msg!("{script}.ps1 · The repository's standard settings"),
                    ),
                    None => (preset.name.clone(), msg_str!("A saved preset.").to_owned()),
                };
                PlayOption {
                    title,
                    detail: localize(&detail),
                }
            })
            .collect();
        PlayChoice {
            options,
            recommended: self.recommended,
        }
    }
}

pub(crate) fn detected_preset(project: &Project) -> Option<LaunchPreset> {
    scripts(&project_scripts(project), &project.name)
        .into_iter()
        .next()
        .filter(|preset| is_custom_script(&preset.name))
}

fn is_custom_script(stem: &str) -> bool {
    stem.eq_ignore_ascii_case("scarlet-fps")
        || (stem.to_ascii_lowercase().starts_with("play-") && stem.len() > "play-".len())
}

fn project_scripts(project: &Project) -> std::path::PathBuf {
    let root = repo_dir(&project.repo);
    super::cargo_workspace(&root)
        .ok()
        .map(|workspace| workspace.join("scripts"))
        .filter(|folder| folder.is_dir())
        .unwrap_or_else(|| root.join("scripts"))
}

fn scripts(folder: &Path, project_name: &str) -> Vec<LaunchPreset> {
    let mut files: Vec<_> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_str()?;
            let supported = path.extension()?.eq_ignore_ascii_case("ps1")
                && (stem.eq_ignore_ascii_case("play") || is_custom_script(stem));
            supported.then_some(path)
        })
        .collect();
    let name = project_name.to_ascii_lowercase();
    let matching = format!("play-{name}");
    let fork = format!("play-{}", name.strip_prefix("victory-").unwrap_or(&name));
    let documented = folder.parent().and_then(super::documented_scarlet_script);
    files.sort_by_key(|path| {
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let rank = if documented == Some(stem.as_str()) {
            0
        } else if stem == matching || stem == fork {
            1
        } else if is_custom_script(&stem) {
            2
        } else {
            3
        };
        (rank, stem)
    });
    files
        .iter()
        .take(32)
        .filter_map(|path| {
            let mut preset = import_file(path).ok()?;
            preset.repo_script = Some(preset.name.clone());
            Some(preset)
        })
        .collect()
}

pub(crate) fn apply_to_profile(
    presets: &mut Vec<LaunchPreset>,
    launch: &mut ProjectLaunch,
    project_key: &str,
    mut preset: LaunchPreset,
) -> Result<String, String> {
    let existing = if preset.repo_script.is_some() {
        presets.iter().position(|saved| {
            saved.repo_script == preset.repo_script
                && saved.projects.iter().any(|key| key == project_key)
        })
    } else {
        presets.iter().position(|saved| saved.name == preset.name)
    };
    let index = match existing {
        Some(index) => index,
        None => {
            preset.projects = vec![project_key.to_owned()];
            store(presets, preset)?
        }
    };
    let saved = &mut presets[index];
    if !saved.projects.iter().any(|key| key == project_key) {
        saved.projects.push(project_key.to_owned());
    }
    saved.apply(launch);
    launch.selected_preset = Some(saved.name.clone());
    launch.preset_choice_confirmed = true;
    Ok(saved.name.clone())
}

pub(crate) fn store(
    presets: &mut Vec<LaunchPreset>,
    mut preset: LaunchPreset,
) -> Result<usize, String> {
    preset.name = preset.name.trim().to_owned();
    encode(&preset)?;
    if presets
        .iter()
        .any(|saved| saved.name.eq_ignore_ascii_case(&preset.name))
    {
        let base = preset.name.clone();
        for number in 2.. {
            let candidate = format!("{base} ({number})");
            if !presets
                .iter()
                .any(|saved| saved.name.eq_ignore_ascii_case(&candidate))
            {
                preset.name = candidate;
                break;
            }
        }
    }
    presets.push(preset);
    Ok(presets.len() - 1)
}

fn read_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| msg!("Could not open preset: {error}"))?;
    let mut text = String::new();
    file.take((MAX_PRESET_BYTES + 1) as u64)
        .read_to_string(&mut text)
        .map_err(|error| msg!("Could not read preset: {error}"))?;
    Ok(text)
}

pub(crate) fn import_file(path: &Path) -> Result<LaunchPreset, String> {
    let text = read_file(path)?;
    if text.len() > MAX_PRESET_BYTES {
        return Err(msg_str!("Preset or script exceeds 128 KiB.").into());
    }
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ps1"))
    {
        return decode(&text);
    }
    let (switches, options) = script_preset::parse(&text)?;
    Ok(LaunchPreset {
        name: path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Launch script")
            .into(),
        args: String::new(),
        switches: Some(switches),
        script_options: Some(options),
        projects: Vec::new(),
        repo_script: None,
    })
}

pub(crate) fn decode(json: &str) -> Result<LaunchPreset, String> {
    if json.len() > MAX_PRESET_BYTES {
        return Err(msg_str!("Preset exceeds 128 KiB.").into());
    }
    serde_json::from_str(json.trim_start_matches('\u{feff}'))
        .map_err(|error| msg!("Invalid preset JSON: {error}"))
}

pub(crate) fn encode(preset: &LaunchPreset) -> Result<String, String> {
    if preset.name.trim().is_empty() || preset.name.chars().any(char::is_control) {
        return Err(msg_str!("Enter a preset name on one line.").into());
    }
    if preset.args.contains('\0')
        || preset
            .switches
            .as_deref()
            .is_some_and(|switches| switches.contains('\0'))
    {
        return Err(msg_str!("Launch settings contain a null character.").into());
    }
    let mut portable = preset.clone();
    portable.projects.clear();
    portable.repo_script = None;
    let json = serde_json::to_string_pretty(&portable).map_err(|error| error.to_string())?;
    if json.len() > MAX_PRESET_BYTES {
        return Err(msg_str!("Preset exceeds 128 KiB.").into());
    }
    Ok(json)
}

#[cfg(test)]
#[test]
fn shared_presets_preserve_quotes_empty_overrides_and_duplicate_names() {
    let original = LaunchPreset {
        name: "Scarlett".into(),
        args: r#"--strict -- -name "two words""#.into(),
        switches: Some(String::new()),
        script_options: None,
        projects: vec!["owner/private-repo".into()],
        repo_script: None,
    };
    let mut saved = vec![original.clone()];
    let imported = decode(&encode(&original).unwrap()).unwrap();
    let index = store(&mut saved, imported).unwrap();
    assert_eq!(saved[index].name, "Scarlett (2)");
    assert_eq!(saved[index].args, original.args);
    assert_eq!(saved[index].switches.as_deref(), Some(""));
    assert!(saved[index].projects.is_empty());
    assert!(decode("just raw arguments").is_err());
}

#[cfg(test)]
#[test]
fn accepting_a_detected_preset_saves_it_to_the_profile_without_duplicates() {
    let mut presets = Vec::new();
    let mut launch = ProjectLaunch::default();
    let preset = LaunchPreset {
        name: "play-scarlett".into(),
        args: String::new(),
        switches: Some("VICTORY_ASYNC_SUBMIT=1".into()),
        script_options: Some(script_preset::Options::default()),
        projects: Vec::new(),
        repo_script: Some("play-scarlett".into()),
    };
    for _ in 0..2 {
        apply_to_profile(&mut presets, &mut launch, "owner/scarlett", preset.clone()).unwrap();
    }
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].projects, ["owner/scarlett"]);
    assert_eq!(launch.selected_preset.as_deref(), Some("play-scarlett"));
    assert!(launch.has_confirmed_preset());
    assert_eq!(launch.switches, preset.switches);
    assert!(decode(&encode(&presets[0]).unwrap())
        .unwrap()
        .repo_script
        .is_none());
}

#[cfg(test)]
#[test]
fn prefills_the_matching_custom_script_ahead_of_the_standard_script() {
    let folder =
        std::env::temp_dir().join(format!("vilauncher-play-presets-{}", std::process::id()));
    std::fs::create_dir(&folder).unwrap();
    let valid = "$settings = @{\nVICTORY_LOG = 'warn'\n}";
    let fixtures = [
        ("play.ps1", valid),
        ("play-scarlett.ps1", valid),
        ("play-alt.ps1", valid),
        ("play-dynamic.ps1", "Write-Host 'No preset here'"),
        ("build.ps1", valid),
    ];
    for (name, contents) in fixtures {
        std::fs::write(folder.join(name), contents).unwrap();
    }
    let options = scripts(&folder, "victory-scarlett");
    let names: Vec<_> = options.iter().map(|preset| preset.name.as_str()).collect();
    assert_eq!(names, ["play-scarlett", "play-alt", "play"]);
    assert!(is_custom_script(names[0]));
    for (name, _) in fixtures {
        std::fs::remove_file(folder.join(name)).unwrap();
    }
    std::fs::remove_dir(folder).unwrap();
}

#[test]
fn documented_scarlet_scripts_take_priority_and_use_release() {
    let root = std::env::temp_dir().join(format!("vilauncher-scarlet-{}", std::process::id()));
    let folder = root.join("scripts");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(root.join("Cargo.toml"), "[profile.diag]").unwrap();
    for stem in ["play-scarlett", "play-3070", "scarlet-fps"] {
        std::fs::write(
            folder.join(format!("{stem}.ps1")),
            "$settings = @{\nVICTORY_LOG = 'warn'\n}",
        )
        .unwrap();
    }
    assert_eq!(super::detect_build_profile(&root), "release");
    for stem in ["scarlet-fps", "play-3070"] {
        std::fs::write(
            root.join("README.md"),
            format!(
                "cargo build --release --bin victory\npowershell -File .\\scripts\\{stem}.ps1\n"
            ),
        )
        .unwrap();
        assert_eq!(super::detect_build_profile(&root), "release");
        assert_eq!(scripts(&folder, "Victory-Scarlett")[0].name, stem);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(test)]
#[test]
fn accepting_current_settings_or_a_preset_skips_future_prompts() {
    let project = Project {
        name: "Fixture".into(),
        repo: "test/preset-picker-fixture".into(),
        branch: None,
        gpu: Vec::new(),
        fps: None,
        render: None,
        verified: false,
        summary: String::new(),
        custom: true,
        local_exe: None,
        archive_exe: None,
        highlights: Vec::new(),
        blockers: Vec::new(),
        listing: Default::default(),
    };
    let presets = ["First", "Second"].map(|name| LaunchPreset {
        name: name.into(),
        args: String::new(),
        switches: None,
        script_options: None,
        projects: vec![project.repo.clone()],
        repo_script: None,
    });
    assert!(PlayPresetPicker::for_project(&project, &presets, None).is_some());
    let current = ProjectLaunch {
        preset_choice_confirmed: true,
        ..ProjectLaunch::default()
    };
    assert!(PlayPresetPicker::for_project(&project, &presets, Some(&current)).is_none());
    let previous = ProjectLaunch {
        confirmed_repo_script: Some("play-scarlett".into()),
        ..ProjectLaunch::default()
    };
    assert!(PlayPresetPicker::for_project(&project, &presets, Some(&previous)).is_none());
}
