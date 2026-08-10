// use gpui::Window;
use crate::actions::{CreateFile, CreateFolder, RenameItem, StressTestPlayground};
use crate::fs::{self, read_request_method};
use crate::helpers::{next_id, render_method_tag};
use crate::settings_panel::AppSettings;
use gpui::*;
// use gpui_component::Icon;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::icons::IconName;
use std::path::PathBuf;
// use gpui_component::sidebar::Sidebar;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use std::collections::HashMap;
// use std::path::PathBuf;

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
    StressTestPlayground {
        path: String,

        node_name: String,
    },
}

#[derive(Clone)]
struct Workspace {
    name: String,
    path: String,
    nodes: HashMap<usize, Node>,
    root_id: Vec<usize>,
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

pub struct ProjectPanel {
    workspaces: Vec<Workspace>,
    selected_workspace: usize,
    sidebar_collapsed: bool,
    active_node_id: Option<usize>,
    new_file: Option<(usize, Entity<InputState>)>,
    new_folder: Option<(usize, Entity<InputState>)>,
    rename_item: Option<(usize, Entity<InputState>)>,
}

impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl ProjectPanel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            workspaces: Vec::new(),
            selected_workspace: 0,
            sidebar_collapsed: false,
            active_node_id: None,
            new_file: None,
            new_folder: None,
            rename_item: None,
        }
    }

    pub fn get_workspace(&self, idx: usize) -> Option<(String, String)> {
        self.workspaces
            .get(idx)
            .map(|w| (w.name.clone(), w.path.clone()))
    }

    pub fn reset(&mut self, _window: &mut Window, cx: &mut Context<Self>, name: String) -> usize {
        let Some(ix) = self.workspaces.iter().position(|w| w.name == name) else {
            return self.selected_workspace;
        };
        if ix != self.selected_workspace {
            self.selected_workspace = ix;
            self.active_node_id = None;
            self.new_file = None;
            self.new_folder = None;
            self.rename_item = None;

            cx.notify();
        }
        ix
    }

    pub async fn list_workspace_dirs() -> Vec<(String, PathBuf)> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let projects_dir = home.join("projects");
        let mut dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    dirs.push((
                        entry.file_name().to_string_lossy().to_string(),
                        entry.path(),
                    ));
                }
            }
        }
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        dirs
    }

    pub fn set_workspaces(
        &mut self,
        dirs: Vec<(String, PathBuf)>,
        cx: &mut Context<Self>,
    ) -> usize {
        self.workspaces = dirs
            .into_iter()
            .map(|(name, path)| Workspace {
                name,
                path: path.to_string_lossy().to_string(),
                nodes: HashMap::new(),
                root_id: Vec::new(),
            })
            .collect();
        self.selected_workspace = self
            .workspaces
            .iter()
            .position(|w| w.name == "react-app")
            .unwrap_or(0);

        cx.notify();
        self.selected_workspace
    }

    pub fn add_workspace(&mut self, name: String, path: String, cx: &mut Context<Self>) {
        self.workspaces.push(Workspace {
            name,
            path,
            nodes: HashMap::new(),
            root_id: Vec::new(),
        });
        self.selected_workspace = self.workspaces.len() - 1;
        self.active_node_id = None;
        self.new_file = None;
        self.new_folder = None;
        self.rename_item = None;

        cx.notify();
    }
    // Called after the recursive scan of one workspace completes.
    pub fn set_workspace_tree(&mut self, ix: usize, tree: DirTree, cx: &mut Context<Self>) {
        if let Some(ws) = self.workspaces.get_mut(ix) {
            ws.nodes = tree.nodes;
            ws.root_id = tree.root_ids;
        }

        cx.notify();
    }

    pub fn read_dir_to_nodes(dir: &std::path::Path) -> DirTree {
        Self::read_dir_to_nodes_depth(dir, 0)
    }

    fn read_dir_to_nodes_depth(dir: &std::path::Path, depth: usize) -> DirTree {
        let mut nodes: HashMap<usize, Node> = HashMap::new();
        let mut root_ids: Vec<usize> = Vec::new();
        if depth > 8 {
            return DirTree { root_ids, nodes };
        }

        let Ok(raw) = std::fs::read_dir(dir) else {
            return DirTree { root_ids, nodes };
        };

        for entry in raw.flatten() {
            let file_type = entry.file_type().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            if name == "target" || name == "node_modules" || name == ".git" || name == "dist" {
                continue;
            }

            if file_type.map_or(false, |ft| ft.is_dir()) {
                let id = next_id();
                let child = Self::read_dir_to_nodes_depth(&path, depth + 1);
                nodes.extend(child.nodes);
                nodes.insert(
                    id,
                    Node {
                        id,
                        path: path.to_string_lossy().to_string(),
                        name: name.clone(),
                        method: String::new(),
                        is_file: false,
                        children: child.root_ids,
                    },
                );
                root_ids.push(id);
            } else if file_type.map_or(false, |ft| ft.is_file()) {
                let id = next_id();
                nodes.insert(
                    id,
                    Node {
                        id,
                        path: path.to_string_lossy().to_string(),
                        name,
                        method: read_request_method(&path),
                        is_file: true,
                        children: vec![],
                    },
                );
                root_ids.push(id);
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

    pub fn render_node(&self, node_id: usize, cx: &mut Context<Self>) -> SidebarMenuItem {
        let Some(ws) = self.workspaces.get(self.selected_workspace) else {
            return SidebarMenuItem::new("???".to_string());
        };
        let Some(node) = ws.nodes.get(&node_id) else {
            return SidebarMenuItem::new("???".to_string());
        };

        let is_file = node.is_file;
        let name = node.name.clone();
        let method = node.method.clone();
        let path = node.path.clone();

        let method_for_suffix = method.clone();
        let _node_id_for_click = node_id;
        let node_id_for_menu = node_id;
        let is_renaming = self
            .rename_item
            .as_ref()
            .map_or(false, |(id, _)| *id == node_id);

        let mut item = if is_renaming {
            let input = self.rename_item.as_ref().unwrap().1.clone();
            SidebarMenuItem::new("")
                .suffix(move |_, _| Input::new(&input).appearance(false).into_any_element())
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

        let rename_node_name = name.clone();
        let stress_playground_node_path = path.clone();

        item = item.context_menu(move |menu, _, _| {
            let menu = if !is_file {
                menu.menu(
                    "Create File",
                    // IconName::File,
                    Box::new(CreateFile {
                        parent_id: node_id_for_menu,
                    }),
                )
                .menu(
                    "Create Folder",
                    // IconName::Folder,
                    Box::new(CreateFolder {
                        parent_id: node_id_for_menu,
                    }),
                )
                .separator()
            } else {
                menu.menu(
                    "Stress Test",
                    Box::new(StressTestPlayground {
                        path: stress_playground_node_path.clone(),
                        node_id,
                        node_name: rename_node_name.clone(),
                    }),
                )
                .separator()
            };
            menu.menu(
                "Rename",
                // IconName::Rename,
                Box::new(RenameItem {
                    node_id,
                    node_name: rename_node_name.clone(),
                    new_name: String::new(),
                }),
            )
        });

        let is_active = self.active_node_id == Some(node_id);
        item = item.active(is_active);

        if !is_file && !self.sidebar_collapsed {
            item = item.icon(if is_active {
                IconName::FolderOpen
            } else {
                IconName::Folder
            })
        }

        if is_file {
            let name_for_click = name.clone();
            let path_for_click = path.clone();
            let method_for_click = node.method.clone();
            item = item.on_click(cx.listener(move |this, _event, _window, cx| {
                this.active_node_id = Some(node_id);
                cx.emit(ProjectPanelEvent::FileActivated {
                    node_id,
                    name: name_for_click.clone(),
                    path: path_for_click.clone(),
                    method: method_for_click.clone(),
                });
                cx.notify();
            }));
        }

        let is_pending_file = self
            .new_file
            .as_ref()
            .map_or(false, |(pid, _)| *pid == node_id);
        let is_pending_folder = self
            .new_folder
            .as_ref()
            .map_or(false, |(pid, _)| *pid == node_id);

        if node.children.is_empty() && !is_pending_file && !is_pending_folder {
            item
        } else {
            let mut children = Vec::new();

            if is_pending_file {
                if let Some((_, ref input)) = self.new_file {
                    let input_clone = input.clone();
                    children.push(SidebarMenuItem::new("").suffix(move |_window, _cx| {
                        Input::new(&input_clone)
                            .appearance(false)
                            .into_any_element()
                    }));
                }
            }

            if is_pending_folder {
                if let Some((_, ref input)) = self.new_folder {
                    let input_clone = input.clone();
                    children.push(SidebarMenuItem::new("").suffix(move |_window, _cx| {
                        Input::new(&input_clone)
                            .appearance(false)
                            .into_any_element()
                    }));
                }
            }

            children.extend(
                node.children
                    .iter()
                    .map(|&child_id| self.render_node(child_id, cx)),
            );

            item.children(children)
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

    pub fn confirm_create_file(
        &mut self,
        parent_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };
        let Some(parent_path) = ws.nodes.get(&parent_id).map(|n| n.path.clone()) else {
            return;
        };

        let Some((_, input)) = self.new_file.take() else {
            return;
        };
        let name = input.read(cx).value().trim().to_string();
        if name.is_empty() {
            return cx.notify();
        }

        match fs::create_file(&name, &parent_path) {
            Ok(path) => {
                let id = next_id();
                ws.nodes.insert(
                    id,
                    Node {
                        id,
                        name: format!("{name}.json").to_string(),
                        path,
                        is_file: true,
                        method: "GET".to_string(),
                        children: vec![],
                    },
                );
                if let Some(parent) = ws.nodes.get_mut(&parent_id) {
                    parent.children.push(id); // we can keep check if it is file or folder
                }
                cx.notify();
            }
            Err(err) => eprintln!("Failed to create file: {err}"),
        }
    }

    pub fn cancel_create_file(&mut self, cx: &mut Context<Self>) {
        self.new_file = None;
        cx.notify();
    }

    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("file-name"));

        cx.subscribe_in(&input, window, {
            let parent_id = action.parent_id;
            move |this: &mut Self, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.confirm_create_file(parent_id, window, cx),
                InputEvent::Blur => this.cancel_create_file(cx),
                _ => {}
            }
        })
        .detach();

        input.update(cx, |i, cx| i.focus(window, cx));
        self.new_file = Some((action.parent_id, input));
        cx.notify();
    }

    pub fn confirm_create_folder(
        &mut self,
        parent_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };
        let Some(parent_path) = ws.nodes.get(&parent_id).map(|n| n.path.clone()) else {
            return;
        };

        let Some((_, input)) = self.new_folder.take() else {
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
                        name: format!("{name}").to_string(),
                        path,
                        is_file: false,
                        method: "".to_string(),
                        children: vec![],
                    },
                );
                if let Some(parent) = ws.nodes.get_mut(&parent_id) {
                    parent.children.push(id); // we can keep check if it is file or folder
                }
                cx.notify();
            }
            Err(err) => eprintln!("Failed to create folder: {err}"),
        }
    }

    pub fn cancel_create_folder(&mut self, cx: &mut Context<Self>) {
        self.new_folder = None;
        cx.notify();
    }

    pub fn handle_create_folder(
        &mut self,
        action: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("folder-name"));

        cx.subscribe_in(&input, window, {
            let parent_id = action.parent_id;
            move |this: &mut Self, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.confirm_create_folder(parent_id, window, cx),
                InputEvent::Blur => this.cancel_create_folder(cx),
                _ => {}
            }
        })
        .detach();

        input.update(cx, |i, cx| i.focus(window, cx));
        self.new_folder = Some((action.parent_id, input));
        cx.notify();
    }

    pub fn handle_rename_item(
        &mut self,
        action: &RenameItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current_name = self.workspaces[self.selected_workspace]
            .nodes
            .get(&action.node_id)
            .map(|n| n.name.clone())
            .unwrap_or_default();

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(&current_name)
                .placeholder("new name")
        });
        cx.subscribe_in(&input, window, {
            let node_id = action.node_id;
            move |this: &mut Self, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.confirm_rename_item(node_id, window, cx),
                InputEvent::Blur => this.cancel_rename_item(cx),
                _ => {}
            }
        })
        .detach();
        input.update(cx, |i, cx| i.focus(window, cx));
        self.rename_item = Some((action.node_id, input));
        cx.notify();
    }

    pub fn confirm_rename_item(
        &mut self,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, input)) = self.rename_item.take() else {
            return;
        };
        let new_name = input.read(cx).value().to_string();

        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };
        let Some(old_path) = ws.nodes.get(&node_id).map(|n| n.path.clone()) else {
            return;
        };

        let new_path = format!(
            "{}/{}",
            std::path::Path::new(&old_path)
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default(),
            new_name
        );

        let rename_ok = fs::rename_item(&old_path, &new_path).is_ok();
        if rename_ok {
            if let Some(node) = ws.nodes.get_mut(&node_id) {
                node.name = new_name.clone();
                node.path = new_path;
            }
            cx.emit(ProjectPanelEvent::FileRenamed { node_id, new_name });
        } else {
            eprintln!("Failed to rename");
        }
        cx.notify();
    }

    pub fn cancel_rename_item(&mut self, cx: &mut Context<Self>) {
        self.rename_item = None;
        cx.notify();
    }

    pub fn set_node_method(&mut self, node_id: usize, method: &str) {
        if let Some(ws) = self.workspaces.get_mut(self.selected_workspace) {
            Self::update_node_method(&mut ws.nodes, node_id, method);
        }
    }

    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.sidebar_collapsed = collapsed;
        cx.notify();
    }

    pub fn get_selected_workspace(&self) -> &str {
        self.workspaces
            .get(self.selected_workspace)
            .map(|w| w.name.as_str())
            .unwrap_or("no workspace")
    }

    pub fn workspace_names(&self) -> Vec<String> {
        self.workspaces.iter().map(|w| w.name.clone()).collect()
    }

    pub fn set_active_node(&mut self, node_id: Option<usize>) {
        self.active_node_id = node_id
    }
}

impl Render for ProjectPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(ws) = self.workspaces.get(self.selected_workspace) else {
            return Sidebar::new("api-sidebar")
                .collapsible(SidebarCollapsible::Offcanvas)
                .collapsed(self.sidebar_collapsed)
                .side(AppSettings::global(cx).sidebar_dock.to_side())
                .into_element();
        };
        let sidebar = Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .side(AppSettings::global(cx).sidebar_dock.to_side())
            .collapsed(self.sidebar_collapsed)
            .child(SidebarGroup::new(&ws.name).child(
                SidebarMenu::new().children(ws.root_id.iter().map(|&id| self.render_node(id, cx))),
            ));

        sidebar.into_element()
    }
}
