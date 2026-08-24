use gpui::Action;

#[derive(Clone, PartialEq, Action)]
pub struct CreateFile;

#[derive(Clone, PartialEq, Action)]
pub struct CreateFolder;

#[derive(Clone, PartialEq, Action)]
pub struct DeleteItem;

#[derive(Clone, PartialEq, Action)]
pub struct TrashItem;

#[derive(Clone, PartialEq, Action)]
pub struct StressTestPlayground;

#[derive(Clone, PartialEq, Action)]
pub struct RenameItem;

#[derive(Clone, PartialEq, Action)]
pub struct CopyPath;

#[derive(Clone, PartialEq, Action)]
pub struct CopyRelativePath;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockSidebarLeft;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockSidebarRight;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockEnvPanelLeft;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct DockEnvPanelRight;

#[derive(Clone, PartialEq, Action)]
pub struct OpenSettings;

#[derive(Clone, PartialEq, Action)]
pub struct QuitArc;

#[derive(Clone, PartialEq, Action)]
pub struct CopySettings;

#[derive(Clone, PartialEq, Action)]
pub struct OpenEnvironmentVariables;

#[derive(Clone, PartialEq, Action)]
pub struct CopyEnvironmentVariables;

#[derive(Clone, PartialEq, Action)]
pub struct ThemeChange;
