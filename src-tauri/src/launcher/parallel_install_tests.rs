use super::*;
use std::{
    fs, thread,
    time::{SystemTime, UNIX_EPOCH},
};

const FIXTURE_ROOT: &str = "VILAUNCHER_TEST_PARALLEL_INSTALL_ROOT";
const TEST_NAME: &str = "launcher::parallel_install_tests::two_installs_and_an_update_overlap";

#[test]
fn two_installs_and_an_update_overlap() {
    // The launcher stores its install directory process-wide. Give this fixture
    // its own process and workspace so the rest of the suite stays isolated.
    let Some(root) = std::env::var_os(FIXTURE_ROOT).map(PathBuf::from) else {
        let temp = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = temp.join(format!(
            "vilauncher-parallel-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let result = Command::new(std::env::current_exe().unwrap())
            .env(FIXTURE_ROOT, &root)
            .env("LOCALAPPDATA", &root)
            .env("CARGO_NET_OFFLINE", "true")
            .env("GIT_ALLOW_PROTOCOL", "file")
            .args(["--exact", TEST_NAME, "--nocapture"])
            .output();
        assert_eq!(
            root.canonicalize().unwrap().parent(),
            Some(temp.canonicalize().unwrap().as_path())
        );
        fs::remove_dir_all(&root).unwrap();
        let output = result.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    };

    set_install_dir(root.join("workspace").to_str().unwrap());
    let ctx = Ctx::new(|| {});
    let mut builds = ProjectBuilds::default();
    let names = ["fixture_update", "fixture_install_a", "fixture_install_b"];
    let ready = root.join("ready");
    fs::create_dir(&ready).unwrap();
    let projects = names.map(|name| make_project(&root, name));

    let update = &projects[0];
    let first = builds.start(&update.repo, 1).unwrap();
    install(
        &first,
        &ctx,
        update,
        false,
        &mut test_launch(root.join("no-game")),
    )
    .unwrap_or_else(|error| panic!("{error}\n{}", first.lock().unwrap().log.join("\n")));
    first.lock().unwrap().running = false;
    let prior = repo_state(update).built.unwrap();
    assert!(installed_exe(&update.repo).is_file());

    for name in names {
        let source = root.join(name);
        fs::write(source.join("build.rs"), rendezvous(&ready, name, &names)).unwrap();
        git(&source, &["add", "build.rs"]);
        git(&source, &["commit", "-qm", "Require all concurrent builds"]);
    }

    thread::scope(|scope| {
        let mut workers = Vec::new();
        for (index, project) in projects.iter().enumerate() {
            let job = builds.start(&project.repo, index as u64 + 2).unwrap();
            let ctx = ctx.clone();
            let game_dir = root.join("no-game");
            workers.push(scope.spawn(move || {
                install(&job, &ctx, project, index == 0, &mut test_launch(game_dir))
                    .unwrap_or_else(|error| {
                        panic!("{error}\n{}", job.lock().unwrap().log.join("\n"))
                    });
                job.lock().unwrap().running = false;
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });

    for project in &projects {
        let state = repo_state(project);
        assert!(state.installed);
        assert_eq!(state.built, state.remote);
        assert!(installed_exe(&project.repo).is_file());
        let log = builds
            .get(&project.repo)
            .unwrap()
            .lock()
            .unwrap()
            .log
            .join("\n");
        assert!(
            log.contains(&format!("overlap verified: {}", project.name)),
            "{log}"
        );
        for other in projects.iter().filter(|other| other.repo != project.repo) {
            assert!(
                !log.contains(&format!("overlap verified: {}", other.name)),
                "{log}"
            );
        }
    }
    assert_eq!(
        previous::previous_commit(&previous_dir(&update.repo)).as_deref(),
        Some(prior.as_str())
    );
}

fn make_project(root: &Path, name: &str) -> Project {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = '{name}'\nversion = '0.1.0'\nedition = '2021'\n"),
    )
    .unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "Initial fixture"]);
    Project {
        name: name.into(),
        repo: url::Url::from_directory_path(dir).unwrap().to_string(),
        ..test_project()
    }
}

fn git(dir: &Path, args: &[&str]) {
    let output = tools::git()
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=.git/no-hooks",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rendezvous(ready: &Path, name: &str, names: &[&str]) -> String {
    // A build cannot finish until the other two Cargo builds are inside their
    // own build scripts. Sequential execution fails at the bounded timeout.
    format!(
        r#"
fn main() {{
    let ready = std::path::Path::new({path:?});
    let name = {name:?};
    std::fs::write(ready.join(name), []).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    while !{names:?}.iter().all(|peer| ready.join(peer).is_file()) {{
        assert!(std::time::Instant::now() < deadline, "other builds did not start");
        std::thread::sleep(std::time::Duration::from_millis(25));
    }}
    println!("cargo:warning=overlap verified: {{name}}");
}}
"#,
        path = ready.to_str().unwrap(),
    )
}
