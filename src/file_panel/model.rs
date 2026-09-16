use super::actions::{
    CopyPath, CopyRelativePath, CreateFile, CreateFolder, DeleteItem, RenameItem,
    StressTestPlayground, TrashItem,
};
use crate::actions::CopyAsCode;
use crate::fs;
use crate::helpers::next_id;
use gpui_kit::*;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::tree::{TreeEvent, TreeItem, TreeState};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct DirTree {
    pub root_ids: Vec<usize>,
    pub nodes: HashMap<usize, Node>,
}

#[derive(Clone, Debug)]
pub enum FilePanelEvent {
    FileActivated {
        node_id: usize,
        name: String,
        path: String,
        method: String,
    },
    FileRenamed {
        node_id: usize,
        new_name: String,
        new_path: String,
    },
    FileDeleted {
        node_id: usize,
        path: String,
        is_file: bool,
    },
    FileTrashed {
        node_id: usize,
        path: String,
    },
    StressTestPlayground {
        path: String,
        node_name: String,
    },
    CopyAsCode {
        node_id: usize,
        path: String,
    },
}

#[derive(Clone)]
pub struct Workspace {
    pub name: String,
    pub path: String,
    pub nodes: HashMap<usize, Node>,
    pub root_id: Vec<usize>,
}

#[derive(Clone)]
pub struct Node {
    pub id: usize,
    pub path: String,
    pub name: String,
    pub method: String,
    pub children: Vec<usize>,
    pub is_file: bool,
}

enum PendingAction {
    CreateFile {
        parent_id: usize,
        input: Entity<InputState>,
    },
    CreateFolder {
        parent_id: usize,
        input: Entity<InputState>,
    },
    Rename {
        node_id: usize,
        input: Entity<InputState>,
    },
}

pub struct FilePanel {
    name: String,
    path: String,
    nodes: HashMap<usize, Node>,
    root_id: Vec<usize>,
    active_node_id: Option<usize>,
    pending_action: Option<PendingAction>,
    pending_error: Option<String>,
    focus: FocusHandle,
    context_target: Option<usize>,
    tree: Entity<TreeState>,
    expanded: HashSet<String>,
}

/// Owned clone of the bits `view.rs` needs to render a frame.
pub struct PanelSnapshot {
    pub workspace_name: String,
    pub nodes: HashMap<usize, Node>,
    pub root_id: Option<usize>,
    pub focus: FocusHandle,
    pub pending_new: Option<Entity<InputState>>,
    pub pending_rename: Option<(usize, Entity<InputState>)>,
    pub pending_error: Option<String>,
}

impl EventEmitter<FilePanelEvent> for FilePanel {}

impl FilePanel {
    pub fn snapshot(&self) -> PanelSnapshot {
        let pending_new = match &self.pending_action {
            Some(PendingAction::CreateFile { input, .. })
            | Some(PendingAction::CreateFolder { input, .. }) => Some(input.clone()),
            _ => None,
        };
        let pending_rename = match &self.pending_action {
            Some(PendingAction::Rename { node_id, input }) => Some((*node_id, input.clone())),
            _ => None,
        };
        PanelSnapshot {
            workspace_name: self.name.clone(),
            nodes: self.nodes.clone(),
            root_id: self.root_id.first().copied(),
            focus: self.focus.clone(),
            pending_new,
            pending_rename,
            pending_error: self.pending_error.clone(),
        }
    }

    pub fn tree_state(&self) -> Entity<TreeState> {
        self.tree.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn focus(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn set_context_target(&mut self, node_id: usize) {
        self.context_target = Some(node_id);
    }

    pub fn click_node(
        &mut self,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        self.active_node_id = Some(node_id);
        if let Some(node) = self.nodes.get(&node_id) {
            if node.is_file {
                cx.emit(FilePanelEvent::FileActivated {
                    node_id,
                    name: node.name.clone(),
                    path: node.path.clone(),
                    method: node.method.clone(),
                });
            }
        }
        cx.notify();
    }
}

impl FilePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| TreeState::new(cx));
        cx.subscribe_in(
            &tree,
            window,
            |this: &mut Self, _, event, _, _| match event {
                TreeEvent::Expanded(id) => {
                    this.expanded.insert(id.to_string());
                }
                TreeEvent::Collapsed(id) => {
                    this.expanded.remove(id.as_ref());
                }
            },
        )
        .detach();
        Self {
            name: String::new(),
            path: String::new(),
            nodes: HashMap::new(),
            root_id: Vec::new(),
            active_node_id: None,
            pending_action: None,
            pending_error: None,
            focus: cx.focus_handle(),
            context_target: None,
            tree,
            expanded: HashSet::new(),
        }
    }

    pub fn set_tree(&mut self, name: String, path: String, tree: DirTree, cx: &mut Context<Self>) {
        let mut nodes = tree.nodes;
        let root_id = next_id();
        nodes.insert(
            root_id,
            Node {
                id: root_id,
                path: path.clone(),
                name: name.clone(),
                method: String::new(),
                is_file: false,
                children: tree.root_ids,
            },
        );
        self.name = name;
        self.path = path;
        self.nodes = nodes;
        self.root_id = vec![root_id];
        self.pending_action = None;
        self.context_target = None;
        self.pending_error = None;
        self.expanded.clear();
        self.expanded.insert(root_id.to_string());

        self.rebuild_tree(cx);
        cx.notify();
    }

    /// Rebuild kit tree items from the node map, restoring expansion.
    fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
        let items: Vec<TreeItem> = self
            .root_id
            .clone()
            .into_iter()
            .filter_map(|id| self.build_tree_item(id))
            .collect();
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
    }

    fn build_tree_item(&self, node_id: usize) -> Option<TreeItem> {
        let node = self.nodes.get(&node_id)?;
        let mut item = TreeItem::new(node_id.to_string(), node.name.clone());
        if !node.children.is_empty() || self.pending_parent_is(node_id) {
            let mut children: Vec<TreeItem> = node
                .children
                .iter()
                .filter_map(|id| self.build_tree_item(*id))
                .collect();
            if self.pending_parent_is(node_id) {
                children.push(TreeItem::new("pending:new", ""));
            }
            item = item.children(children);
        }
        if self.expanded.contains(&node_id.to_string()) {
            item = item.expanded(true);
        }
        Some(item)
    }

    fn pending_parent_is(&self, node_id: usize) -> bool {
        matches!(
            &self.pending_action,
            Some(PendingAction::CreateFile { parent_id, .. })
            | Some(PendingAction::CreateFolder { parent_id, .. })
                if *parent_id == node_id
        )
    }

    pub fn list_workspace_dirs() -> Vec<(String, PathBuf)> {
        let projects_dir = fs::workspace::config_dir();

        let mut dirs: Vec<(String, PathBuf)> = WalkDir::new(&projects_dir)
            .max_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_dir() && !entry.file_name().to_string_lossy().starts_with('.')
            })
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().to_string(),
                    entry.path().to_path_buf(),
                )
            })
            .collect();

        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        dirs
    }

    pub fn read_dir_to_nodes(active_dir_path: &Path) -> DirTree {
        let mut nodes = HashMap::new();
        let mut root_ids = Vec::new();
        let mut path_to_id = HashMap::new();

        for entry in WalkDir::new(active_dir_path)
            .max_depth(8)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && matches!(
                        e.file_name().to_str(),
                        Some("target" | "node_modules" | ".git" | "dist" | ".arc")
                    ))
            })
            .filter_map(Result::ok)
            .filter(|entry| entry.path() != active_dir_path)
        {
            let path = entry.path();

            let id = next_id();
            let name = entry.file_name().to_string_lossy().to_string();
            let clean_name = name.strip_suffix(".json").unwrap_or(&name);

            let node = Node {
                id,
                path: path.to_string_lossy().to_string(),
                name: clean_name.to_string(),
                method: if entry.file_type().is_file() {
                    fs::request::read_method(path)
                } else {
                    String::new()
                },
                is_file: entry.file_type().is_file(),
                children: vec![],
            };

            path_to_id.insert(path.to_path_buf(), id);
            nodes.insert(id, node);

            if let Some(parent) = path.parent() {
                if let Some(&parent_id) = path_to_id.get(parent) {
                    nodes.get_mut(&parent_id).unwrap().children.push(id);
                } else {
                    root_ids.push(id);
                }
            }
        }

        DirTree { root_ids, nodes }
    }

    pub fn update_node_method(nodes: &mut HashMap<usize, Node>, id: usize, method: &str) -> bool {
        if let Some(node) = nodes.get_mut(&id) {
            node.method = method.to_string();
            return true;
        }
        false
    }

    pub fn activate_stress_test_playground(
        &mut self,
        _: &StressTestPlayground,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        cx.emit(FilePanelEvent::StressTestPlayground {
            path: node.path.clone(),
            node_name: node.name.clone(),
        });
    }

    pub fn handle_copy_as_code(
        &mut self,
        _: &CopyAsCode,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        if !node.is_file {
            return;
        }
        cx.emit(FilePanelEvent::CopyAsCode {
            node_id,
            path: node.path.clone(),
        });
    }

    fn initiate_pending(
        &mut self,
        pending: PendingAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = match &pending {
            PendingAction::CreateFile { input, .. }
            | PendingAction::CreateFolder { input, .. }
            | PendingAction::Rename { input, .. } => input.clone(),
        };

        cx.subscribe_in(
            &input,
            window,
            move |this, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.confirm_action(window, cx),
                InputEvent::Blur => this.cancel_action(cx),
                _ => {}
            },
        )
        .detach();

        input.update(cx, |i, cx| i.focus(window, cx));
        self.pending_action = Some(pending);
        self.rebuild_tree(cx);
        cx.notify();
        let focus_input = input.clone();
        window.defer(cx, move |window, cx| {
            focus_input.update(cx, |i, cx| i.focus(window, cx));
        });
    }

    pub fn handle_create_file(
        &mut self,
        _: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(parent_id) = self
            .folder_target()
            .or_else(|| self.root_id.first().copied())
        else {
            return;
        };
        self.expanded.insert(parent_id.to_string());
        self.pending_error = None;
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("file-name"));
        self.initiate_pending(PendingAction::CreateFile { parent_id, input }, window, cx);
    }

    pub fn handle_create_folder(
        &mut self,
        _: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(parent_id) = self
            .folder_target()
            .or_else(|| self.root_id.first().copied())
        else {
            return;
        };
        self.expanded.insert(parent_id.to_string());
        self.pending_error = None;
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("folder-name"));
        self.initiate_pending(PendingAction::CreateFolder { parent_id, input }, window, cx);
    }

    pub fn handle_rename_item(
        &mut self,
        _: &RenameItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let current_name = self
            .nodes
            .get(&node_id)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(current_name)
                .placeholder("new name")
        });
        self.pending_error = None;
        self.initiate_pending(PendingAction::Rename { node_id, input }, window, cx);
    }

    pub fn handle_delete_item(
        &mut self,
        _: &DeleteItem,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(path) = self.nodes.get(&node_id).map(|n| n.path.clone()) else {
            return;
        };
        let is_file = self.nodes.get(&node_id).map(|n| n.is_file).unwrap_or(false);

        if fs::request::delete(Path::new(&path)).is_err() {
            return;
        }
        self.remove_node_from_tree(node_id);
        self.rebuild_tree(cx);
        cx.emit(FilePanelEvent::FileDeleted {
            node_id,
            path,
            is_file,
        });
        cx.notify();
    }

    pub fn handle_trash_item(
        &mut self,
        _: &TrashItem,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(path) = self.nodes.get(&node_id).map(|n| n.path.clone()) else {
            return;
        };
        if fs::request::trash(Path::new(&path)).is_err() {
            return;
        }

        self.remove_node_from_tree(node_id);
        self.rebuild_tree(cx);

        cx.emit(FilePanelEvent::FileTrashed {
            node_id: node_id,
            path: path.clone(),
        });
        cx.notify();
    }

    fn insert_child_sorted(nodes: &mut HashMap<usize, Node>, parent_id: usize, child_id: usize) {
        let Some(child_name) = nodes.get(&child_id).map(|node| node.name.clone()) else {
            return;
        };

        let position = nodes
            .get(&parent_id)
            .and_then(|parent| {
                parent.children.iter().position(|&existing_id| {
                    nodes
                        .get(&existing_id)
                        .map(|node| node.name > child_name)
                        .unwrap_or(false)
                })
            })
            .unwrap_or_else(|| {
                nodes
                    .get(&parent_id)
                    .map(|parent| parent.children.len())
                    .unwrap_or(0)
            });

        if let Some(parent) = nodes.get_mut(&parent_id) {
            parent.children.insert(position, child_id);
        }
    }

    fn confirm_action(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let pending = match self.pending_action.take() {
            Some(pending) => pending,
            None => return,
        };
        self.context_target = None;

        let mut ws = Workspace {
            name: self.name.clone(),
            path: self.path.clone(),
            nodes: self.nodes.clone(),
            root_id: self.root_id.clone(),
        };

        match pending {
            PendingAction::CreateFile { parent_id, input } => {
                let Some(parent_path) = ws.nodes.get(&parent_id).map(|node| node.path.clone())
                else {
                    self.rebuild_tree(cx);
                    return;
                };

                let name = input.read(cx).value().trim().to_string();
                let name = name.strip_suffix(".json").unwrap_or(&name).to_string();

                if name.is_empty() {
                    self.pending_error = Some("Please enter a name.".to_string());
                    self.pending_action =
                        Some(PendingAction::CreateFile { parent_id, input });
                    self.rebuild_tree(cx);
                    return cx.notify();
                }

                match fs::request::file(&name, &parent_path) {
                    Ok(path) => {
                        let id = next_id();
                        ws.nodes.insert(
                            id,
                            Node {
                                id,
                                name: name.clone(),
                                path,
                                is_file: true,
                                method: "GET".to_string(),
                                children: vec![],
                            },
                        );

                        Self::insert_child_sorted(&mut ws.nodes, parent_id, id);

                        self.nodes = ws.nodes;
                        self.expanded.insert(parent_id.to_string());
                        self.active_node_id = Some(id);
                        self.pending_error = None;
                        cx.notify();
                    }
                    Err(err) => {
                        self.pending_error = Some(if err.kind() == std::io::ErrorKind::AlreadyExists {
                            format!("File or directory '{name}' already exists at location. Please choose a different name.")
                        } else {
                            format!("Failed to create '{name}': {err}")
                        });
                        self.pending_action =
                            Some(PendingAction::CreateFile { parent_id, input });
                        cx.notify();
                    }
                }
                self.rebuild_tree(cx);
            }

            PendingAction::CreateFolder { parent_id, input } => {
                let Some(parent_path) = ws.nodes.get(&parent_id).map(|node| node.path.clone())
                else {
                    self.rebuild_tree(cx);
                    return;
                };

                let name = input.read(cx).value().trim().to_string();

                if name.is_empty() {
                    self.pending_error = Some("Please enter a name.".to_string());
                    self.pending_action =
                        Some(PendingAction::CreateFolder { parent_id, input });
                    self.rebuild_tree(cx);
                    return cx.notify();
                }

                match fs::request::folder(&name, &parent_path) {
                    Ok(path) => {
                        let id = next_id();

                        ws.nodes.insert(
                            id,
                            Node {
                                id,
                                name: name.clone(),
                                path,
                                is_file: false,
                                method: String::new(),
                                children: vec![],
                            },
                        );

                        Self::insert_child_sorted(&mut ws.nodes, parent_id, id);

                        self.nodes = ws.nodes;
                        self.expanded.insert(parent_id.to_string());
                        self.active_node_id = Some(id);
                        self.pending_error = None;
                        cx.notify();
                    }
                    Err(err) => {
                        self.pending_error = Some(if err.kind() == std::io::ErrorKind::AlreadyExists {
                            format!("File or directory '{name}' already exists at location. Please choose a different name.")
                        } else {
                            format!("Failed to create '{name}': {err}")
                        });
                        self.pending_action =
                            Some(PendingAction::CreateFolder { parent_id, input });
                        cx.notify();
                    }
                }
                self.rebuild_tree(cx);
            }

            PendingAction::Rename { node_id, input } => {
                let new_name = input.read(cx).value().trim().to_string();

                if new_name.is_empty() {
                    self.rebuild_tree(cx);
                    return cx.notify();
                }

                let Some(old_path) = ws.nodes.get(&node_id).map(|node| node.path.clone()) else {
                    self.rebuild_tree(cx);
                    return;
                };
                let is_file = ws.nodes.get(&node_id).map(|n| n.is_file).unwrap_or(true);

                // Request files live on disk with a .json suffix: normalize so
                // "y" and "y.json" resolve to the same target. Folders keep
                // the typed name verbatim.
                let (display_name, disk_name) = if is_file {
                    let stem = new_name.strip_suffix(".json").unwrap_or(&new_name);
                    (stem.to_string(), format!("{stem}.json"))
                } else {
                    (new_name.clone(), new_name.clone())
                };
                let new_path = format!(
                    "{}/{}",
                    Path::new(&old_path)
                        .parent()
                        .map(|path| path.to_string_lossy())
                        .unwrap_or_default(),
                    disk_name
                );

                // Never silently overwrite: a rename onto an existing path
                // keeps the row open with an inline error instead.
                if new_path != old_path && Path::new(&new_path).exists() {
                    self.pending_error = Some(format!("File or directory '{display_name}' already exists at location. Please choose a different name."));
                    self.pending_action =
                        Some(PendingAction::Rename { node_id, input });
                    self.rebuild_tree(cx);
                    return cx.notify();
                }

                match fs::request::rename(&old_path, &new_path) {
                    Ok(_) => {
                        if let Some(node) = ws.nodes.get_mut(&node_id) {
                            node.name = display_name.clone();
                            node.path = new_path.clone();
                        }
                        self.pending_error = None;

                        cx.emit(FilePanelEvent::FileRenamed {
                            node_id,
                            new_name: display_name,
                            new_path,
                        });
                    }
                    Err(err) => {
                        self.pending_error =
                            Some(format!("Failed to rename: {err}"));
                        self.pending_action =
                            Some(PendingAction::Rename { node_id, input });
                        cx.notify();
                    }
                }

                self.nodes = ws.nodes;
                self.rebuild_tree(cx);
                cx.notify();
            }
        }
    }

    fn cancel_action(&mut self, cx: &mut Context<Self>) {
        self.pending_action = None;
        self.context_target = None;
        self.pending_error = None;
        self.rebuild_tree(cx);
        cx.notify();
    }

    fn remove_node_from_tree(&mut self, node_id: usize) {
        if self.nodes.contains_key(&node_id) {
            let parent_ids: Vec<usize> = self
                .nodes
                .values()
                .filter(|n| n.children.contains(&node_id))
                .map(|n| n.id)
                .collect();
            for parent_id in parent_ids {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&id| id != node_id);
                }
            }
            self.nodes.remove(&node_id);
        }
    }

    pub fn set_node_method(&mut self, node_id: usize, method: &str) {
        Self::update_node_method(&mut self.nodes, node_id, method);
    }

    /// Mark the opened file (kept for context-menu targeting) and move
    /// tree selection to it, so it carries the selected-row background
    /// (Zed-style) instead of a one-off color.
    pub fn set_active_node(&mut self, node_id: Option<usize>, cx: &mut Context<Self>) {
        self.active_node_id = node_id;
        if let Some(id) = node_id {
            let key: SharedString = id.to_string().into();
            self.tree.update(cx, |tree, cx| {
                if let Some(ix) = tree.index_of(&key) {
                    tree.set_selected_index(Some(ix), cx);
                    tree.reveal_item(&key, gpui::ScrollStrategy::Top, cx);
                }
            });
        }
        cx.notify();
    }

    pub fn handle_copy_path(&mut self, _: &CopyPath, _w: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self
            .target_id()
            .and_then(|id| self.nodes.get(&id))
            .map(|n| n.path.clone())
        else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(path));
    }

    pub fn handle_copy_relative_path(
        &mut self,
        _: &CopyRelativePath,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.target_id() else { return };
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let rel = Path::new(&node.path)
            .strip_prefix(&self.path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| node.path.clone());
        cx.write_to_clipboard(ClipboardItem::new_string(rel));
    }

    fn target_id(&self) -> Option<usize> {
        self.context_target.or(self.active_node_id)
    }

    fn folder_target(&self) -> Option<usize> {
        let id = self.target_id()?;
        match self.nodes.get(&id) {
            Some(n) if !n.is_file => Some(id),
            Some(_) => self
                .nodes
                .values()
                .find(|n| n.children.contains(&id))
                .map(|n| n.id),
            None => None,
        }
    }
}

