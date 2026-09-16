use crate::dock::tabs::Playground;
use crate::helpers::render_method_tag;
use crate::http_request::HttpRequest;
use crate::playground::RequestPlayground;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::{
    IndexPath,
    input::{EditorState, TabSize},
    select::{SelectEvent, SelectState},
};
use gpui_kit::*;

struct CodeLang {
    label: &'static str,
    language: &'static str,
}

const CODE_LANGS: [CodeLang; 5] = [
    CodeLang {
        label: "cURL",
        language: "bash",
    },
    CodeLang {
        label: "Fetch",
        language: "javascript",
    },
    CodeLang {
        label: "Axios",
        language: "javascript",
    },
    CodeLang {
        label: "Python",
        language: "python",
    },
    CodeLang {
        label: "Rust",
        language: "rust",
    },
];

pub struct CodeScreen {
    focus: FocusHandle,
    codes: Vec<String>,
    selected: usize,
    lang_select: Entity<SelectState<Vec<String>>>,
    editor: Entity<EditorState>,
}

impl CodeScreen {
    pub fn new(
        window: &mut Window,
        source: Option<WeakEntity<RequestPlayground>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let req = source
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|v| HttpRequest::from_file_content(&v.read(cx).current_content(cx)))
            .unwrap_or_else(|| HttpRequest::new("GET", ""));
        return Self::new_with_request(req, window, cx);
    }

    pub fn new_with_request(req: HttpRequest, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let codes = Self::generate_all(&req);

        let items: Vec<String> = CODE_LANGS.iter().map(|l| l.label.to_string()).collect();
        let lang_select = cx.new(|cx| {
            SelectState::new(
                items,
                Some(IndexPath {
                    section: 0,
                    row: 0,
                    column: 0,
                }),
                window,
                cx,
            )
        });
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .language(CODE_LANGS[0].language)
                .default_value(&codes[0])
        });

        cx.subscribe_in(
            &lang_select,
            window,
            move |this: &mut Self, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(label)) = event {
                    if let Some(row) = CODE_LANGS.iter().position(|l| l.label == label) {
                        this.selected = row;
                        this.editor.update(cx, |editor, cx| {
                            editor.set_highlighter(CODE_LANGS[row].language, cx);
                            editor.set_value(this.codes[row].clone(), window, cx);
                        });
                        cx.notify();
                    }
                }
            },
        )
        .detach();

        Self {
            focus: cx.focus_handle(),
            codes,
            selected: 0,
            lang_select,
            editor,
        }
    }

    pub fn current_code(&self) -> String {
        self.codes.get(self.selected).cloned().unwrap_or_default()
    }

    pub fn lang_picker(&self) -> Entity<SelectState<Vec<String>>> {
        self.lang_select.clone()
    }

    pub fn code_editor(&self) -> Entity<EditorState> {
        self.editor.clone()
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

impl Playground for CodeScreen {
    fn tab_label(&self, _cx: &App) -> SharedString {
        "Code".into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(render_method_tag("CODE").into_any_element())
    }
}

