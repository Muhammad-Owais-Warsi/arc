use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::ContextMenuExt;
use gpui_component::separator::Separator;
use gpui_component::status_bar::StatusBar;
use gpui_component::{ActiveTheme, Icon, Sizable};

use crate::actions::{DockEnvPanelLeft, DockEnvPanelRight, DockSidebarLeft, DockSidebarRight};
use crate::icons::IconName;
use crate::settings_panel::SidebarDock;

pub struct Footer {
    show_toggle: bool,
    project_panel_collapsed: bool,
    env_panel_collapsed: bool,
    response_collapsed: bool,
    project_panel_dock: SidebarDock,
    env_panel_dock: SidebarDock,
}

#[derive(Clone, Debug)]
pub enum FooterEvent {
    ToggleResponse,
    ToggleProjectPanel,
    ToggleEnvPanel,
}

impl EventEmitter<FooterEvent> for Footer {}

impl Footer {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            show_toggle: false,
            project_panel_collapsed: true,
            env_panel_collapsed: true,
            response_collapsed: true,
            project_panel_dock: SidebarDock::Left,
            env_panel_dock: SidebarDock::Right,
        }
    }

    pub fn set_show_toggle(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_toggle = show;
        cx.notify();
    }

    pub fn set_project_panel_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.project_panel_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_env_panel_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.env_panel_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_response_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        self.response_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_project_panel_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        self.project_panel_dock = dock;
        cx.notify();
    }

    pub fn set_env_panel_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        self.env_panel_dock = dock;
        cx.notify();
    }

    fn render_project_panel_toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = !self.project_panel_collapsed;
        let dock = self.project_panel_dock;
        Button::new("toggle-project-panel")
            .ghost()
            .small()
            .toggled(is_open)
            .icon(
                Icon::new(IconName::FolderTree)
                    .when(is_open, |icon| icon.text_color(cx.theme().primary)),
            )
            .tooltip("Project Panel")
            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                cx.emit(FooterEvent::ToggleProjectPanel);
            }))
            .context_menu(move |menu, _, _| {
                menu.menu_with_check(
                    "Dock Left",
                    dock == SidebarDock::Left,
                    Box::new(DockSidebarLeft),
                )
                .menu_with_check(
                    "Dock Right",
                    dock == SidebarDock::Right,
                    Box::new(DockSidebarRight),
                )
            })
    }

    fn render_env_panel_toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = !self.env_panel_collapsed;
        let dock = self.env_panel_dock;
        Button::new("toggle-env-panel")
            .ghost()
            .small()
            .toggled(is_open)
            .icon(
                Icon::new(IconName::SquareMenu)
                    .when(is_open, |icon| icon.text_color(cx.theme().primary)),
            )
            .tooltip("Environment Panel")
            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                cx.emit(FooterEvent::ToggleEnvPanel);
            }))
            .context_menu(move |menu, _, _| {
                menu.menu_with_check(
                    "Dock Left",
                    dock == SidebarDock::Left,
                    Box::new(DockEnvPanelLeft),
                )
                .menu_with_check(
                    "Dock Right",
                    dock == SidebarDock::Right,
                    Box::new(DockEnvPanelRight),
                )
            })
    }

    fn render_response_panel_toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = !self.response_collapsed;
        Button::new("toggle-response")
            .ghost()
            .small()
            .toggled(is_open)
            .icon(
                Icon::new(IconName::PanelBottom)
                    .when(is_open, |icon| icon.text_color(cx.theme().primary)),
            )
            .tooltip("Response")
            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                cx.emit(FooterEvent::ToggleResponse);
            }))
    }
}

impl Render for Footer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pp_left = self.project_panel_dock == SidebarDock::Left;
        let ep_left = self.env_panel_dock == SidebarDock::Left;
        let pp_right = self.project_panel_dock == SidebarDock::Right;
        let ep_right = self.env_panel_dock == SidebarDock::Right;

        StatusBar::new()
            .when(pp_left, |this| {
                this.left(self.render_project_panel_toggle_button(cx))
            })
            .when(pp_left && ep_left, |this| this.left(Separator::vertical()))
            .when(ep_left, |this| {
                this.left(self.render_env_panel_toggle_button(cx))
            })
            .when(pp_right, |this| {
                this.right(self.render_project_panel_toggle_button(cx))
            })
            .when(pp_right && ep_right, |this| {
                this.right(Separator::vertical())
            })
            .when(ep_right, |this| {
                this.right(self.render_env_panel_toggle_button(cx))
            })
            .when(self.show_toggle, |this| {
                this.right(Separator::vertical())
            })
            .when(self.show_toggle, |this| {
                this.right(self.render_response_panel_toggle_button(cx))
            })
    }
}
