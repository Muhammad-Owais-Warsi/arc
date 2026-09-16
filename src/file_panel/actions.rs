use gpui_kit::Action;

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
