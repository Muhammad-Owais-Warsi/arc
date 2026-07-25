use gpui::Action;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent_id: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct RenameFile {
    pub node_id: usize,
    pub node_name: String,
    pub new_name: String,
}
