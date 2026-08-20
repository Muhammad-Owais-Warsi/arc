// use gpui::Window;
use crate::actions::{
    CopyPath, CopyRelativePath, CreateFile, CreateFolder, DeleteItem, RenameItem,
    StressTestPlayground, TrashItem,
};
use crate::fs::{self, read_request_method};
use crate::helpers::{next_id, render_method_tag};
use crate::settings_panel::AppSettings;
use gpui::*;
// use gpui_component::Icon;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::icons::IconName;
use std::path::{Path, PathBuf};
// use gpui_component::sidebar::Sidebar;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use std::collections::HashMap;
use walkdir::WalkDir;

pub struct DirTree {
    pub root_ids: Vec<usize>,
    pub nodes: HashMap<usize, Node>,
}

#[derive(Clone, Debug)]
pub enum ProjectPanelEvent {
    FileActivated {
        node_id: usize,
        name: String,
        path: String,
        method: String,
    },
    FileRenamed {
        node_id: usize,
        new_name: String,
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
    CopyPath {
        path: String,
    },
    CopyRelativePath {
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

pub struct ProjectPanel {
    name: String,
    path: String,
    nodes: HashMap<usize, Node>,
    root_id: Vec<usize>,
    sidebar_collapsed: bool,
    active_node_id: Option<usize>,
    pending_action: Option<PendingAction>,
}

impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl ProjectPanel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            name: String::new(),
            path: String::new(),
            nodes: HashMap::new(),
            root_id: Vec::new(),
            sidebar_collapsed: true,
            active_node_id: None,
            pending_action: None,
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
        self.active_node_id = None;
        self.pending_action = None;

        cx.notify();
    }

    pub fn list_workspace_dirs() -> Vec<(String, PathBuf)> {
        let projects_dir = fs::config_dir();

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
            .filter_map(Result::ok)
            .filter(|entry| entry.path() != active_dir_path)
        {
            let path = entry.path();

            if entry.file_type().is_dir()
                && matches!(
                    entry.file_name().to_str(),
                    Some("target" | "node_modules" | ".git" | "dist")
                )
            {
                continue;
            }

            let id = next_id();
            let name = entry.file_name().to_string_lossy().to_string();
            let clean_name = name.strip_suffix(".json").unwrap_or(&name);

            let node = Node {
                id,
                path: path.to_string_lossy().to_string(),
                name: clean_name.to_string(),
                method: if entry.file_type().is_file() {
                    read_request_method(path)
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

    fn build_node_context(
        item: SidebarMenuItem,
        ws_path: &str,
        path: &str,
        name: &str,
        node_id: usize,
        is_file: bool,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let is_workspace_root = ws_path == path;
        let ws_path = ws_path.to_owned();
        let path = path.to_owned();
        let name = name.to_owned();

        item.context_menu(move |menu, _, _| {
            let menu = menu.min_w(px(200.));

            let menu = if !is_file {
                menu.menu("Create File", Box::new(CreateFile { parent_id: node_id }))
                    .menu(
                        "Create Folder",
                        Box::new(CreateFolder { parent_id: node_id }),
                    )
                    .separator()
            } else {
                menu.menu(
                    "Stress Test",
                    Box::new(StressTestPlayground {
                        path: path.clone(),
                        node_id,
                        node_name: name.clone(),
                    }),
                )
                .separator()
            };

            let menu = menu
                .menu("Copy Path", Box::new(CopyPath { path: path.clone() }))
                .menu(
                    "Copy Relative Path",
                    Box::new(CopyRelativePath {
                        path: Path::new(&path)
                            .strip_prefix(&ws_path)
                            .map(|rel| rel.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    }),
                )
                .separator();

            if !is_workspace_root {
                menu.menu(
                    "Rename",
                    Box::new(RenameItem {
                        node_id,
                        node_name: name.clone(),
                        new_name: String::new(),
                    }),
                )
                .menu(
                    "Trash",
                    Box::new(TrashItem {
                        node_id,
                        path: path.clone(),
                    }),
                )
                .menu(
                    "Delete",
                    Box::new(DeleteItem {
                        node_id,
                        path: path.clone(),
                        is_file,
                    }),
                )
            } else {
                menu
            }
        })
    }

    fn add_node_click_handler(
        item: SidebarMenuItem,
        node_id: usize,
        name: &str,
        path: &str,
        method: &str,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let name = name.to_owned();
        let path = path.to_owned();
        let method = method.to_owned();

        item.on_click(cx.listener(move |this, _event, _window, cx| {
            this.active_node_id = Some(node_id);

            cx.emit(ProjectPanelEvent::FileActivated {
                node_id,
                name: name.clone(),
                path: path.clone(),
                method: method.clone(),
            });

            cx.notify();
        }))
    }

    fn node_pending_file_or_folder(
        &mut self,
        item: SidebarMenuItem,
        pending_parent_id: Option<usize>,
        children_ids: Vec<usize>,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let mut children = Vec::new();

        if let Some(parent_id) = pending_parent_id {
            if let Some(ref pending) = self.pending_action {
                let input = match pending {
                    PendingAction::CreateFile { input, .. }
                    | PendingAction::CreateFolder { input, .. } => Some(input.clone()),
                    _ => None,
                };
                if let Some(input) = input {
                    children.push(SidebarMenuItem::new("").suffix(move |_window, _cx| {
                        Input::new(&input).appearance(false).into_any_element()
                    }));
                }
            }
        }

        children.extend(
            children_ids
                .iter()
                .map(|&child_id| self.render_node(child_id, cx)),
        );

        item.children(children)
    }

    pub fn render_node(&mut self, node_id: usize, cx: &mut Context<Self>) -> SidebarMenuItem {
        let Some(node) = self.nodes.get(&node_id) else {
            return SidebarMenuItem::new("???".to_string());
        };

        let is_file = node.is_file;
        let name = node.name.clone();
        let method = node.method.clone();
        let path = node.path.clone();
        let children_ids: Vec<usize> = node.children.clone();

        let method_for_suffix = method.clone();
        let is_renaming = matches!(
            &self.pending_action,
            Some(PendingAction::Rename { node_id: id, .. }) if *id == node_id
        );

        let mut item = if is_renaming {
            if let Some(PendingAction::Rename { input, .. }) = &self.pending_action {
                let input = input.clone();
                SidebarMenuItem::new("")
                    .suffix(move |_, _| Input::new(&input).appearance(false).into_any_element())
            } else {
                SidebarMenuItem::new(name.clone())
            }
        } else {
            SidebarMenuItem::new(name.clone()).suffix(move |_, _| {
                if is_file {
                    div()
                        .child(render_method_tag(&method_for_suffix))
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            })
        };

        let is_active = self.active_node_id == Some(node_id);
        if !is_file && !self.sidebar_collapsed {
            item = item.icon(if is_active {
                IconName::FolderOpen
            } else {
                IconName::Folder
            });
        }

        item = Self::build_node_context(item, &self.path, &path, &name, node_id, is_file, cx);

        let is_active = self.active_node_id == Some(node_id);
        item = item.active(is_active);

        if is_file {
            item = Self::add_node_click_handler(item, node_id, &name, &path, &method, cx)
        }

        let pending_parent_id = match &self.pending_action {
            Some(PendingAction::CreateFile { parent_id, .. })
            | Some(PendingAction::CreateFolder { parent_id, .. })
                if *parent_id == node_id =>
            {
                Some(*parent_id)
            }
            _ => None,
        };

        if children_ids.is_empty() && pending_parent_id.is_none() {
            item
        } else {
            self.node_pending_file_or_folder(item, pending_parent_id, children_ids, cx)
        }
    }

    pub fn activate_stress_test_playground(
        &mut self,
        action: &StressTestPlayground,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.emit(ProjectPanelEvent::StressTestPlayground {
            path: action.path.clone(),
            node_name: action.node_name.clone(),
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
        cx.notify();
    }

    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("file-name"));
        self.initiate_pending(
            PendingAction::CreateFile {
                parent_id: action.parent_id,
                input,
            },
            window,
            cx,
        );
    }

    pub fn handle_create_folder(
        &mut self,
        action: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("folder-name"));
        self.initiate_pending(
            PendingAction::CreateFolder {
                parent_id: action.parent_id,
                input,
            },
            window,
            cx,
        );
    }

    pub fn handle_rename_item(
        &mut self,
        action: &RenameItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current_name = action.node_name.clone();

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&current_name)
                .placeholder("new name")
        });

        self.initiate_pending(
            PendingAction::Rename {
                node_id: action.node_id,
                input,
            },
            window,
            cx,
        );
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

    fn confirm_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pending = match self.pending_action.take() {
            Some(pending) => pending,
            None => return,
        };

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
                    return;
                };

                let name = input.read(cx).value().trim().to_string();

                if name.is_empty() {
                    return cx.notify();
                }

                match fs::create_file(&name, &parent_path) {
                    Ok(path) => {
                        let id = next_id();
                        let new_name = format!("{name}.json");

                        ws.nodes.insert(
                            id,
                            Node {
                                id,
                                name: new_name,
                                path,
                                is_file: true,
                                method: "GET".to_string(),
                                children: vec![],
                            },
                        );

                        Self::insert_child_sorted(&mut ws.nodes, parent_id, id);

                        self.nodes = ws.nodes;
                        cx.notify();
                    }
                    Err(err) => {
                        eprintln!("Failed to create file: {err}");
                    }
                }
            }

            PendingAction::CreateFolder { parent_id, input } => {
                let Some(parent_path) = ws.nodes.get(&parent_id).map(|node| node.path.clone())
                else {
                    return;
                };

                let name = input.read(cx).value().trim().to_string();

                if name.is_empty() {
                    return cx.notify();
                }

                match fs::create_folder(&name, &parent_path) {
                    Ok(path) => {
                        let id = next_id();

                        ws.nodes.insert(
                            id,
                            Node {
                                id,
                                name,
                                path,
                                is_file: false,
                                method: String::new(),
                                children: vec![],
                            },
                        );

                        Self::insert_child_sorted(&mut ws.nodes, parent_id, id);

                        self.nodes = ws.nodes;
                        cx.notify();
                    }
                    Err(err) => {
                        eprintln!("Failed to create folder: {err}");
                    }
                }
            }

            PendingAction::Rename { node_id, input } => {
                let new_name = input.read(cx).value().trim().to_string();

                if new_name.is_empty() {
                    return cx.notify();
                }

                let Some(old_path) = ws.nodes.get(&node_id).map(|node| node.path.clone()) else {
                    return;
                };

                let new_path = format!(
                    "{}/{}",
                    Path::new(&old_path)
                        .parent()
                        .map(|path| path.to_string_lossy())
                        .unwrap_or_default(),
                    new_name
                );

                if fs::rename_item(&old_path, &new_path).is_ok() {
                    if let Some(node) = ws.nodes.get_mut(&node_id) {
                        node.name = new_name.clone();
                        node.path = new_path;
                    }

                    cx.emit(ProjectPanelEvent::FileRenamed { node_id, new_name });
                } else {
                    eprintln!("Failed to rename");
                }

                self.nodes = ws.nodes;
                cx.notify();
            }
        }
    }

    fn cancel_action(&mut self, cx: &mut Context<Self>) {
        self.pending_action = None;
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

    pub fn handle_delete_item(&mut self, action: &DeleteItem, cx: &mut Context<Self>) {
        if fs::delete_file_or_folder(Path::new(&action.path)).is_err() {
            return;
        }

        self.remove_node_from_tree(action.node_id);

        cx.emit(ProjectPanelEvent::FileDeleted {
            node_id: action.node_id,
            path: action.path.clone(),
            is_file: action.is_file,
        });
        cx.notify();
    }

    pub fn handle_trash_item(&mut self, action: &TrashItem, cx: &mut Context<Self>) {
        if fs::trash_file_or_folder(Path::new(&action.path)).is_err() {
            return;
        }

        self.remove_node_from_tree(action.node_id);

        cx.emit(ProjectPanelEvent::FileTrashed {
            node_id: action.node_id,
            path: action.path.clone(),
        });
        cx.notify();
    }

    pub fn set_node_method(&mut self, node_id: usize, method: &str) {
        Self::update_node_method(&mut self.nodes, node_id, method);
    }

    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.sidebar_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_active_node(&mut self, node_id: Option<usize>) {
        self.active_node_id = node_id
    }
}

impl Render for ProjectPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.nodes.is_empty() {
            return Sidebar::new("api-sidebar")
                .collapsible(SidebarCollapsible::Offcanvas)
                .collapsed(self.sidebar_collapsed)
                .side(
                    AppSettings::global(cx)
                        .panel
                        .project_panel
                        .sidebar_dock
                        .to_side(),
                )
                .into_element();
        }

        let ws_name = self.name.clone();
        let root_ids: Vec<usize> = self.root_id.clone();

        Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .side(
                AppSettings::global(cx)
                    .panel
                    .project_panel
                    .sidebar_dock
                    .to_side(),
            )
            .collapsed(self.sidebar_collapsed)
            .child(SidebarGroup::new(&ws_name).child(
                SidebarMenu::new().children(root_ids.iter().map(|&id| self.render_node(id, cx))),
            ))
            .into_element()
    }
}
