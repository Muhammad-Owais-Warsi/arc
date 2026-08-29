use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::env_playground::Environment;

fn workspace_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arc")
        .join("workspace.json")
}

fn env_file_path() -> Option<PathBuf> {
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(workspace_path()).ok()?).ok()?;
    let path = data
        .get("active_workspace")?
        .get("path")?
        .as_str()?;
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path).join(".arc").join("environments.json"))
}

fn default_data() -> serde_json::Value {
    serde_json::json!({
        "active_environment": "local",
        "environments": [
            {"name": "local", "variables": []},
            {"name": "production", "variables": []}
        ]
    })
}

fn read_data() -> serde_json::Value {
    env_file_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(default_data)
}

fn write_data(data: &serde_json::Value) {
    let Some(path) = env_file_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, content);
    }
}

fn environments_from(data: &serde_json::Value) -> Vec<Environment> {
    data.get("environments")
        .and_then(|e| serde_json::from_value(e.clone()).ok())
        .unwrap_or_default()
}

pub fn read_environments() -> String {
    serde_json::to_string_pretty(&environments_from(&read_data())).unwrap_or_else(|_| "[]".into())
}

pub fn write_environments(json_str: &str) {
    let Ok(envs) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return;
    };
    let mut data = read_data();
    data["environments"] = envs;
    write_data(&data);
}

pub fn read_active() -> String {
    read_data()
        .get("active_environment")
        .and_then(|v| v.as_str())
        .unwrap_or("local")
        .to_string()
}

pub fn save_active(name: &str) {
    let mut data = read_data();
    data["active_environment"] = serde_json::json!(name);
    write_data(&data);
}

pub fn delete(name: &str) {
    let mut data = read_data();
    let mut envs = environments_from(&data);
    envs.retain(|e| e.name != name);
    data["environments"] = serde_json::to_value(&envs).unwrap_or_default();
    write_data(&data);

    if read_active() == name {
        let new_active = envs
            .first()
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "local".into());
        save_active(&new_active);
    }
}

pub fn rename(old_name: &str, new_name: &str) {
    let mut data = read_data();
    let mut envs = environments_from(&data);
    if let Some(env) = envs.iter_mut().find(|e| e.name == old_name) {
        env.name = new_name.to_string();
    }
    data["environments"] = serde_json::to_value(&envs).unwrap_or_default();
    write_data(&data);

    if read_active() == old_name {
        save_active(new_name);
    }
}

pub fn init_workspace(path: &str) {
    if path.is_empty() {
        return;
    }
    let env_path = PathBuf::from(path).join(".arc").join("environments.json");
    if !env_path.exists() {
        if let Some(parent) = env_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            &env_path,
            serde_json::to_string_pretty(&default_data()).unwrap_or_default(),
        );
    }
}

pub fn interpolate(input: &str) -> String {
    let data = read_data();
    let active = data
        .get("active_environment")
        .and_then(|v| v.as_str())
        .unwrap_or("local");
    let vars: HashMap<String, String> = environments_from(&data)
        .into_iter()
        .find(|e| e.name == active)
        .map(|e| {
            e.variables
                .iter()
                .filter(|v| v.active)
                .map(|v| (v.key.clone(), v.value.clone()))
                .collect()
        })
        .unwrap_or_default();

    if vars.is_empty() {
        return input.to_string();
    }

    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next();
            let mut key = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '}' && chars.peek() == Some(&'}') {
                    chars.next();
                    chars.next();
                    break;
                }
                key.push(chars.next().unwrap());
            }
            match vars.get(key.trim()) {
                Some(val) => result.push_str(val),
                None => result.push_str(&format!("{{{{{}}}}}", key.trim())),
            }
        } else {
            result.push(c);
        }
    }
    result
}
