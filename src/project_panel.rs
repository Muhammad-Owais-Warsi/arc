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
    focus: FocusHandle,
    context_target: Option<usize>,
}

impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl ProjectPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            name: String::new(),
            path: String::new(),
            nodes: HashMap::new(),
            root_id: Vec::new(),
            sidebar_collapsed: true,
            active_node_id: None,
            pending_action: None,
            focus: cx.focus_handle(),
            context_target: None,
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
        self.context_target = None;

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
        &self,
        item: SidebarMenuItem,
        node_id: usize,
        is_file: bool,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let this = cx.weak_entity();
        let focus = self.focus.clone();
        let is_workspace_root = self.is_workspace_root(node_id);

        item.context_menu(move |menu, _, cx| {
            this.update(cx, move |p, cx| {
                p.context_target = Some(node_id);
            })
            .ok();

            let menu = menu.min_w(px(200.)).action_context(focus.clone());

            let menu = if !is_file {
                menu.menu("Create File", Box::new(CreateFile))
                    .menu("Create Folder", Box::new(CreateFolder))
                    .separator()
            } else {
                menu.menu("Stress Test", Box::new(StressTestPlayground))
                    .separator()
            };

            let menu = menu
                .menu("Copy Path", Box::new(CopyPath))
                .menu("Copy Relative Path", Box::new(CopyRelativePath))
                .separator();

            if !is_workspace_root {
                menu.menu("Rename", Box::new(RenameItem))
                    .menu("Trash", Box::new(TrashItem))
                    .menu("Delete", Box::new(DeleteItem))
            } else {
                menu
            }
        })
    }

    fn is_workspace_root(&self, node_id: usize) -> bool {
        node_id == self.root_id[0]
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

        item.on_click(cx.listener(move |this, _event, window, cx| {
            window.focus(&this.focus, cx);
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

        item = self.build_node_context(item, node_id, is_file, cx);

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
        cx.emit(ProjectPanelEvent::StressTestPlayground {
            path: node.path.clone(),
            node_name: node.name.clone(),
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
        _: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(parent_id) = self.folder_target() else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("file-name"));
        self.initiate_pending(PendingAction::CreateFile { parent_id, input }, window, cx);
    }

    pub fn handle_create_folder(
        &mut self,
        _: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(parent_id) = self.folder_target() else {
            return;
        };
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
        self.initiate_pending(PendingAction::Rename { node_id, input }, window, cx);
    }

    pub fn handle_delete_item(
        &mut self,
        _: &DeleteItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(path) = self.nodes.get(&node_id).map(|n| n.path.clone()) else {
            return;
        };
        let is_file = self.nodes.get(&node_id).map(|n| n.is_file).unwrap_or(false);
        if fs::delete_file_or_folder(Path::new(&path)).is_err() {
            return;
        }
        self.remove_node_from_tree(node_id);
        cx.emit(ProjectPanelEvent::FileDeleted {
            node_id,
            path,
            is_file,
        });
        cx.notify();
    }

    pub fn handle_trash_item(
        &mut self,
        _: &TrashItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node_id) = self.target_id() else {
            return;
        };
        let Some(path) = self.nodes.get(&node_id).map(|n| n.path.clone()) else {
            return;
        };
        if fs::trash_file_or_folder(Path::new(&path)).is_err() {
            return;
        }

        self.remove_node_from_tree(node_id);

        cx.emit(ProjectPanelEvent::FileTrashed {
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
                        let clean_name = new_name.strip_suffix(".json").unwrap_or(&new_name);
                        ws.nodes.insert(
                            id,
                            Node {
                                id,
                                name: clean_name.to_string(),
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
                        let old_name = node.name.clone();
                        let clean_name = old_name.strip_suffix(".json").unwrap_or(&old_name);
                        node.name = clean_name.to_string();
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

impl Render for ProjectPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws_name = self.name.clone();
        let root_ids: Vec<usize> = self.root_id.clone();

        let sidebar = if self.nodes.is_empty() {
            Sidebar::new("api-sidebar")
                .collapsible(SidebarCollapsible::Offcanvas)
                .collapsed(self.sidebar_collapsed)
                .side(
                    AppSettings::global(cx)
                        .panel
                        .project_panel
                        .sidebar_dock
                        .to_side(),
                )
                .into_element()
        } else {
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
                .child(
                    SidebarGroup::new(&ws_name).child(
                        SidebarMenu::new()
                            .children(root_ids.iter().map(|&id| self.render_node(id, cx))),
                    ),
                )
                .into_element()
        };

        div()
            .id("project-panel")
            .track_focus(&self.focus)
            .h_full()
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_create_folder))
            .on_action(cx.listener(Self::handle_rename_item))
            .on_action(cx.listener(Self::handle_delete_item))
            .on_action(cx.listener(Self::handle_trash_item))
            .on_action(cx.listener(Self::handle_copy_path))
            .on_action(cx.listener(Self::handle_copy_relative_path))
            .on_action(cx.listener(Self::activate_stress_test_playground))
            .child(sidebar)
            .into_element()
    }
}
