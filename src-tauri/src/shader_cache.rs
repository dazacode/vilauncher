//! The shader cache a project keeps (`VICTORY_CACHE_DIR`, `<workspace>\cache\<slug>`):
//! what the player's "Clear shader cache" deletes, and how it checks the folder is
//! empty afterwards. Nothing outside the folder is touched, and no link is followed.

use std::fs;
use std::io;
use std::path::{Component, Path};

/// Deletes what is inside `dir` and keeps `dir` itself, then lists it again and fails
/// unless it is empty. `dir` must be a folder inside `root` (not `root`, not through a
/// `..`, not a link). A `dir` that is gone already clears nothing. Returns the bytes of
/// files removed.
pub(crate) fn clear(root: &Path, dir: &Path) -> Result<u64, String> {
    let inside = dir.starts_with(root)
        && dir != root
        && !dir.components().any(|c| matches!(c, Component::ParentDir));
    if !inside {
        return Err(format!(
            "{} is not inside the cache folder {}; nothing was deleted.",
            dir.display(),
            root.display()
        ));
    }
    let meta = match fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("Could not read {}: {error}", dir.display())),
    };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!(
            "{} is not a folder; nothing was deleted.",
            dir.display()
        ));
    }
    let mut freed = 0;
    for entry in read(dir)? {
        freed += remove_entry(&entry.map_err(|e| e.to_string())?.path())?;
    }
    let left = read(dir)?.count();
    if left > 0 {
        return Err(format!(
            "{left} item(s) are still in {} after clearing it.",
            dir.display()
        ));
    }
    Ok(freed)
}

fn read(dir: &Path) -> Result<fs::ReadDir, String> {
    fs::read_dir(dir).map_err(|error| format!("Could not list {}: {error}", dir.display()))
}

/// Deletes one entry of the cache and returns the bytes of the files in it. A link is
/// removed as a link, never followed.
fn remove_entry(path: &Path) -> Result<u64, String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let (bytes, result) = if meta.file_type().is_symlink() {
        (0, fs::remove_file(path).or_else(|_| fs::remove_dir(path)))
    } else if meta.is_dir() {
        (tree_bytes(path), fs::remove_dir_all(path))
    } else {
        (meta.len(), fs::remove_file(path))
    };
    result
        .map(|()| bytes)
        .map_err(|error| format!("Could not delete {}: {error}", path.display()))
}

/// The bytes of the files under `path`, not following links. Unreadable parts count as 0.
fn tree_bytes(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.file_type().is_symlink() {
        0
    } else if meta.is_dir() {
        fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| tree_bytes(&entry.path()))
            .sum()
    } else {
        meta.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("vilauncher-shader-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn clearing_deletes_what_is_inside_the_project_folder_and_reports_the_bytes() {
        let base = temp("clear");
        let cache = base.join("cache");
        let project = cache.join("owner_victory");
        fs::create_dir_all(project.join("pipelines/nested")).unwrap();
        fs::write(project.join("a.bin"), [0u8; 100]).unwrap();
        fs::write(project.join("pipelines/nested/b.bin"), [0u8; 50]).unwrap();
        // A neighbour project's cache and a file outside the cache must survive.
        let other = cache.join("owner_other");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("keep.bin"), [1u8; 10]).unwrap();
        let outside = base.join("data.txt");
        fs::write(&outside, b"saves").unwrap();

        assert_eq!(clear(&cache, &project), Ok(150));
        assert!(project.is_dir(), "the folder itself is kept, now empty");
        assert_eq!(fs::read_dir(&project).unwrap().count(), 0);
        assert!(
            other.join("keep.bin").is_file(),
            "another project's cache is untouched"
        );
        assert_eq!(
            fs::read(&outside).unwrap(),
            b"saves",
            "nothing outside the cache is touched"
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn a_folder_that_is_gone_clears_nothing() {
        let base = temp("gone");
        let cache = base.join("cache");
        fs::create_dir_all(&cache).unwrap();
        assert_eq!(clear(&cache, &cache.join("owner_victory")), Ok(0));
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn only_a_folder_inside_the_cache_is_ever_cleared() {
        let base = temp("refuse");
        let cache = base.join("cache");
        fs::create_dir_all(&cache).unwrap();
        let data = base.join("data").join("owner_victory");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("save.dat"), b"keep").unwrap();

        assert!(
            clear(&cache, &data).is_err(),
            "a data folder beside the cache is refused"
        );
        assert!(
            clear(&cache, &cache).is_err(),
            "the whole cache folder is refused"
        );
        assert!(clear(&cache, &cache.join("..").join("data").join("owner_victory")).is_err());
        assert_eq!(fs::read(data.join("save.dat")).unwrap(), b"keep");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn a_plain_file_in_place_of_the_folder_is_refused() {
        let base = temp("file");
        let cache = base.join("cache");
        fs::create_dir_all(&cache).unwrap();
        let file = cache.join("owner_victory");
        fs::write(&file, b"not a folder").unwrap();
        assert!(clear(&cache, &file).is_err());
        assert!(file.is_file(), "nothing was deleted");
        fs::remove_dir_all(base).unwrap();
    }
}
