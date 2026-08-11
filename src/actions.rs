use gpui::Action;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent_id: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFolder {
    pub parent_id: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct StressTestPlayground {
    pub path: String,
    pub node_id: usize,
    pub node_name: String,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct RenameItem {
    pub node_id: usize,
    pub node_name: String,
    pub new_name: String,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CopyPath {
    pub path: String,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CopyRelativePath {
    pub path: String,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockSidebarLeft;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockSidebarRight;
