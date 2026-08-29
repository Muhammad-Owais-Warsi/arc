use std::fs;
use std::io;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arc")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn read() -> String {
    fs::read_to_string(settings_path()).unwrap_or_default()
}

pub fn write(content: &str) -> io::Result<()> {
    fs::write(settings_path(), content)
}
