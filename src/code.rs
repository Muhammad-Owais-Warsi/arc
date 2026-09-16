use crate::dock::tabs::CenterTab;
use crate::helpers::render_method_tag;
use crate::http_request::HttpRequest;
use crate::http_response::AuthPayload;
use crate::icons::IconName;
use crate::request_playground::RequestPlayground;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    ActiveTheme, IndexPath, StyledExt, h_flex,
    input::{Editor, EditorState, TabSize},
    select::{Select, SelectEvent, SelectState},
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

    pub fn new_with_request(
        req: HttpRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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

    fn generate_all(req: &HttpRequest) -> Vec<String> {
        vec![
            Self::to_curl(req),
            Self::to_fetch(req),
            Self::to_axios(req),
            Self::to_python(req),
            Self::to_rust(req),
        ]
    }

    fn url_encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                b' ' => out.push_str("%20"),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }

    fn full_url(req: &HttpRequest) -> String {
        let base = if req.url.is_empty() {
            "http://localhost".to_string()
        } else {
            req.url.clone()
        };
        if req.query_params.is_empty() {
            return base;
        }
        let sep = if base.contains('?') { '&' } else { '?' };
        let qs = req
            .query_params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::url_encode(k), Self::url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{base}{sep}{qs}")
    }

    fn is_json(body: &str) -> bool {
        matches!(body.trim_start().chars().next(), Some('{') | Some('['))
    }

    fn esc_shell(s: &str) -> String {
        s.replace('\'', "'\\''")
    }

    fn esc_dq(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }

    fn esc_rust(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }

    fn to_curl(req: &HttpRequest) -> String {
        let mut parts = vec![format!(
            "curl -X {} \"{}\"",
            req.method,
            Self::esc_dq(&Self::full_url(req))
        )];
        for (k, v) in &req.headers {
            parts.push(format!("-H \"{}: {}\"", Self::esc_dq(k), Self::esc_dq(v)));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                parts.push(format!(
                    "-u \"{}:{}\"",
                    Self::esc_dq(username),
                    Self::esc_dq(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                parts.push(format!(
                    "-H \"Authorization: Bearer {}\"",
                    Self::esc_dq(token)
                ));
            }
        }
        if !req.body.is_empty() {
            parts.push(format!("--data-raw '{}'", Self::esc_shell(&req.body)));
        }
        parts.join(" \\\n  ")
    }

    fn fetch_headers(req: &HttpRequest) -> Vec<String> {
        let mut lines = Vec::new();
        for (k, v) in &req.headers {
            lines.push(format!(
                "    \"{}\": \"{}\",",
                Self::esc_dq(k),
                Self::esc_dq(v)
            ));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                lines.push(format!(
                    "    \"Authorization\": \"Basic \" + btoa(\"{}:{}\"),",
                    Self::esc_dq(username),
                    Self::esc_dq(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                lines.push(format!(
                    "    \"Authorization\": \"Bearer {}\",",
                    Self::esc_dq(token)
                ));
            }
        }
        lines
    }

    fn to_fetch(req: &HttpRequest) -> String {
        let mut out = format!(
            "fetch(\"{}\", {{\n  method: \"{}\",",
            Self::esc_dq(&Self::full_url(req)),
            req.method
        );
        let headers = Self::fetch_headers(req);
        if !headers.is_empty() {
            out.push_str("\n  headers: {\n");
            out.push_str(&headers.join("\n"));
            out.push_str("\n  },");
        }
        if !req.body.is_empty() {
            out.push_str(&format!("\n  body: \"{}\",", Self::esc_dq(&req.body)));
        }
        out.push_str("\n});");
        out
    }

    fn to_axios(req: &HttpRequest) -> String {
        let mut out = format!(
            "axios({{\n  method: \"{}\",\n  url: \"{}\"",
            req.method.to_lowercase(),
            Self::esc_dq(&Self::full_url(req))
        );
        let headers = Self::fetch_headers(req);
        if !headers.is_empty() {
            out.push_str(",\n  headers: {\n");
            out.push_str(&headers.join("\n"));
            out.push_str("\n  }");
        }
        if let AuthPayload::Basic { username, password } = &req.auth {
            out.push_str(&format!(
                ",\n  auth: {{\n    username: \"{}\",\n    password: \"{}\",\n  }}",
                Self::esc_dq(username),
                Self::esc_dq(password)
            ));
        }
        if !req.body.is_empty() {
            if Self::is_json(&req.body) {
                out.push_str(&format!(",\n  data: {}", req.body.trim()));
            } else {
                out.push_str(&format!(",\n  data: \"{}\"", Self::esc_dq(&req.body)));
            }
        }
        out.push_str("\n});");
        out
    }

    fn to_python(req: &HttpRequest) -> String {
        let mut out = String::from("import requests\n\n");
        out.push_str(&format!(
            "url = \"{}\"\n",
            Self::esc_dq(&Self::full_url(req))
        ));
        if req.headers.is_empty() && !matches!(req.auth, AuthPayload::Bearer { .. }) {
            out.push_str("headers = {}\n");
        } else {
            out.push_str("headers = {\n");
            for (k, v) in &req.headers {
                out.push_str(&format!(
                    "    \"{}\": \"{}\",\n",
                    Self::esc_dq(k),
                    Self::esc_dq(v)
                ));
            }
            if let AuthPayload::Bearer { token } = &req.auth {
                out.push_str(&format!(
                    "    \"Authorization\": \"Bearer {}\",\n",
                    Self::esc_dq(token)
                ));
            }
            out.push_str("}\n");
        }
        let mut call = format!(
            "response = requests.request(\"{}\", url, headers=headers",
            req.method
        );
        if !req.body.is_empty() {
            if Self::is_json(&req.body) {
                out.push_str(&format!("payload = {}\n", req.body.trim()));
                call.push_str(", json=payload");
            } else {
                out.push_str(&format!("data = \"{}\"\n", Self::esc_dq(&req.body)));
                call.push_str(", data=data");
            }
        }
        if let AuthPayload::Basic { username, password } = &req.auth {
            call.push_str(&format!(
                ", auth=(\"{}\", \"{}\")",
                Self::esc_dq(username),
                Self::esc_dq(password)
            ));
        }
        call.push_str(")");
        out.push_str(&call);
        out.push_str("\nprint(response.status_code)\nprint(response.text)\n");
        out
    }

    fn to_rust(req: &HttpRequest) -> String {
        let url = Self::esc_rust(&Self::full_url(req));
        let start = match req.method.to_uppercase().as_str() {
            "GET" => format!("client.get(\"{url}\")"),
            "POST" => format!("client.post(\"{url}\")"),
            "PUT" => format!("client.put(\"{url}\")"),
            "PATCH" => format!("client.patch(\"{url}\")"),
            "DELETE" => format!("client.delete(\"{url}\")"),
            "HEAD" => format!("client.head(\"{url}\")"),
            _ => format!(
                "client.request(reqwest::Method::from_bytes(b\"{}\").unwrap(), \"{url}\")",
                req.method.to_uppercase()
            ),
        };
        let mut out = String::from("let client = reqwest::blocking::Client::new();\n");
        out.push_str("let res = ");
        out.push_str(&start);
        out.push('\n');
        for (k, v) in &req.headers {
            out.push_str(&format!(
                "    .header(\"{}\", \"{}\")\n",
                Self::esc_rust(k),
                Self::esc_rust(v)
            ));
        }
        match &req.auth {
            AuthPayload::None => {}
            AuthPayload::Basic { username, password } => {
                out.push_str(&format!(
                    "    .basic_auth(\"{}\", Some(\"{}\"))\n",
                    Self::esc_rust(username),
                    Self::esc_rust(password)
                ));
            }
            AuthPayload::Bearer { token } => {
                out.push_str(&format!(
                    "    .bearer_auth(\"{}\")\n",
                    Self::esc_rust(token)
                ));
            }
        }
        if !req.body.is_empty() {
            out.push_str(&format!("    .body(\"{}\")\n", Self::esc_rust(&req.body)));
        }
        out.push_str("    .send()?;\nprintln!(\"{}\", res.text()?);\n");
        out
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
        let current = self.codes.get(self.selected).cloned().unwrap_or_default();
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
                        .child(div().w(px(140.)).child(Select::new(&self.lang_select)))
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
                                Editor::new(&self.editor)
                                    .w_full()
                                    .h_full()
                                    .appearance(false)
                                    .readonly(true),
                            ),
                    ),
            )
    }
}
