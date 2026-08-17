use crate::auth::AuthType;
use serde::{Deserialize, Serialize};
use std::{
    fs::{OpenOptions, read_to_string, remove_dir_all, remove_file},
    io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Default, PartialEq)]
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
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let projects_dir = home.join("projects");
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

pub fn settings_file_path() -> PathBuf {
    PathBuf::from("./settings.json")
}

pub fn get_settings() -> String {
    std::fs::read_to_string(settings_file_path()).unwrap_or_default()
}

pub fn save_settings(content: &str) -> io::Result<()> {
    std::fs::write(settings_file_path(), content)
}
