use gpui_kit::component::{
    ActiveTheme, StyledExt,
    input::{Editor},
    select::Select,
};
use gpui_kit::*;

use super::model::Body;

impl Render for Body {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .gap(px(4.))
            .child(div().w(px(110.)).child(Select::new(&self.body_type_select())))
            .child(
                div()
                    .flex_basis(DefiniteLength::Fraction(0.75))
                    .min_h(px(0.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        Editor::new(&self.body_editor())
                            .size_full()
                            .appearance(false)
                            .bordered(false),
                    ),
            )
            .into_any_element()
    }
}
