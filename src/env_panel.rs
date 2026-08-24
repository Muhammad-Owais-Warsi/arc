use gpui::*;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};

use crate::config_fs::ConfigFileSystem;
use crate::env::Environment;
use crate::icons::IconName;

pub enum EnvPanelEvent {
    EnvActivated { name: String },
}

impl EventEmitter<EnvPanelEvent> for EnvPanel {}

pub struct EnvPanel {
    pub envs: Vec<String>,
    collapsed: bool,
}

impl EnvPanel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let envs = Self::read_env_names();

        Self {
            envs,
            collapsed: true,
        }
    }

    fn read_env_names() -> Vec<String> {
        let content = ConfigFileSystem::read_environment_variables();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter().map(|e| e.name).collect()
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.envs = Self::read_env_names();
        cx.notify();
    }

    pub fn set_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.collapsed = collapsed;
        cx.notify();
    }

    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    fn render_items(&self, cx: &mut Context<Self>) -> Vec<SidebarMenuItem> {
        self.envs
            .iter()
            .map(|name| {
                let name = name.clone();
                let item = SidebarMenuItem::new(name.clone()).icon(IconName::Variable);

                let item = item.on_click(cx.listener(move |_, _, _window, cx| {
                    cx.emit(EnvPanelEvent::EnvActivated { name: name.clone() });
                }));

                item
            })
            .collect()
    }
}

impl Render for EnvPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = Sidebar::new("env-sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.collapsed)
            .side(gpui_component::Side::Right)
            .child(
                SidebarGroup::new("Environments")
                    .child(SidebarMenu::new().children(self.render_items(cx))),
            );

        div().id("env-panel").h_full().child(sidebar)
    }
}
