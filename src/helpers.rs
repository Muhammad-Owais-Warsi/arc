use crate::project_panel::Node;
use gpui::*;
use gpui_component::tag::Tag;
use gpui_component::{ColorName, Sizable};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn build_method_tag(method: &str) -> impl IntoElement {
    match method {
        "GET" => Tag::color(ColorName::Green).outline().child("GET").xsmall(),

        "POST" => Tag::color(ColorName::Blue).outline().child("POST").xsmall(),

        "PUT" => Tag::color(ColorName::Yellow)
            .outline()
            .child("PUT")
            .xsmall(),

        "PATCH" => Tag::color(ColorName::Orange)
            .outline()
            .child("PATCH")
            .xsmall(),

        "DELETE" => Tag::color(ColorName::Red)
            .outline()
            .child("DELETE")
            .xsmall(),

        "HEAD" => Tag::color(ColorName::Purple)
            .outline()
            .child("HEAD")
            .xsmall(),

        "OPTIONS" => Tag::color(ColorName::Gray)
            .outline()
            .child("OPTIONS")
            .xsmall(),

        _ => Tag::color(ColorName::Neutral)
            .outline()
            .child("Nan")
            .xsmall(),
    }
}

pub fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn update_node_method(nodes: &mut HashMap<usize, Node>, id: usize, method: &str) -> bool {
    if let Some(node) = nodes.get_mut(&id) {
        node.method = method.to_string();
        return true;
    }
    false
}

pub fn read_request_method(path: &std::path::Path) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return String::new();
    };
    value
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_uppercase()
}

pub struct DirTree {
    pub root_ids: Vec<usize>,
    pub nodes: HashMap<usize, Node>,
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
            let child = read_dir_to_nodes(&path);
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
