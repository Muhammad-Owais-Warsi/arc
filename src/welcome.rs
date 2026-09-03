use crate::playground::Playground;
use crate::response_panel::ResponsePanel;
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::*;

pub struct WelcomeScreen {}

impl WelcomeScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Playground for WelcomeScreen {
    fn method(&self, _cx: &App) -> String {
        "WELCOME".to_string()
    }
    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        None
    }
}

impl Render for WelcomeScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex_1().flex().items_center().justify_center().child(
            div()
                .flex_col()
                .items_center()
                .gap_0p5()
                .child(
                    div()
                        .text_2xl()
                        .font_bold()
                        .text_center()
                        .text_color(cx.theme().foreground)
                        .child("Welcome to Arc"),
                )
                .child(
                    div()
                        .text_sm()
                        .italic()
                        .text_center()
                        .text_color(cx.theme().muted_foreground)
                        .child("API client built for speed"),
                ),
        )
    }
}
