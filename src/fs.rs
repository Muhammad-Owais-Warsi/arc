use crate::auth::AuthType;
use serde::{Deserialize, Serialize};
use std::{
    fs::{OpenOptions, read_to_string, remove_dir_all, remove_file},
    io,
    path::{Path, PathBuf},
};
use trash;
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct Auth {
    pub auth_type: AuthType,
    pub username: String,
    pub password: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct Body {
    pub body_type: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct FileContent {
    pub name: String,
    pub url: String,
    pub method: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub auth: Auth,
    pub body: Body,
}

pub fn create_folder(name: &str, parent_dir: &str) -> io::Result<String> {
    let path = format!("{parent_dir}/{name}");

    std::fs::create_dir(&path)?;
    Ok(path)
}

pub fn create_workspace(name: &str) -> io::Result<String> {
    let projects_dir = config_dir();
    let path = projects_dir.join(name);

    std::fs::create_dir(&path)?;
    Ok(path.to_string_lossy().to_string())
}

pub fn create_file(name: &str, parent_dir: &str) -> io::Result<String> {
    let path = format!("{parent_dir}/{name}.json");

    let file = OpenOptions::new()
        .write(true)
        .create_new(true) // Fails if the file already exists
        .open(&path)?;

    let content = FileContent {
        name: name.to_string(),
        url: String::new(),
        method: "GET".to_string(),
        params: vec![],
        headers: vec![],
        auth: Auth {
            auth_type: AuthType::None,
            username: String::new(),
            password: String::new(),
            token: String::new(),
        },
        body: Body {
            body_type: "JSON".to_string(),
            body: String::new(),
        },
    };

    serde_json::to_writer_pretty(file, &content).map_err(io::Error::other)?;

    Ok(path)
}

pub fn delete_file_or_folder(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        remove_dir_all(path)?;
    } else {
        remove_file(path)?;
    }
    Ok(())
}

pub fn trash_file_or_folder(path: &Path) -> io::Result<()> {
    trash::delete(path).map_err(io::Error::other)
}

pub fn write_request_file(path: &Path, content: &FileContent) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    serde_json::to_writer_pretty(file, content).map_err(io::Error::other)?;
    Ok(())
}

pub fn read_request_file(path: &Path) -> serde_json::Value {
    let Ok(content) = read_to_string(path) else {
        return serde_json::json!({});
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return serde_json::json!({});
    };

    value
}

pub fn read_request_method(path: &Path) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return String::new();
    };
    value
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_uppercase()
}

pub fn rename_item(old_path: &str, new_path: &str) -> io::Result<String> {
    std::fs::rename(old_path, new_path)?;

    Ok(new_path.to_string())
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arc")
}

pub fn settings_file_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn environments_path() -> PathBuf {
    config_dir().join("environments.json")
}

pub fn workspace_path() -> PathBuf {
    config_dir().join("workspace.json")
}

/// Create the app config directory (`%APPDATA%\.arc`) and seed it with default
/// `settings.json` and `environments.json` files when they don't exist yet.
/// This is called once on startup so a fresh install always has both files.
pub fn ensure_config_dir() -> io::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;

    fn write_if_missing(path: &Path, content: &str) -> io::Result<()> {
        if path.exists() {
            return Ok(());
        }
        std::fs::write(path, content)
    }

    write_if_missing(
        &dir.join("settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({}))
            .unwrap_or_else(|_| "{}".to_string())
            .as_str(),
    )?;

    let default_envs = serde_json::json!([
        {
            "name": "Local",
            "variables": []
        },
        {
            "name": "Production",
            "variables": []
        }
    ]);
    write_if_missing(
        &dir.join("environments.json"),
        serde_json::to_string_pretty(&default_envs)
            .unwrap_or_else(|_| "[]".to_string())
            .as_str(),
    )?;

    let default_workspace = serde_json::json!([
        {
            "active_workspace": {
                "name": "",
                "path": ""
            }
        }
    ]);
    write_if_missing(
        &dir.join("workspace.json"),
        serde_json::to_string_pretty(&default_workspace)
            .unwrap_or_else(|_| "[]".to_string())
            .as_str(),
    )?;

    Ok(())
}

pub fn get_workspace_config() -> String {
    std::fs::read_to_string(workspace_path()).unwrap_or_default()
}

pub fn save_workspace_config(content: &str) -> io::Result<()> {
    std::fs::write(workspace_path(), content)
}

pub fn save_last_workspace(name: &str, path: &str) {
    let content = serde_json::json!([
        {
            "active_workspace": {
                "name": name,
                "path": path
            }
        }
    ]);
    if let Ok(json) = serde_json::to_string_pretty(&content) {
        let _ = save_workspace_config(&json);
    }
}

pub fn load_last_workspace() -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(&get_workspace_config()).ok()?;
    let active = value.as_array()?.first()?.get("active_workspace")?;
    let name = active.get("name")?.as_str().unwrap_or("").to_string();
    let path = active.get("path")?.as_str().unwrap_or("").to_string();
    if name.is_empty() || path.is_empty() {
        None
    } else {
        Some((name, path))
    }
}

pub fn get_settings() -> String {
    std::fs::read_to_string(settings_file_path()).unwrap_or_default()
}

pub fn save_settings(content: &str) -> io::Result<()> {
    std::fs::write(settings_file_path(), content)
}

pub fn get_environment_variables() -> String {
    std::fs::read_to_string(environments_path()).unwrap_or_default()
}
