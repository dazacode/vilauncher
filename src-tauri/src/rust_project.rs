use crate::{hide_window, tools};
use serde_json::Value;
use std::path::Path;

pub fn metadata(workspace: &Path) -> Result<Value, String> {
    let mut command = tools::cargo();
    command
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace);
    hide_window(&mut command);
    let output = command
        .output()
        .map_err(|error| format!("Could not run Cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Could not read Cargo metadata: {error}"))
}

fn binaries(package: &Value) -> impl Iterator<Item = &str> {
    package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|target| {
            target["kind"]
                .as_array()?
                .iter()
                .any(|kind| kind == "bin")
                .then(|| target["name"].as_str())
                .flatten()
        })
}

pub fn is_victory(workspace: &Path, metadata: &Value) -> bool {
    let Some(packages) = metadata["packages"].as_array() else {
        return false;
    };
    let has_victory_binary = packages.iter().any(|package| {
        package["name"] == "victory" && binaries(package).any(|name| name == "victory")
    });
    has_victory_binary
        && (["apps/victory", "crates/victory", "victory"]
            .iter()
            .any(|path| workspace.join(path).join("Cargo.toml").is_file())
            || packages.iter().any(|package| {
                package["name"] == "victory-hle" || package["name"] == "victory-loader"
            }))
}

pub fn select_executable<'a>(metadata: &Value, artifacts: &'a [Value]) -> Option<&'a str> {
    let packages = metadata["packages"].as_array()?;
    let defaults = metadata["workspace_default_members"].as_array()?;
    artifacts
        .iter()
        .filter_map(|artifact| {
            let id = &artifact["package_id"];
            let package = packages.iter().find(|package| package["id"] == *id)?;
            let name = artifact["target"]["name"].as_str()?;
            if !binaries(package).any(|bin| bin == name) {
                return None;
            }
            let executable = artifact["executable"].as_str()?;
            Some((
                !defaults.contains(id),
                package["default_run"] != name,
                package["name"] != name,
                name,
                executable,
            ))
        })
        .min()
        .map(|(_, _, _, _, executable)| executable)
}

#[cfg(test)]
#[test]
fn chooses_default_run_from_workspace_artifacts() {
    let metadata = serde_json::json!({
        "workspace_default_members": ["app"],
        "packages": [
            {"id": "app", "name": "app", "default_run": "runner", "targets": [
                {"name": "app", "kind": ["bin"]}, {"name": "runner", "kind": ["bin"]}
            ]},
            {"id": "helper", "name": "helper", "targets": [{"name": "helper", "kind": ["bin"]}]}
        ]
    });
    let artifacts = [
        serde_json::json!({"package_id": "helper", "target": {"name": "helper"}, "executable": "helper.exe"}),
        serde_json::json!({"package_id": "app", "target": {"name": "app"}, "executable": "app.exe"}),
        serde_json::json!({"package_id": "app", "target": {"name": "runner"}, "executable": "runner.exe"}),
    ];
    assert_eq!(select_executable(&metadata, &artifacts), Some("runner.exe"));
    assert_eq!(select_executable(&metadata, &[]), None);
}
