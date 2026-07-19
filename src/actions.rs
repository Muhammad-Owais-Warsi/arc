use crate::helpers::next_id;
use crate::{ApiClient, Node};
use gpui::{Action, Context, Window};

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent_id: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct RenameFile {
    pub node_id: usize,
    pub new_name: String,
}

impl ApiClient {
    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };

        let parent_path = match ws.nodes.get(&action.parent_id) {
            Some(node) => node.path.clone(),
            None => return,
        };

        match crate::fs::create_file("new", &parent_path) {
            Ok(path) => {
                let id = next_id();
                let new_node = Node {
                    id,
                    name: "new.json".to_string(),
                    path,
                    is_file: true,
                };

                ws.nodes.insert(id, new_node);
                if let Some(parent) = ws.nodes.get_mut(&action.parent_id) {
                    parent.children.push(id);
                }

                cx.notify();
            }
            Err(err) => eprintln!("Failed to create file: {err}"),
        }
    }

    pub fn handle_rename(
        &mut self,
        action: &RenameFile,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ws) = self.workspaces.get_mut(self.selected_workspace) else {
            return;
        };

        let old_path = match ws.nodes.get(&action.node_id) {
            Some(node) => node.path.clone(),
            None => return,
        };

        let new_path = format!(
            "{}/{}",
            std::path::Path::new(&old_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            &action.new_name
        );

        match crate::fs::rename_file(&old_path, &new_path) {
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
}
