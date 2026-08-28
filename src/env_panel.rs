use gpui::*;

use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};

use crate::actions::{CopyEnv, DeleteEnv};
use crate::env_fs::EnvFileSystem;
use crate::env_playground::Environment;
use crate::icons::IconName;
use crate::request_fs::KeyValue;
use crate::settings_panel::AppSettings;

pub enum EnvPanelEvent {
    EnvActivated { name: String },
    EnvDeleted { name: String },
}

impl EventEmitter<EnvPanelEvent> for EnvPanel {}

pub struct EnvPanel {
    pub envs: Vec<String>,
    collapsed: bool,
    context_target: Option<String>,
    focus_handle: FocusHandle,
}

impl EnvPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let envs = Self::read_env_names();

        Self {
            envs,
            collapsed: true,
            context_target: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn read_env_names() -> Vec<String> {
        let content = EnvFileSystem::read_environment_variables();
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

    fn build_item_context(
        &self,
        item: SidebarMenuItem,
        name: String,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        let this = cx.weak_entity();

        item.context_menu(move |menu, _, cx| {
            this.update(cx, |p, _| {
                p.context_target = Some(name.clone());
            })
            .ok();

            menu.menu("Copy Variables", Box::new(CopyEnv))
                .separator()
                .menu("Delete", Box::new(DeleteEnv))
        })
    }

    fn render_items(&self, cx: &mut Context<Self>) -> Vec<SidebarMenuItem> {
        self.envs
            .iter()
            .map(|name| {
                let name = name.clone();

                let mut item = SidebarMenuItem::new(name.clone()).icon(IconName::Variable);

                let activate_name = name.clone();

                item = item.on_click(cx.listener(move |_, _, _window, cx| {
                    cx.emit(EnvPanelEvent::EnvActivated {
                        name: activate_name.clone(),
                    });
                }));

                item = self.build_item_context(item, name, cx);
                item
            })
            .collect()
    }

    fn read_env_from_disk(&self, name: &str) -> Vec<KeyValue> {
        let content = EnvFileSystem::read_environment_variables();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter()
            .find(|e| e.name == name)
            .map(|e| e.variables)
            .unwrap_or_default()
    }

    fn handle_copy_env(&mut self, _: &CopyEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            let vars = self.read_env_from_disk(&name);
            let json = serde_json::to_string_pretty(&vars).unwrap_or_default();
            cx.write_to_clipboard(ClipboardItem::new_string(json));
        }
    }

    fn handle_delete_env(&mut self, _: &DeleteEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            EnvFileSystem::delete_environment(&name);
            cx.emit(EnvPanelEvent::EnvDeleted { name: name.clone() });
            self.refresh(cx);
        }
    }
}

impl Render for EnvPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let side = AppSettings::global(cx)
            .panel
            .env_panel
            .sidebar_dock
            .to_side();
        let _dock_left = side == gpui_component::Side::Left;

        let sidebar = Sidebar::new("env-sidebar")
            .collapsible(SidebarCollapsible::Offcanvas)
            .collapsed(self.collapsed)
            .side(side)
            .child(
                SidebarGroup::new("Environments")
                    .child(SidebarMenu::new().children(self.render_items(cx))),
            );

        div()
            .id("env-panel")
            .track_focus(&self.focus_handle)
            .h_full()
            .on_action(cx.listener(Self::handle_copy_env))
            .on_action(cx.listener(Self::handle_delete_env))
            .child(sidebar)
    }
}
