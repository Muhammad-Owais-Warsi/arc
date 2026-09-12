use crate::dock::tabs::CenterTab;
use crate::helpers::render_method_tag;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::*;

pub struct WelcomeScreen {
    focus: FocusHandle,
}

impl WelcomeScreen {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }
}

impl Panel for WelcomeScreen {
    fn panel_name(&self) -> &'static str {
        "welcome"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for WelcomeScreen {}

impl Focusable for WelcomeScreen {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl CenterTab for WelcomeScreen {
    fn tab_label(&self, _cx: &App) -> SharedString {
        "Welcome".into()
    }

    fn tab_prefix(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        Some(render_method_tag("WELCOME").into_any_element())
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
