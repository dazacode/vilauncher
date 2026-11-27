use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FILES: usize = 4096;

fn invalid(message: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message)
}

fn safe_component(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_~".contains(&byte))
        && !matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit())
}

pub fn import(root: &Path, input: &Path) -> io::Result<(String, PathBuf)> {
    let name = input
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("The ZIP needs a valid filename."))?;
    let name = name.trim();
    if name.is_empty()
        || name.len() > 60
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b" _-".contains(&byte))
    {
        return Err(invalid(
            "Rename the ZIP using up to 60 letters, numbers, spaces, hyphens, or underscores.",
        ));
    }
    let file = File::open(input)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err(invalid("This save ZIP is too large."));
    }
    let mut archive = ZipArchive::new(file)?;
    if archive.len() > MAX_FILES {
        return Err(invalid("This save ZIP contains too many entries."));
    }
    fs::create_dir_all(root)?;
    let destination = root.join(format!("Checkpoint - {name}"));
    if destination.exists() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            "A checkpoint with this name already exists. Rename the ZIP to import a separate copy.",
        ));
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let stage = root.join(format!(".import-{}-{id}", std::process::id()));
    fs::create_dir(&stage)?;
    let result = (|| {
        let mut total = 0;
        let mut manifests = 0;
        let mut save_root = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            if entry.is_dir() {
                continue;
            }
            let full = entry.name()?;
            let parts: Vec<_> = full.split('/').collect();
            let Some(start) = parts
                .windows(2)
                .position(|pair| pair[0].eq_ignore_ascii_case("7952987A") && pair[1] == "saves")
                .map(|i| i + 2)
                .or_else(|| (parts.first() == Some(&"saves")).then_some(1))
            else {
                continue;
            };
            let relative = &parts[start..];
            if entry.enclosed_name().is_none()
                || entry.is_symlink()
                || relative.is_empty()
                || relative.len() > 12
                || !relative.iter().all(|part| safe_component(part))
            {
                return Err(invalid("The ZIP contains an unsafe save entry."));
            }
            let prefix = parts[..start].join("/");
            if save_root
                .as_ref()
                .is_some_and(|previous| previous != &prefix)
            {
                return Err(invalid("The ZIP contains more than one save folder."));
            }
            save_root = Some(prefix);
            let size = entry.size();
            if size > MAX_BYTES - total {
                return Err(invalid("The extracted saves exceed the size limit."));
            }
            let path = relative
                .iter()
                .fold(stage.clone(), |path, part| path.join(part));
            if relative.last() == Some(&"manifest") {
                manifests += 1;
            }
            fs::create_dir_all(path.parent().unwrap())?;
            let mut output = OpenOptions::new().write(true).create_new(true).open(path)?;
            if io::copy(&mut (&mut entry).take(size + 1), &mut output)? != size {
                return Err(invalid("A ZIP entry did not match its declared size."));
            }
            total += size;
        }
        if manifests == 0 {
            return Err(invalid("No VIctory save manifests were found in the ZIP."));
        }
        fs::rename(&stage, &destination)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result?;
    Ok((name.into(), destination))
}

pub fn export(source: &Path, destination: &Path) -> io::Result<()> {
    let mut archive = ZipWriter::new(io::Cursor::new(Vec::new()));
    let mut limits = (0, 0);
    write_tree(&mut archive, source, "7952987A/saves", &mut limits)?;
    if limits.0 == 0 {
        return Err(invalid("This save has no files to export."));
    }
    let bytes = archive.finish()?.into_inner();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if let Err(error) = file.write_all(&bytes) {
        drop(file);
        let _ = fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

fn write_tree(
    archive: &mut ZipWriter<io::Cursor<Vec<u8>>>,
    source: &Path,
    prefix: &str,
    limits: &mut (usize, u64),
) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .filter(|name| safe_component(name))
            .ok_or_else(|| invalid("A save entry has an unsupported filename."))?;
        let path = format!("{prefix}/{name}");
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if prefix.matches('/').count() >= 12 {
                return Err(invalid("The save folder is nested too deeply."));
            }
            write_tree(archive, &entry.path(), &path, limits)?;
        } else if kind.is_file() {
            if limits.0 >= MAX_FILES || limits.1 >= MAX_BYTES {
                return Err(invalid("This save exceeds the export limits."));
            }
            archive.start_file(path, SimpleFileOptions::default())?;
            let max = MAX_BYTES - limits.1;
            let written = io::copy(&mut File::open(entry.path())?.take(max + 1), archive)?;
            if written > max {
                return Err(invalid("This save exceeds the export size limit."));
            }
            limits.0 += 1;
            limits.1 += written;
        } else {
            return Err(invalid("The save folder contains an unsupported entry."));
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn exports_and_imports_a_save_without_overwriting_checkpoints() -> io::Result<()> {
    let root = std::env::temp_dir().join(format!("vilauncher-zip-{}", std::process::id()));
    fs::create_dir(&root)?;
    let result = (|| {
        let source = root.join("source");
        let slot = "0009000000000001/00000000-0000-0000-0000-00007952987a/Player~example/v1";
        fs::create_dir_all(source.join(slot))?;
        fs::write(source.join(slot).join("manifest"), b"example manifest")?;
        fs::write(source.join(slot).join("0"), b"example save")?;
        let archive = root.join("Example Save.zip");
        export(&source, &archive)?;
        assert!(export(&source, &archive).is_err());
        let checkpoints = root.join("checkpoints");
        let (name, imported) = import(&checkpoints, &archive)?;
        assert_eq!(name, "Example Save");
        assert_eq!(fs::read(imported.join(slot).join("0"))?, b"example save");
        assert!(import(&checkpoints, &archive).is_err());
        Ok(())
    })();
    fs::remove_dir_all(root)?;
    result
}
