use std::{cell::RefCell, io, path::PathBuf};

use crate::env_playground::Environment;

thread_local! {
    static CURRENT_WORKSPACE: RefCell<PathBuf> = RefCell::new(PathBuf::new());
}

pub struct EnvFileSystem {}

impl EnvFileSystem {
    pub fn set_current_workspace(path: &str) {
        CURRENT_WORKSPACE.with(|c| *c.borrow_mut() = PathBuf::from(path));
    }

    fn workspace_env_path() -> Option<PathBuf> {
        CURRENT_WORKSPACE.with(|c| {
            let ws = c.borrow();
            if ws.as_os_str().is_empty() {
                None
            } else {
                Some(ws.join(".arc").join("environments.json"))
            }
        })
    }

    fn read_raw() -> String {
        let Some(path) = Self::workspace_env_path() else {
            return Self::default_json();
        };
        std::fs::read_to_string(path).unwrap_or_default()
    }

    fn write_raw(content: &str) -> io::Result<()> {
        let Some(path) = Self::workspace_env_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, content)
    }

    fn ensure_file() {
        let Some(path) = Self::workspace_env_path() else {
            return;
        };
        if path.exists() {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, Self::default_json());
    }

    fn default_json() -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "active_environment": "local",
            "environments": [
                {"name": "local", "variables": []},
                {"name": "production", "variables": []}
            ]
        }))
        .unwrap_or_default()
    }

    fn parse() -> serde_json::Value {
        Self::ensure_file();
        let raw = Self::read_raw();
        let data: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
        if data.is_object() {
            data
        } else {
            let _ = Self::write_raw(&Self::default_json());
            serde_json::from_str(&Self::default_json()).unwrap_or_default()
        }
    }

    pub fn read_environment_variables() -> String {
        let data = Self::parse();
        data.get("environments")
            .and_then(|e| serde_json::to_string_pretty(e).ok())
            .unwrap_or_else(|| "[]".into())
    }

    pub fn rename_environment(old_name: &str, new_name: &str) {
        let mut envs: Vec<Environment> =
            serde_json::from_str(&Self::read_environment_variables()).unwrap_or_default();
        if let Some(env) = envs.iter_mut().find(|e| e.name == old_name) {
            env.name = new_name.to_string();
        }
        let _ = Self::save_environment_variables(
            &serde_json::to_string_pretty(&envs).unwrap_or_default(),
        );
        if Self::read_active_environment() == old_name {
            Self::save_active_environment(new_name);
        }
    }

    pub fn save_environment_variables(content: &str) -> io::Result<()> {
        let mut data = Self::parse();
        if let Ok(envs) = serde_json::from_str::<serde_json::Value>(content) {
            data["environments"] = envs;
        }
        Self::write_raw(&serde_json::to_string_pretty(&data).unwrap_or_default())
    }

    pub fn read_active_environment() -> String {
        let data = Self::parse();
        data.get("active_environment")
            .and_then(|v| v.as_str())
            .unwrap_or("Local")
            .into()
    }

    pub fn save_active_environment(name: &str) {
        let mut data = Self::parse();
        data["active_environment"] = serde_json::json!(name);
        let _ = Self::write_raw(&serde_json::to_string_pretty(&data).unwrap_or_default());
    }

    pub fn validate_active_environment() -> String {
        let active = Self::read_active_environment();
        let envs: Vec<Environment> =
            serde_json::from_str(&Self::read_environment_variables()).unwrap_or_default();
        if envs.iter().any(|e| e.name == active) {
            active
        } else {
            let fallback = envs
                .first()
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Local".into());
            Self::save_active_environment(&fallback);
            fallback
        }
    }

    pub fn delete_environment(name: &str) {
        let mut envs: Vec<Environment> =
            serde_json::from_str(&Self::read_environment_variables()).unwrap_or_default();
        envs.retain(|e| e.name != name);
        let _ = Self::save_environment_variables(
            &serde_json::to_string_pretty(&envs).unwrap_or_default(),
        );
        if Self::read_active_environment() == name {
            let new_active = envs
                .first()
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "Local".into());
            Self::save_active_environment(&new_active);
        }
    }

    pub fn init_workspace(path: &str) {
        if path.is_empty() {
            return;
        }
        Self::set_current_workspace(path);
        let Some(env_path) = Self::workspace_env_path() else {
            return;
        };
        if !env_path.exists() {
            if let Some(parent) = env_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(env_path, Self::default_json());
        }
    }
}
