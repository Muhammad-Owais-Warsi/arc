//! Keymap orchestration: embedded defaults, then the user file over them.
//! Later-added bindings win, so the user file always takes precedence.

mod file;

pub use file::KeymapFile;

use std::path::PathBuf;

use gpui_kit::{App, Global, KeyBinding, KeyBindingMetaIndex};

use crate::assets::Assets;

const DEFAULT_META: KeyBindingMetaIndex = KeyBindingMetaIndex(2);
const USER_META: KeyBindingMetaIndex = KeyBindingMetaIndex(0);

/// Bindings gpui-kit registers at init (input editing keys, menus, lists…).
/// Snapshotted once, before our first clear, and restored on every reload —
/// otherwise `clear_key_bindings` would delete Backspace/Delete/arrows in
/// every input. Snapshot entries carry no meta (User rank), but ours and the
/// user file are appended after, so ties still resolve in later-added order.
struct ComponentDefaults {
    bindings: Vec<KeyBinding>,
}

impl Global for ComponentDefaults {}

/// Zed layout: one default file per OS, because gpui matches modifiers
/// exactly (on Windows Ctrl reports `control`, never `platform`).
fn default_asset_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "keymaps/default-macos.json"
    } else if cfg!(target_os = "windows") {
        "keymaps/default-windows.json"
    } else {
        "keymaps/default-linux.json"
    }
}

fn initial_template_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "keymaps/initial-keymap-macos.json"
    } else if cfg!(target_os = "windows") {
        "keymaps/initial-keymap-windows.json"
    } else {
        "keymaps/initial-keymap-linux.json"
    }
}

fn user_keymap_path() -> PathBuf {
    // Same convention as src/fs/env.rs.
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arc")
        .join("keymap.json")
}

/// Seed ~/.config/.arc/keymap.json from the embedded template once.
pub fn ensure_user_keymap() {
    let path = user_keymap_path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Some(template) = Assets::get(initial_template_name()) {
        let _ = std::fs::write(&path, template.data.as_ref());
    }
}

/// Defaults first, user file over them.
pub fn reload_keymaps(cx: &mut App) {
    // Snapshot whatever is bound right now on the first call only
    // (component defaults); later reloads reuse the stored snapshot so our
    // own bindings are never snapshotted and duplicated.
    if !cx.has_global::<ComponentDefaults>() {
        let snapshot: Vec<KeyBinding> = cx
            .key_bindings()
            .borrow()
            .bindings()
            .cloned()
            .collect();
        cx.set_global(ComponentDefaults { bindings: snapshot });
    }
    let component = cx.global::<ComponentDefaults>().bindings.clone();
    cx.clear_key_bindings();
    cx.bind_keys(component);

    let mut problems = Vec::new();
    let mut bound = 0;
    if let Some(raw) = Assets::get(default_asset_name()) {
        match KeymapFile::parse(std::str::from_utf8(raw.data.as_ref()).unwrap_or("")) {
            Ok(file) => {
                let (bindings, mut errors) = file.load(cx, DEFAULT_META);
                problems.append(&mut errors);
                bound += bindings.len();
                cx.bind_keys(bindings);
            }
            Err(e) => problems.push(format!("{}: {e:?}", default_asset_name())),
        }
    }
    match std::fs::read_to_string(user_keymap_path()) {
        Ok(text) => match KeymapFile::parse(&text) {
            Ok(file) => {
                let (bindings, mut errors) = file.load(cx, USER_META);
                problems.append(&mut errors);
                bound += bindings.len();
                cx.bind_keys(bindings);
            }
            Err(e) => problems.push(format!("keymap.json: {e:?}")),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => problems.push(format!("keymap.json: {e:?}")),
    }
    eprintln!("[keymap] bound {bound} bindings");
    for problem in problems {
        eprintln!("[keymap] {problem}");
    }
}
