//! How much disk the launcher's projects use, read from the workspace folders.
//! Only reads here; clearing is done by the launcher, inside the workspace.

use serde::Serialize;
use std::path::Path;

/// The bytes one project uses in each of its folders under the workspace.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct ProjectUse {
    pub(crate) slug: String,
    /// The repo the project was added from; the slug until the launcher fills it in.
    pub(crate) repo: String,
    /// The name shown to the player; the slug until the launcher fills it in.
    pub(crate) label: String,
    /// `repos\<slug>`: the downloaded sources.
    pub(crate) sources: u64,
    /// `target\<slug>`: the build output.
    pub(crate) builds: u64,
    /// `previous\<slug>`: the build kept for rollback.
    pub(crate) previous: u64,
    /// `cache\<slug>`: the shader cache.
    pub(crate) cache: u64,
    /// `data\<slug>`: saves and profiles. Never cleared.
    pub(crate) saves: u64,
}

/// What the Storage section shows.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct StorageView {
    /// Everything under the workspace.
    pub(crate) total_bytes: u64,
    /// What is in the workspace but not in any project's folders (checkpoints, logs, ...).
    pub(crate) other_bytes: u64,
    /// Free space on the drive, when it could be read.
    pub(crate) free_bytes: Option<u64>,
    pub(crate) projects: Vec<ProjectUse>,
}

/// Total size of the files under `path`, recursively. Symlinks are skipped and
/// anything unreadable counts as zero, so a folder in use never stops the scan.
pub(crate) fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Sizes every project in `slugs` and the workspace as a whole.
/// `free_bytes` is left empty; the launcher fills it from the drive.
pub(crate) fn scan(workspace: &Path, slugs: &[String]) -> StorageView {
    let total_bytes = dir_size(workspace);
    let mut projects_total = 0u64;
    let projects = slugs
        .iter()
        .map(|slug| {
            let folder = |name: &str| dir_size(&workspace.join(name).join(slug));
            let project = ProjectUse {
                slug: slug.clone(),
                repo: slug.clone(),
                label: slug.clone(),
                sources: folder("repos"),
                builds: folder("target"),
                previous: folder("previous"),
                cache: folder("cache"),
                saves: folder("data"),
            };
            projects_total +=
                project.sources + project.builds + project.previous + project.cache + project.saves;
            project
        })
        .collect();
    StorageView {
        total_bytes,
        other_bytes: total_bytes.saturating_sub(projects_total),
        free_bytes: None,
        projects,
    }
}

/// Whether `dir` may be deleted: it must be strictly inside `workspace` and not in `data`.
pub(crate) fn may_clear(workspace: &Path, dir: &Path) -> bool {
    dir.starts_with(workspace) && dir != workspace && !dir.starts_with(workspace.join("data"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir; never the real workspace.
    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vilauncher-storage-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![7u8; bytes]).unwrap();
    }

    #[test]
    fn dir_size_adds_up_nested_files() {
        let root = temp_root("size");
        write(&root.join("a.bin"), 10);
        write(&root.join("sub").join("b.bin"), 25);
        write(&root.join("sub").join("deeper").join("c.bin"), 7);
        assert_eq!(dir_size(&root), 42);
        assert_eq!(dir_size(&root.join("missing")), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn dir_size_skips_symlinks() {
        let root = temp_root("symlink");
        let outside = temp_root("symlink-target");
        write(&outside.join("big.bin"), 1000);
        write(&root.join("small.bin"), 5);
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        assert_eq!(dir_size(&root), 5);
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn scan_splits_projects_and_other() {
        let ws = temp_root("scan");
        write(&ws.join("repos").join("a_b").join("src.rs"), 100);
        write(&ws.join("target").join("a_b").join("victory.exe"), 200);
        write(&ws.join("cache").join("a_b").join("shaders.bin"), 30);
        write(&ws.join("data").join("a_b").join("save.json"), 4);
        write(&ws.join("previous").join("a_b").join("victory.exe"), 50);
        write(&ws.join("repos").join("other_c").join("x.rs"), 11);
        write(&ws.join("checkpoints").join("note.txt"), 9);

        let slugs = vec!["a_b".to_string(), "missing_proj".to_string()];
        let view = scan(&ws, &slugs);

        assert_eq!(view.total_bytes, 100 + 200 + 30 + 4 + 50 + 11 + 9);
        assert_eq!(view.other_bytes, 11 + 9);
        assert_eq!(view.free_bytes, None);
        assert_eq!(view.projects.len(), 2);
        let a = &view.projects[0];
        assert_eq!(a.slug, "a_b");
        assert_eq!(a.label, "a_b");
        assert_eq!(a.sources, 100);
        assert_eq!(a.builds, 200);
        assert_eq!(a.previous, 50);
        assert_eq!(a.cache, 30);
        assert_eq!(a.saves, 4);
        let missing = &view.projects[1];
        assert_eq!(
            missing.sources + missing.builds + missing.cache + missing.saves,
            0
        );
        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn may_clear_only_inside_workspace_and_never_data() {
        let ws = Path::new(if cfg!(windows) { "C:\\ws" } else { "/ws" });
        assert!(may_clear(ws, &ws.join("cache").join("a_b")));
        assert!(may_clear(ws, &ws.join("previous").join("a_b")));
        assert!(!may_clear(ws, ws));
        assert!(!may_clear(ws, &ws.join("data").join("a_b")));
        assert!(!may_clear(ws, &ws.join("data")));
        assert!(!may_clear(
            ws,
            &ws.parent().unwrap().join("ws2").join("cache")
        ));
    }
}
