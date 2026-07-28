use gpui::*;
use gpui_component::{
    ActiveTheme, IndexPath, StyledExt,
    input::{Input, InputState, TabSize},
    select::{Select, SelectEvent, SelectState},
};

pub struct Body {
    body: Entity<InputState>,
    body_type: Entity<SelectState<Vec<String>>>,
}

impl Body {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let body_types: Vec<String> = vec!["text", "json", "html"]
            .into_iter()
            .map(|bt| bt.to_string())
            .collect();

        let selected_body_type = body_types.iter().position(|m| *m == "json").unwrap_or(0);
        let initial_language = body_types
            .get(selected_body_type)
            .map(|s| s.as_str())
            .unwrap_or("json")
            .to_string();
        let body_type_state = cx.new(|cx| {
            SelectState::new(
                body_types,
                Some(IndexPath {
                    section: 0,
                    row: selected_body_type,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        let body = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .code_editor(&initial_language)
        });

        cx.subscribe_in(
            &body_type_state,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let SelectEvent::Confirm(Some(body_type)) = event {
                    this.body.update(cx, |editor, cx| {
                        editor.set_highlighter(body_type, cx);
                        cx.notify();
                    })
                }
            },
        )
        .detach();

        Self {
            body,
            body_type: body_type_state,
        }
    }

    pub fn value(&self, cx: &App) -> String {
        self.body.read(cx).value().to_string()
    }
}

impl Render for Body {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .gap(px(4.))
            .child(div().w(px(110.)).child(Select::new(&self.body_type)))
            .child(
                div()
                    .flex_basis(DefiniteLength::Fraction(0.75))
                    .min_h(px(0.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(Input::new(&self.body).size_full().appearance(false)),
            )
            .into_any_element()
    }
}
