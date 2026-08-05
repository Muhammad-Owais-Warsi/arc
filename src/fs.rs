use crate::auth::AuthType;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, fs::read_to_string, io, path::Path};

#[derive(Serialize, Deserialize)]
struct KeyValue {
    key: String,
    value: String,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct Auth {
    auth_type: AuthType,
    username: String,
    password: String,
    token: String,
}

#[derive(Serialize, Deserialize)]
struct Body {
    body_type: String,
    body: String,
}

#[derive(Serialize, Deserialize)]
struct FileContent {
    name: String,
    url: String,
    method: String,
    params: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    auth: Auth,
    body: Body,
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
