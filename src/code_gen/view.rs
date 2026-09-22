use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    ActiveTheme, StyledExt, h_flex,
    input::Editor,
    select::Select,
};
use gpui_kit::*;

use super::model::CodeScreen;
use crate::ui::IconName;

impl Render for CodeScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current_code();
        div()
            .id("code-screen")
            .flex_1()
            .w_full()
            .h_full()
            .v_flex()
            .bg(cx.theme().background)
            .child(
                div().flex_none().w_full().px(px(24.)).pt(px(12.)).child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(div().w(px(140.)).child(Select::new(&self.lang_picker())))
                        .child(
                            Button::new("code-copy")
                                .label("Copy")
                                .icon(IconName::Copy)
                                .tooltip("Copy snippet")
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        current.clone(),
                                    ));
                                })),
                        ),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.))
                    .overflow_y_scrollbar()
                    .p(px(24.))
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_md()
                            .overflow_hidden()
                            .child(
                                Editor::new(&self.code_editor())
                                    .w_full()
                                    .h_full()
                                    .appearance(false)
                                    .readonly(true),
                            ),
                    ),
            )
    }
}
