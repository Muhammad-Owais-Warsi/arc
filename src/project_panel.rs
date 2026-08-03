// use gpui::Window;
use crate::actions::{CreateFile, RenameFile};
use crate::fs;
use crate::helpers::{next_id, render_method_tag};
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
pub(crate) enum ProjectPanelEvent {
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

pub(crate) struct ProjectPanel {
    workspaces: Vec<Workspace>,
    selected_workspace: usize,
    sidebar_collapsed: bool,
    active_node_id: Option<usize>,
    new_file: Option<(usize, Entity<InputState>)>,
    rename_file: Option<(usize, Entity<InputState>)>,
}

impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl ProjectPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let workspace_path = home.join("projects").join("react-app");
        let tree = Self::read_dir_to_nodes(&workspace_path);
        let workspace = Workspace {
            name: "react-app".into(),
            path: workspace_path.to_string_lossy().to_string(),
            nodes: tree.nodes,
            root_id: tree.root_ids,
        };

        Self {
            workspaces: vec![workspace],
            selected_workspace: 0,
            sidebar_collapsed: false,
            active_node_id: None,
            new_file: None,
            rename_file: None,
        }
    }

    pub fn read_dir_to_nodes(dir: &std::path::Path) -> DirTree {
        let mut nodes: HashMap<usize, Node> = HashMap::new();
        let mut root_ids: Vec<usize> = Vec::new();
        let Ok(raw) = std::fs::read_dir(dir) else {
            return DirTree { root_ids, nodes };
        };

        for entry in raw.flatten() {
            let file_type = entry.file_type().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            if file_type.map_or(false, |ft| ft.is_dir()) {
                let id = next_id();
                let child = Self::read_dir_to_nodes(&path);
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
                        method: crate::helpers::read_request_method(&path),
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
        let ws = &self.workspaces[self.selected_workspace];
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
            .rename_file
            .as_ref()
            .map_or(false, |(id, _)| *id == node_id);

        let mut item = if is_renaming {
            let input = self.rename_file.as_ref().unwrap().1.clone();
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

        item = item.context_menu(move |menu, _, _| {
            let menu = if !is_file {
                menu.menu_with_icon(
                    "Create File",
                    IconName::File,
                    Box::new(CreateFile {
                        parent_id: node_id_for_menu,
                    }),
                )
            } else {
                menu
            };
            menu.menu_with_icon(
                "Rename",
                IconName::Rename,
                Box::new(RenameFile {
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

        let is_pending = self
            .new_file
            .as_ref()
            .map_or(false, |(pid, _)| *pid == node_id);

        if node.children.is_empty() && !is_pending {
            item
        } else {
            let mut children = Vec::new();

            if is_pending {
                if let Some((_, ref input)) = self.new_file {
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
                    parent.children.push(id);
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

    pub fn handle_rename_file(
        &mut self,
        action: &RenameFile,
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
                InputEvent::PressEnter { .. } => this.confirm_rename_file(node_id, window, cx),
                InputEvent::Blur => this.cancel_rename_file(cx),
                _ => {}
            }
        })
        .detach();
        input.update(cx, |i, cx| i.focus(window, cx));
        self.rename_file = Some((action.node_id, input));
        cx.notify();
    }

    pub fn confirm_rename_file(
        &mut self,
        node_id: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, input)) = self.rename_file.take() else {
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

        let rename_ok = fs::rename_file(&old_path, &new_path).is_ok();
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

    pub fn cancel_rename_file(&mut self, cx: &mut Context<Self>) {
        self.rename_file = None;
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
        let ws = &self.workspaces[self.selected_workspace];
        let sidebar = Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.sidebar_collapsed)
            .child(SidebarGroup::new(&ws.name).child(
                SidebarMenu::new().children(ws.root_id.iter().map(|&id| self.render_node(id, cx))),
            ));

        sidebar.into_element()
    }
}
