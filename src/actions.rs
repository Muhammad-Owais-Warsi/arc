use gpui_kit::Action;

// Shared: handled by more than one view (playground + project panel).
#[derive(Clone, PartialEq, Action)]
pub struct CopyAsCode;

// App-global: handled by ApiClient, dispatched from footer/titlebar menus.
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
pub struct ThemeChange;
