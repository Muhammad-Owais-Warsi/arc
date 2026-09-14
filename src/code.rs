use crate::dock::tabs::CenterTab;
use crate::helpers::render_method_tag;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::*;

pub struct CodeScreen {
    focus: FocusHandle,
}

impl CodeScreen {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }
}

impl Panel for CodeScreen {
    fn panel_name(&self) -> &'static str {
        "code"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for CodeScreen {}

impl Focusable for CodeScreen {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl CenterTab for CodeScreen {
    fn tab_label(&self, _cx: &App) -> SharedString {
        "Code".into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(render_method_tag("CODE").into_any_element())
    }
}

impl Render for CodeScreen {
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
