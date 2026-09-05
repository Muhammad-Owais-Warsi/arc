use gpui_kit::component::{
    ActiveTheme, IndexPath, StyledExt,
    input::{Editor, EditorState, InputEvent, TabSize},
    select::{Select, SelectEvent, SelectState},
};
use gpui_kit::*;

pub enum BodyEvent {
    Changed,
}

impl EventEmitter<BodyEvent> for Body {}

struct BodyType {
    label: &'static str,
    language: &'static str,
}

const BODY_TYPES: [BodyType; 3] = [
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

pub struct Body {
    body: Entity<EditorState>,
    body_type: Entity<SelectState<Vec<String>>>,
}

impl Body {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select_items: Vec<String> = BODY_TYPES.iter().map(|t| t.label.to_string()).collect();

        let selected = BODY_TYPES
            .iter()
            .position(|t| t.language == "json")
            .unwrap_or(0);

        let initial_language = BODY_TYPES[selected].language.to_string();

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
            EditorState::new(window, cx)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .language(&initial_language)
        });

        cx.subscribe_in(
            &body_type_state,
            window,
            move |this: &mut Self, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(label)) = event {
                    if let Some(body_type) = BODY_TYPES.iter().find(|t| t.label == label) {
                        this.body.update(cx, |editor, cx| {
                            let value = editor.value().to_string();
                            editor.set_highlighter(body_type.language, cx);
                            editor.set_value(value, window, cx);
                        });
                        cx.emit(BodyEvent::Changed);
                    }
                }
            },
        )
        .detach();

        cx.subscribe_in(&body, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(BodyEvent::Changed);
            }
        })
        .detach();

        Self {
            body,
            body_type: body_type_state,
        }
    }

    pub fn value(&self, cx: &App) -> String {
        self.body.read(cx).value().to_string()
    }

    pub fn body_type(&self, cx: &App) -> String {
        self.body_type
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "JSON".to_string())
    }

    pub fn load_from_json(
        &mut self,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(body) = data.get("body") else {
            return;
        };
        let body_type = body
            .get("body_type")
            .and_then(|v| v.as_str())
            .unwrap_or("JSON");
        let body_value = body.get("body").and_then(|v| v.as_str()).unwrap_or("");

        let row = BODY_TYPES
            .iter()
            .position(|t| t.label == body_type)
            .unwrap_or(1);
        let language = BODY_TYPES[row].language;

        self.body_type.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
        });
        self.body.update(cx, |editor, cx| {
            editor.set_highlighter(language, cx);
            editor.set_value(body_value, window, cx);
        });
    }
}

impl Render for Body {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(
                        Editor::new(&self.body)
                            .size_full()
                            .appearance(false)
                            .bordered(false),
                    ),
            )
            .into_any_element()
    }
}
