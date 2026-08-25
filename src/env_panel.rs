use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::Button;
use gpui_component::button::ButtonVariants;
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};

use std::cell::RefCell;
use std::rc::Rc;

use crate::env_fs::EnvFileSystem;
use crate::env_playground::Environment;
use crate::icons::IconName;
use crate::settings_panel::AppSettings;

pub enum EnvPanelEvent {
    EnvActivated { name: String },
    EnvDeleted { name: String },
}

impl EventEmitter<EnvPanelEvent> for EnvPanel {}

pub struct EnvPanel {
    pub envs: Vec<String>,
    collapsed: bool,
    pending_delete: Rc<RefCell<Option<String>>>,
}

impl EnvPanel {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let envs = Self::read_env_names();

        Self {
            envs,
            collapsed: true,
            pending_delete: Rc::new(RefCell::new(None)),
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

    fn render_items(&self, cx: &mut Context<Self>) -> Vec<SidebarMenuItem> {
        self.envs
            .iter()
            .map(|name| {
                let name = name.clone();

                let item = SidebarMenuItem::new(name.clone()).icon(IconName::Variable);

                let activate_name = name.clone();

                let item = item.on_click(cx.listener(move |_, _, _window, cx| {
                    cx.emit(EnvPanelEvent::EnvActivated {
                        name: activate_name.clone(),
                    });
                }));

                let del_name = name.clone();
                let pending = self.pending_delete.clone();
                let item = item.suffix(move |_, _cx| {
                    let name = del_name.clone();
                    let pending = pending.clone();
                    Button::new(format!("del-env-{name}"))
                        .ghost()
                        .small()
                        .icon(IconName::Trash)
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            *pending.borrow_mut() = Some(name.clone());
                        })
                        .into_any_element()
                });

                item
            })
            .collect()
    }
}

impl Render for EnvPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(name) = self.pending_delete.borrow_mut().take() {
            cx.emit(EnvPanelEvent::EnvDeleted { name });
        }

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

        div().id("env-panel").h_full().child(sidebar)
    }
}
