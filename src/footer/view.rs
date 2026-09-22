use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::separator::Separator;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::model::{Footer, FooterEvent};
use crate::actions::{DockEnvPanelLeft, DockEnvPanelRight, DockSidebarLeft, DockSidebarRight};
use crate::ui::IconName;
use crate::settings::SidebarDock;

impl Footer {
    fn render_file_panel_toggle_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.file_panel_open();
        let dock = self.file_panel_dock();
        Button::new("toggle-file-panel")
            .ghost()
            .small()
            .toggled(is_open)
            .icon(
                Icon::new(IconName::FolderTree)
                    .when(is_open, |icon| icon.text_color(cx.theme().primary)),
            )
            .tooltip("File Panel")
            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                cx.emit(FooterEvent::ToggleFilePanel);
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
        let is_open = self.env_panel_open();
        let dock = self.env_panel_dock();
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
        let is_open = self.response_open();
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
        let fp_left = self.file_panel_dock() == SidebarDock::Left;
        let ep_left = self.env_panel_dock() == SidebarDock::Left;
        let fp_right = self.file_panel_dock() == SidebarDock::Right;
        let ep_right = self.env_panel_dock() == SidebarDock::Right;

        StatusBar::new()
            .when(fp_left, |this| {
                this.left(self.render_file_panel_toggle_button(cx))
            })
            .when(fp_left && ep_left, |this| this.left(Separator::vertical()))
            .when(ep_left, |this| {
                this.left(self.render_env_panel_toggle_button(cx))
            })
            .when(fp_right, |this| {
                this.right(self.render_file_panel_toggle_button(cx))
            })
            .when(self.show_response_toggle(), |this| {
                this.right(Separator::vertical())
            })
            .when(self.show_response_toggle(), |this| {
                this.right(self.render_response_panel_toggle_button(cx))
            })
            .when(ep_right && self.show_response_toggle(), |this| {
                this.right(Separator::vertical())
            })
            .when(ep_right, |this| {
                this.right(self.render_env_panel_toggle_button(cx))
            })
    }
}
