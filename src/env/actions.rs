use gpui_kit::Action;

#[derive(Clone, PartialEq, Action)]
pub struct CopyEnv;

#[derive(Clone, PartialEq, Action)]
pub struct DeleteEnv;
