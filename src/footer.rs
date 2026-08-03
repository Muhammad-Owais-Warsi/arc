use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Sizable};

use crate::icons::IconName;

pub struct Footer {
    show_toggle: bool,
    show_settings: bool,
}

#[derive(Clone, Debug)]
pub enum FooterEvent {
    ToggleResponse,
    ToggleSettings(bool),
}

impl EventEmitter<FooterEvent> for Footer {}

impl Footer {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            show_toggle: false,
            show_settings: false,
        }
    }

    pub fn set_show_toggle(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_toggle = show;
        cx.notify();
    }
}

impl Render for Footer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .h(px(32.0))
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .flex()
            .items_center()
            .px(px(16.0))
            .when(self.show_toggle, |this| {
                this.child(
                    Button::new("toggle-response")
                        .ghost()
                        .small()
                        .icon(IconName::PanelBottom)
                        .tooltip("Response")
                        .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                            cx.emit(FooterEvent::ToggleResponse);
                        })),
                )
            })
            .child(div().flex_1())
            .child(
                Button::new("toggle-settings")
                    .ghost()
                    .small()
                    .icon(IconName::Settings)
                    .tooltip("Settings")
                    .on_click(cx.listener(|this: &mut Self, _, _window, cx| {
                        this.show_settings = !this.show_settings;
                        cx.emit(FooterEvent::ToggleSettings(this.show_settings));
                        cx.notify();
                    })),
            )
    }
}
