use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

const MAX_EXPANDED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

fn valid_component(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    !part.is_empty()
        && part != "."
        && part != ".."
        && !part.ends_with([' ', '.'])
        && !part
            .chars()
            .any(|ch| ch.is_control() || "<>:\"/\\|?*".contains(ch))
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        && !stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(
                    number,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}

/// What a ZIP holds: a program to run, or a source tree (a Cargo workspace) to build.
#[derive(Debug, PartialEq)]
pub enum Extracted {
    Program(PathBuf),
    Source,
}

pub fn extract(input: &Path, destination: &Path) -> Result<Extracted, String> {
    let file = File::open(input).map_err(|error| error.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    fs::create_dir(destination).map_err(|error| error.to_string())?;
    let result = extract_contents(&mut zip, destination);
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result.map_err(|error| error.to_string())
}

fn extract_contents(zip: &mut zip::ZipArchive<File>, destination: &Path) -> io::Result<Extracted> {
    if zip.len() > 50_000 {
        return Err(io::Error::other(msg_str!(
            "The build ZIP contains too many entries."
        )));
    }
    let mut remaining = MAX_EXPANDED_BYTES;
    let mut executables = Vec::new();
    let mut bundles = Vec::new();
    // A Cargo.toml at the top or one folder down: the ZIP holds a source tree.
    let mut manifest = false;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name()?;
        let name = name.trim_end_matches('/');
        if !name.split('/').all(valid_component) {
            return Err(io::Error::other(msg_str!(
                "The build ZIP contains an unsafe file path."
            )));
        }
        let relative = PathBuf::from(name);
        let output = destination.join(&relative);
        if relative
            .file_name()
            .is_some_and(|file| file == "Cargo.toml")
            && relative.components().count() <= 2
        {
            manifest = true;
        }
        if entry.is_symlink() {
            // macOS bundles carry symlinks; they are recreated where the system has them.
            let mut target = String::new();
            io::Read::read_to_string(&mut entry, &mut target)?;
            let target = target.trim();
            if target.is_empty()
                || target.starts_with('/')
                || !target.split('/').all(valid_component)
            {
                return Err(io::Error::other(msg_str!(
                    "The build ZIP contains an unsafe symbolic link."
                )));
            }
            fs::create_dir_all(output.parent().expect("a link has a parent"))?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, &output)?;
            continue;
        }
        if entry.is_dir() {
            if name.ends_with(".app") {
                bundles.push(relative.clone());
            }
            fs::create_dir_all(output)?;
            continue;
        }
        if entry.size() > remaining {
            return Err(io::Error::other(msg_str!(
                "The extracted build exceeds 8 GiB."
            )));
        }
        fs::create_dir_all(output.parent().expect("extracted file has a parent"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)?;
        let written = io::copy(&mut io::Read::take(&mut entry, remaining + 1), &mut file)?;
        remaining = remaining
            .checked_sub(written)
            .ok_or_else(|| io::Error::other(msg_str!("The extracted build exceeds 8 GiB.")))?;
        // A program: a Windows exe, a Mach-O binary or a script. Data files
        // (dlls, rpf) are not even read for their first bytes.
        let executable = relative
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            || crate::platform::binary_kind(&output) != crate::platform::Binary::Unknown;
        if executable {
            crate::platform::make_executable(&output);
            executables.push(relative);
        }
    }
    // Apple's quarantine would block a freshly unpacked game on first start.
    crate::platform::clear_quarantine(destination);
    let mut victory = executables.iter().filter(|path| is_victory(path));
    let chosen = match (victory.next(), victory.next(), executables.as_slice()) {
        (Some(exe), None, _) | (None, None, [exe]) => Ok(Extracted::Program(exe.clone())),
        (None, None, []) if bundles.len() == 1 => Ok(Extracted::Program(bundles[0].clone())),
        // A source tree has its own scripts (git hooks, build helpers) that look like
        // programs; it is built rather than run, so it is not refused for them.
        _ if manifest => Ok(Extracted::Source),
        (None, None, []) if bundles.is_empty() => {
            Err(io::Error::other(msg_str!("The ZIP contains no program.")))
        }
        (None, None, []) => Err(io::Error::other(msg_str!(
            "The ZIP contains multiple possible launch programs."
        ))),
        _ => Err(io::Error::other(msg_str!(
            "The ZIP contains multiple possible launch executables."
        ))),
    }?;
    if let Extracted::Program(exe) = &chosen {
        // Bundles keep their own inner permissions; a lone binary is made runnable.
        crate::platform::make_executable(&destination.join(exe));
    }
    Ok(chosen)
}

/// Whether a path is the victory program, named either way.
pub fn is_victory(executable: &Path) -> bool {
    executable
        .file_stem()
        .is_some_and(|name| name.eq_ignore_ascii_case("victory"))
}

/// Read only literal defaults from a companion script. The script itself is never run.
pub fn launch_defaults(directory: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut scripts: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            (name.starts_with("play-") && name.ends_with(".cmd")).then_some(path)
        })
        .collect();
    scripts.sort();
    let [script] = scripts.as_slice() else {
        return Vec::new();
    };
    if !script
        .metadata()
        .is_ok_and(|metadata| metadata.len() <= 128 * 1024)
    {
        return Vec::new();
    }
    let Ok(contents) = fs::read_to_string(script) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let (guard, assignment) = line
                .trim()
                .strip_prefix("if not defined ")?
                .split_once(" set \"")?;
            let (name, value) = assignment.strip_suffix('"')?.split_once('=')?;
            (guard == name
                && name.starts_with("VICTORY_")
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte)))
            .then(|| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The `VICTORY_*` settings a fork's own launch `.bat` sets at its top level, e.g.
/// `set "VICTORY_VSYNC=0"`. Indented lines (the bat's profile blocks inside `if`s) and
/// values that expand a `%variable%` are skipped. Read only; the script is never run.
pub fn batch_defaults(directory: &Path) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<Vec<(String, String)>> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".bat")
        })
        .filter(|entry| {
            entry
                .metadata()
                .is_ok_and(|m| m.is_file() && m.len() <= 128 * 1024)
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .map(|text| parse_batch_defaults(&text))
        .filter(|settings| !settings.is_empty())
        .collect();
    // Only a single launch script that sets settings is the project's; several are ambiguous.
    if found.len() == 1 {
        found.remove(0)
    } else {
        Vec::new()
    }
}

fn parse_batch_defaults(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            let assignment = line.strip_prefix("set \"")?.strip_suffix('"')?;
            let (name, value) = assignment.split_once('=')?;
            (name.starts_with("VICTORY_")
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                && !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte)))
            .then(|| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_forks_top_level_victory_settings_are_read_and_its_profile_blocks_are_not() {
        // Indentation is written out: a `\` line continuation would strip it.
        let text = [
            "@echo off",
            "set \"VICTORY_LOG=info\"",
            "set \"VICTORY_FSR_PC=1\"",
            "set \"VICTORY_FRAMEGEN_AHEAD=1\"",
            "set \"VICTORY_FIDELITYFX_FRAMEGEN=%XEN_FRAMEGEN%\"",
            "set \"VICTORY_KBM_PC=1\"",
            "set \"VICTORY_SKIP_PIPELINES=240cb516c1af0bc3\"",
            "if /I \"%XEN_PROFILE%\"==\"safe\" (",
            "    set \"VICTORY_VSYNC=1\"",
            ")",
            "set \"FG_LABEL=ON\"",
        ]
        .join("\r\n");
        assert_eq!(
            parse_batch_defaults(&text),
            vec![
                ("VICTORY_LOG".to_string(), "info".to_string()),
                ("VICTORY_FSR_PC".to_string(), "1".to_string()),
                ("VICTORY_FRAMEGEN_AHEAD".to_string(), "1".to_string()),
                ("VICTORY_KBM_PC".to_string(), "1".to_string()),
                (
                    "VICTORY_SKIP_PIPELINES".to_string(),
                    "240cb516c1af0bc3".to_string()
                ),
            ]
        );
    }

    #[test]
    fn imports_nested_victory_with_support_files_and_rejects_traversal() {
        let root =
            std::env::temp_dir().join(format!("vilauncher-build-zip-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let archive = root.join("build.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("cool_fox/victory.exe", options).unwrap();
        writer.write_all(b"fixture").unwrap();
        writer
            .start_file("cool_fox/Play-cool_fox.cmd", options)
            .unwrap();
        writer.write_all(b"if not defined VICTORY_RELAY set \"VICTORY_RELAY=2\"\n+if not defined VICTORY_DIR set \"VICTORY_DIR=%LOCALAPPDATA%\"\n").unwrap();
        writer.finish().unwrap();

        let destination = root.join("build");
        assert_eq!(
            extract(&archive, &destination).unwrap(),
            Extracted::Program(PathBuf::from("cool_fox/victory.exe"))
        );
        assert_eq!(
            launch_defaults(&destination.join("cool_fox")),
            [("VICTORY_RELAY".into(), "2".into())]
        );

        // A source tree whose scripts look like programs is imported for building.
        let source = root.join("source.zip");
        let mut writer = zip::ZipWriter::new(File::create(&source).unwrap());
        writer.start_file("tree-src/Cargo.toml", options).unwrap();
        writer.write_all(b"[workspace]\n").unwrap();
        writer
            .start_file("tree-src/.githooks/pre-commit", options)
            .unwrap();
        writer.write_all(b"#!/bin/sh\n").unwrap();
        writer
            .start_file("tree-src/launcher/build.sh", options)
            .unwrap();
        writer.write_all(b"#!/bin/sh\n").unwrap();
        writer.finish().unwrap();
        assert_eq!(
            extract(&source, &root.join("source")).unwrap(),
            Extracted::Source
        );

        let malicious = root.join("unsafe.zip");
        let mut writer = zip::ZipWriter::new(File::create(&malicious).unwrap());
        writer.start_file("../escape.exe", options).unwrap();
        writer.finish().unwrap();
        assert!(extract(&malicious, &root.join("unsafe")).is_err());
        assert!(!root.join("unsafe").exists());
        assert!(!root.join("escape.exe").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
