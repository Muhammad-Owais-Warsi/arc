use std::{
    io,
    path::{Path, PathBuf},
};

use crate::env_fs::EnvFileSystem;

pub struct ConfigFileSystem {}

impl ConfigFileSystem {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".arc")
    }

    fn settings_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }

    fn workspace_path() -> PathBuf {
        Self::config_dir().join("workspace.json")
    }

    pub fn init_setup() -> io::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;

        fn write_if_missing(path: &Path, content: &str) -> io::Result<()> {
            if path.exists() {
                return Ok(());
            }
            std::fs::write(path, content)
        }

        write_if_missing(
            &dir.join("settings.json"),
            &serde_json::to_string_pretty(&serde_json::json!({})).unwrap_or_else(|_| "{}".into()),
        )?;

        write_if_missing(
            &Self::workspace_path(),
            &serde_json::to_string_pretty(&serde_json::json!({
                "active_workspace": { "name": "", "path": "" }
            }))
            .unwrap_or_else(|_| "{}".into()),
        )?;

        Ok(())
    }

    pub fn create_workspace(name: &str) -> io::Result<String> {
        let path = Self::config_dir().join(name);
        std::fs::create_dir(&path)?;
        EnvFileSystem::init_workspace(&path.to_string_lossy());
        Ok(path.to_string_lossy().to_string())
    }

    pub fn read_workspace_config() -> Option<(String, String)> {
        let data: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(Self::workspace_path()).unwrap_or_default(),
        )
        .ok()?;
        let active = data.get("active_workspace")?;
        let name = active.get("name")?.as_str()?.to_string();
        let path = active.get("path")?.as_str()?.to_string();
        if name.is_empty() || path.is_empty() {
            None
        } else {
            Some((name, path))
        }
    }

    pub fn save_workspace_config(name: &str, path: &str) {
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "active_workspace": { "name": name, "path": path }
        }))
        .unwrap_or_default();
        let _ = std::fs::write(Self::workspace_path(), &json);
        EnvFileSystem::set_current_workspace(path);
    }

    pub fn read_settings() -> String {
        std::fs::read_to_string(Self::settings_path()).unwrap_or_default()
    }

    pub fn save_settings(content: &str) -> io::Result<()> {
        std::fs::write(Self::settings_path(), content)
    }
}
