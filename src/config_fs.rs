use std::{
    io,
    path::{Path, PathBuf},
};

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

    pub fn environments_path() -> PathBuf {
        Self::config_dir().join("environments.json")
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

    pub fn create_workspace(name: &str) -> io::Result<String> {
        let projects_dir = Self::config_dir();
        let path = projects_dir.join(name);

        std::fs::create_dir(&path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn read_workspace_config() -> String {
        std::fs::read_to_string(Self::workspace_path()).unwrap_or_default()
    }

    pub fn save_workspace_config(content: &str) -> io::Result<()> {
        std::fs::write(Self::workspace_path(), content)
    }

    pub fn read_last_workspace() -> Option<(String, String)> {
        let value: serde_json::Value = serde_json::from_str(&Self::read_workspace_config()).ok()?;
        let active = value.as_array()?.first()?.get("active_workspace")?;
        let name = active.get("name")?.as_str().unwrap_or("").to_string();
        let path = active.get("path")?.as_str().unwrap_or("").to_string();
        if name.is_empty() || path.is_empty() {
            None
        } else {
            Some((name, path))
        }
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
            let _ = Self::save_workspace_config(&json);
        }
    }

    pub fn read_settings() -> String {
        std::fs::read_to_string(Self::settings_path()).unwrap_or_default()
    }

    pub fn save_settings(content: &str) -> io::Result<()> {
        std::fs::write(Self::settings_path(), content)
    }

    pub fn read_environment_variables() -> String {
        std::fs::read_to_string(Self::environments_path()).unwrap_or_default()
    }

    pub fn save_environment_variables(content: &str) -> io::Result<()> {
        std::fs::write(Self::environments_path(), content)
    }

    // pub fn save_environment_variables() -> io::Result<()> {}
}
