// use gpui::Window;
use crate::actions::{CreateFile, RenameFile};
use crate::helpers::{build_method_tag, next_id, read_dir_to_nodes};
use crate::{ApiClient, fs};
use gpui::*;
use gpui_component::IconName;
use std::path::PathBuf;
// use gpui_component::sidebar::Sidebar;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use std::collections::HashMap;
// use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) enum ProjectPanelEvent {
    FileActivated {
        node_id: usize,
        name: String,
        path: String,
        method: String,
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
}

impl EventEmitter<ProjectPanelEvent> for ProjectPanel {}

impl ProjectPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<ApiClient>) -> Entity<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let workspace_path = home.join("projects").join("react-app");
        let tree = read_dir_to_nodes(&workspace_path);
        let workspace = Workspace {
            name: "react-app".into(),
            path: workspace_path.to_string_lossy().to_string(),
            nodes: tree.nodes,
            root_id: tree.root_ids,
        };

        let sidebar = cx.new(|_| Self {
            workspaces: vec![workspace],
            selected_workspace: 0,
            sidebar_collapsed: false,
            active_node_id: None,
        });

        sidebar
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

        let mut item = SidebarMenuItem::new(name.clone())
            .suffix(move |_, _| {
                if is_file {
                    div().child(build_method_tag(&method_for_suffix))
                } else {
                    div()
                }
            })
            .active(self.active_node_id == Some(node_id));

        if !is_file {
            item = item.context_menu(move |menu, _window, _cx| {
                menu.menu_with_icon(
                    "Create File",
                    IconName::File,
                    Box::new(CreateFile {
                        parent_id: node_id_for_menu,
                    }),
                )
            });
        }

        if is_file {
            let rename_node_id = node_id;
            item = item.context_menu(move |menu, _window, _cx| {
                menu.menu_with_icon(
                    "Rename",
                    IconName::Redo,
                    Box::new(RenameFile {
                        node_id: rename_node_id,
                        new_name: "renamed.json".to_string(),
                    }),
                )
            });

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

        if node.children.is_empty() {
            item
        } else {
            let mut children = Vec::new();
            for &child_id in &node.children {
                children.push(self.render_node(child_id, cx));
            }
            item.children(children)
        }
    }

    pub fn handle_create_file(&mut self, action: &CreateFile, cx: &mut Context<Self>) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };
        let Some(parent_path) = ws.nodes.get(&action.parent_id).map(|n| n.path.clone()) else {
            return;
        };

        match fs::create_file("new", &parent_path) {
            Ok(path) => {
                let id = next_id();
                ws.nodes.insert(
                    id,
                    Node {
                        id,
                        name: "new.json".to_string(),
                        path,
                        is_file: true,
                        method: "GET".to_string(),
                        children: vec![],
                    },
                );
                if let Some(parent) = ws.nodes.get_mut(&action.parent_id) {
                    parent.children.push(id);
                }
                cx.notify();
            }
            Err(err) => eprintln!("Failed to create file: {err}"),
        }
    }

    pub fn handle_rename(&mut self, action: &RenameFile, cx: &mut Context<Self>) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };
        let Some(old_path) = ws.nodes.get(&action.node_id).map(|n| n.path.clone()) else {
            return;
        };

        let new_path = format!(
            "{}/{}",
            std::path::Path::new(&old_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            &action.new_name
        );

        match fs::rename_file(&old_path, &new_path) {
            Ok(_) => {
                if let Some(node) = ws.nodes.get_mut(&action.node_id) {
                    node.name = action.new_name.clone();
                    node.path = new_path;
                }
            }
            Err(err) => eprintln!("Failed to rename file: {err}"),
        }
        cx.notify();
    }

    pub fn set_node_method(&mut self, node_id: usize, method: &str) {
        if let Some(ws) = self.workspaces.get_mut(self.selected_workspace) {
            crate::helpers::update_node_method(&mut ws.nodes, node_id, method);
        }
    }

    pub fn toggle_collapsed(&mut self) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
    }

    pub fn collapsed(&self) -> bool {
        self.sidebar_collapsed
    }
}

impl Render for ProjectPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = &self.workspaces[self.selected_workspace];
        Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.sidebar_collapsed)
            .child(SidebarGroup::new(&ws.name).child(
                SidebarMenu::new().children(ws.root_id.iter().map(|&id| self.render_node(id, cx))),
            ))
    }
}
