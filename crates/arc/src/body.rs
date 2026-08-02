// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use gpui::*;
use ui::{
    ActiveTheme, IndexPath, StyledExt,
    input::{Input, InputState, TabSize},
    select::{Select, SelectEvent, SelectState},
};

struct BodyType {
    label: &'static str,
    language: &'static str,
}

pub struct Body {
    body: Entity<InputState>,
    body_type: Entity<SelectState<Vec<String>>>,
}

impl Body {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let body_types = vec![
            BodyType {
                label: "Text",
                language: "text",
            },
            BodyType {
                label: "JSON",
                language: "json",
            },
            BodyType {
                label: "HTML",
                language: "html",
            },
        ];
        let select_items: Vec<String> = body_types.iter().map(|t| t.label.to_string()).collect();

        let selected = body_types
            .iter()
            .position(|t| t.language == "json")
            .unwrap_or(0);

        let initial_language = body_types[selected].language.to_string();

        let body_type_state = cx.new(|cx| {
            SelectState::new(
                select_items,
                Some(IndexPath {
                    section: 0,
                    row: selected,
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
            move |this: &mut Self, _, event, _, cx| {
                if let SelectEvent::Confirm(Some(label)) = event {
                    if let Some(body_type) = body_types.iter().find(|t| t.label == label) {
                        this.body.update(cx, |editor, cx| {
                            editor.set_highlighter(body_type.language, cx);
                            cx.notify();
                        });
                    }
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
