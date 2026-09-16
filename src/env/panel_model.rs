use gpui_kit::component::tree::{TreeItem, TreeState};
use gpui_kit::*;

use super::actions::{CopyEnv, DeleteEnv};
use super::editor_model::Environment;
use crate::fs;
use crate::fs::request::KeyValue;

pub enum EnvPanelEvent {
    EnvActivated { name: String },
    EnvDeleted { name: String },
}

impl EventEmitter<EnvPanelEvent> for EnvPanel {}

pub struct EnvPanel {
    envs: Vec<String>,
    context_target: Option<String>,
    focus_handle: FocusHandle,
    tree: Entity<TreeState>,
}

impl EnvPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let envs = Self::read_env_names();
        let items: Vec<TreeItem> = envs
            .iter()
            .map(|name| TreeItem::new(name.clone(), name.clone()))
            .collect();
        let tree = cx.new(|cx| TreeState::new(cx).items(items));

        Self {
            envs,
            context_target: None,
            focus_handle: cx.focus_handle(),
            tree,
        }
    }

    pub fn read_env_names() -> Vec<String> {
        let content = fs::env::read_environments();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter().map(|e| e.name).collect()
    }

    pub fn env_names(&self, _cx: &App) -> Vec<String> {
        self.envs.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.envs.is_empty()
    }

    pub fn tree_state(&self) -> Entity<TreeState> {
        self.tree.clone()
    }

    pub fn focus(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub fn set_context_target(&mut self, name: String) {
        self.context_target = Some(name);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.envs = Self::read_env_names();
        let items: Vec<TreeItem> = self
            .envs
            .iter()
            .map(|name| TreeItem::new(name.clone(), name.clone()))
            .collect();
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
        cx.notify();
    }

    fn read_env_from_disk(&self, name: &str) -> Vec<KeyValue> {
        let content = fs::env::read_environments();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter()
            .find(|e| e.name == name)
            .map(|e| e.variables)
            .unwrap_or_default()
    }

    pub fn handle_copy_env(&mut self, _: &CopyEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            let vars = self.read_env_from_disk(&name);
            let json = serde_json::to_string_pretty(&vars).unwrap_or_default();
            cx.write_to_clipboard(ClipboardItem::new_string(json));
        }
    }

    pub fn handle_delete_env(&mut self, _: &DeleteEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            fs::env::delete(&name);
            cx.emit(EnvPanelEvent::EnvDeleted { name: name.clone() });
            self.refresh(cx);
        }
    }
}

