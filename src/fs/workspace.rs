use std::fs;
use std::io;
use std::path::PathBuf;

use crate::fs::env;
use crate::fs::settings;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arc")
}

fn workspace_path() -> PathBuf {
    config_dir().join("workspace.json")
}

pub fn init() -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;

    fn write_if_missing(path: &std::path::Path, content: &str) -> io::Result<()> {
        if path.exists() {
            return Ok(());
        }
        fs::write(path, content)
    }

    write_if_missing(
        &dir.join("settings.json"),
        &serde_json::to_string_pretty(&serde_json::json!({})).unwrap_or_else(|_| "{}".into()),
    )?;

    write_if_missing(
        &workspace_path(),
        &serde_json::to_string_pretty(&serde_json::json!({
            "active_workspace": { "name": "", "path": "" }
        }))
        .unwrap_or_else(|_| "{}".into()),
    )?;

    Ok(())
}

pub fn read() -> Option<(String, String)> {
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(workspace_path()).unwrap_or_default()).ok()?;
    let active = data.get("active_workspace")?;
    let name = active.get("name")?.as_str()?.to_string();
    let path = active.get("path")?.as_str()?.to_string();
    if name.is_empty() || path.is_empty() {
        None
    } else {
        Some((name, path))
    }
}

pub fn save(name: &str, path: &str) {
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "active_workspace": { "name": name, "path": path }
    }))
    .unwrap_or_default();
    let _ = fs::write(workspace_path(), &json);
}

pub fn create(name: &str) -> io::Result<String> {
    let path = config_dir().join(name);
    fs::create_dir(&path)?;
    env::init_workspace(&path.to_string_lossy());
    Ok(path.to_string_lossy().to_string())
}
