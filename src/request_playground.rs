use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::clipboard::Clipboard;
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{ActiveTheme, Disableable, StyledExt, h_flex};
use gpui_component::{
    IndexPath,
    input::{InputEvent, InputState},
    resizable::{resizable_panel, v_resizable},
    select::{Select, SelectEvent, SelectState},
    tab::{self, Tab, TabBar},
};

use crate::fs;
use crate::http::{self, AuthPayload, Response};
use crate::playground::Playground;
use crate::{
    auth::{Auth, AuthEvent, AuthType},
    body::{Body, BodyEvent},
    fs::{FileContent, KeyValue},
    headers::{Headers, HeadersEvent},
    icons::IconName,
    query_params::{QueryParams, QueryParamsEvent},
    response_panel::ResponsePanel,
};

pub struct RequestPlayground {
    path: Option<String>,
    method: Entity<SelectState<Vec<String>>>,
    url: Entity<InputState>,
    auth: Entity<Auth>,
    query_params: Entity<QueryParams>,
    headers: Entity<Headers>,
    body: Entity<Body>,
    selected_config: usize,
    pending: bool,
    dirty: bool,
    response_panel: Entity<ResponsePanel>,
    snapshot: FileContent,
}

pub enum RequestPlaygroundEvent {
    MethodChanged(String),
}

impl EventEmitter<RequestPlaygroundEvent> for RequestPlayground {}

impl Playground for RequestPlayground {
    fn method(&self, cx: &App) -> String {
        self.method(cx)
    }
    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        Some(self.respone_panel_entity())
    }
}

impl RequestPlayground {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));

        let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
            .into_iter()
            .map(String::from)
            .collect();
        let selected_method = methods.iter().position(|m| *m == "GET").unwrap_or(0);
        let method_state = cx.new(|cx| {
            SelectState::new(
                methods,
                Some(IndexPath {
                    section: 0,
                    row: selected_method,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        let query_params = cx.new(|_| QueryParams::new());
        let headers = cx.new(|_| Headers::new());
        let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
        let auth = cx.new(|cx| Auth::new(window, cx));
        let body = cx.new(|cx| Body::new(window, cx));

        let qp_for_sub = query_params.clone();
        let headers_for_sub = headers.clone();
        let body_for_sub = body.clone();
        let auth_for_sub = auth.clone();

        let mut this = Self {
            method: method_state.clone(),
            url: url.clone(),
            auth,
            query_params,
            headers,
            body,
            selected_config: 0,
            pending: false,
            dirty: false,
            response_panel,
            snapshot: FileContent::default(),
            path: None,
        };
        this.snapshot = this.current_content(cx);

        cx.subscribe_in(
            &method_state,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let SelectEvent::Confirm(_) = event {
                    let method = this.method(cx);
                    this.evaluate_dirty(cx);
                    cx.emit(RequestPlaygroundEvent::MethodChanged(method));
                }
            },
        )
        .detach();

        cx.subscribe_in(&url, window, |this: &mut Self, _, event, _window, cx| {
            if let InputEvent::Change = event {
                this.evaluate_dirty(cx);
            }
        })
        .detach();

        cx.subscribe_in(
            &qp_for_sub,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if matches!(event, QueryParamsEvent::Changed) {
                    this.evaluate_dirty(cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &headers_for_sub,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if matches!(event, HeadersEvent::Changed) {
                    this.evaluate_dirty(cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &body_for_sub,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if matches!(event, BodyEvent::Changed) {
                    this.evaluate_dirty(cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &auth_for_sub,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if matches!(event, AuthEvent::Changed) {
                    this.evaluate_dirty(cx);
                }
            },
        )
        .detach();

        this
    }

    pub fn mark_dirty(&mut self, status: bool) {
        self.dirty = status
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn method(&self, cx: &App) -> String {
        self.method
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "GET".to_string())
    }

    pub fn method_entity(&self) -> Entity<SelectState<Vec<String>>> {
        self.method.clone()
    }

    fn evaluate_dirty(&mut self, cx: &mut Context<Self>) {
        let current = self.current_content(cx);
        self.dirty = current != self.snapshot;
        cx.notify();
    }

    fn current_content(&self, cx: &App) -> FileContent {
        FileContent {
            name: self.snapshot.name.clone(),
            url: self.url.read(cx).value().to_string(),
            method: self.method(cx),
            params: self
                .query_params
                .read(cx)
                .rows(cx)
                .into_iter()
                .map(|(key, value, active)| KeyValue { key, value, active })
                .collect(),
            headers: self
                .headers
                .read(cx)
                .rows(cx)
                .into_iter()
                .map(|(key, value, active)| KeyValue { key, value, active })
                .collect(),
            auth: fs::Auth {
                auth_type: self.auth.read(cx).auth_type(),
                username: self.auth.read(cx).basic_auth_values(cx).0,
                password: self.auth.read(cx).basic_auth_values(cx).1,
                token: self.auth.read(cx).bearer_auth_value(cx),
            },
            body: fs::Body {
                body_type: self.body.read(cx).body_type(cx),
                body: self.body.read(cx).value(cx),
            },
        }
    }

    pub fn respone_panel_entity(&self) -> Entity<ResponsePanel> {
        self.response_panel.clone()
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Some(path);
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            eprintln!("Cannot save: no file path for this tab");
            return;
        };

        let current = self.current_content(cx);
        match fs::write_request_file(std::path::Path::new(&path), &current) {
            Ok(()) => {
                self.snapshot = current;
                self.dirty = false;
                cx.notify();
            }
            Err(err) => eprintln!("Failed to save request: {err}"),
        }
    }

    pub fn load(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        content: &serde_json::Value,
    ) {
        if let Some(url) = content.get("url").and_then(|v| v.as_str()) {
            self.url.update(cx, |s, cx| s.set_value(url, window, cx));
        }
        if let Some(method) = content.get("method").and_then(|v| v.as_str()) {
            let methods: Vec<String> =
                vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                    .into_iter()
                    .map(String::from)
                    .collect();
            let row = methods.iter().position(|m| *m == method).unwrap_or(0);

            self.method.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
            });
        }
        self.query_params
            .update(cx, |qp, cx| qp.load_from_json(content, window, cx));
        self.headers
            .update(cx, |h, cx| h.load_from_json(content, window, cx));
        self.auth
            .update(cx, |a, cx| a.load_from_json(content, window, cx));
        self.body
            .update(cx, |b, cx| b.load_from_json(content, window, cx));
        self.snapshot = self.current_content(cx);
        self.dirty = false;
    }

    pub fn send_request(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let url_str = self.url.read(cx).value().to_string();
        let method_str = self
            .method
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "GET".to_string());
        let query_params = self.query_params.read(cx).active_params(cx);
        let headers = self.headers.read(cx).active_headers(cx);
        let body = self.body.read(cx).value(cx);

        let auth = match self.auth.read(cx).auth_type() {
            AuthType::None => AuthPayload::None,
            AuthType::Basic => {
                let (u, p) = self.auth.read(cx).basic_auth_values(cx);
                AuthPayload::Basic {
                    username: u,
                    password: p,
                }
            }
            AuthType::Bearer => AuthPayload::Bearer {
                token: self.auth.read(cx).bearer_auth_value(cx),
            },
        };

        let response_panel = self.response_panel.clone();

        response_panel.update(cx, |panel, cx| panel.open(cx));

        let rp = response_panel;
        cx.spawn(async move |this, cx| {
            let result = http::HttpRequest::new()
                .url(&url_str)
                .method(&method_str)
                .headers(headers)
                .queries(query_params)
                .body(&body)
                .auth(auth)
                .send()
                .await;
            let _ = this.update_in(cx, |_this, window, cx| {
                match result {
                    Ok(response) => {
                        rp.update(cx, |p, cx| {
                            p.set_response(response, window, cx);
                        });
                    }
                    Err(err) => {
                        rp.update(cx, |p, cx| {
                            p.set_response(
                                Response {
                                    status_code: 0,
                                    status_text: "Error".to_string(),
                                    headers: http::ResponseHeaders {
                                        headers: vec![],
                                        response_size: 0,
                                    },
                                    body: http::ResponseBody {
                                        body: format!("Error: {err}"),
                                        response_size: 0,
                                    },
                                    cookies: vec![],
                                    request: http::RequestStats {
                                        header_size: 0,
                                        body_size: 0,
                                        size: 0,
                                    },
                                    response_size: 0,
                                    duration: std::time::Duration::ZERO,
                                },
                                window,
                                cx,
                            );
                        });
                    }
                }
                _this.pending = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn render_editor_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&self.method)))
            .child(
                div().flex_1().child(
                    Input::new(&self.url).suffix(
                        Clipboard::new("url-clip")
                            .tooltip("Copy")
                            .value(self.url.read(cx).value()),
                    ),
                ),
            )
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(self.dirty, |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    })
                    .on_click(cx.listener(|this: &mut Self, _, _window, cx| {
                        this.save(cx);
                    })),
            )
            .child(
                Button::new("send")
                    .primary()
                    .icon(IconName::Send)
                    .label("Send")
                    .disabled(self.pending)
                    .loading(self.pending)
                    .loading_icon(IconName::Spinner)
                    .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                        this.pending = true;
                        this.send_request(window, cx);
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn render_config_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div().px(px(24.)).child(
                    TabBar::new("request-tabs")
                        .w_full()
                        .with_variant(tab::TabVariant::Underline)
                        .selected_index(self.selected_config)
                        .child(Tab::new().label("Params"))
                        .child(Tab::new().label("Authorization"))
                        .child(Tab::new().label("Headers"))
                        .child(Tab::new().label("Body"))
                        .on_click(cx.listener(|this: &mut Self, idx: &usize, _window, cx| {
                            this.selected_config = *idx;
                            cx.notify();
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_config_content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.selected_config {
            0 => div()
                .size_full()
                .child(self.query_params.clone())
                .into_any_element(),
            1 => div()
                .size_full()
                .child(self.auth.clone())
                .into_any_element(),
            2 => div()
                .size_full()
                .child(self.headers.clone())
                .into_any_element(),
            3 => div()
                .size_full()
                .child(self.body.clone())
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }
}

impl Render for RequestPlayground {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_response = self.response_panel.read(cx).is_shown();

        let editor_content = div()
            .size_full()
            .min_h(px(0.))
            .v_flex()
            .gap(px(16.))
            .child(
                div()
                    .flex_none()
                    .v_flex()
                    .px(px(24.))
                    .pt(rems(1.0))
                    .child(self.render_editor_bar(cx)),
            )
            .child(self.render_config_tabs(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .px(px(24.))
                    .child(self.render_config_content(cx)),
            );

        if show_response {
            v_resizable("editor-response-split")
                .child(
                    resizable_panel()
                        .size(px(500.))
                        .size_range(px(200.)..px(4000.))
                        .child(editor_content),
                )
                .child(
                    resizable_panel()
                        .size(px(280.))
                        .size_range(px(100.)..px(600.))
                        .child(self.response_panel.clone()),
                )
                .into_any_element()
        } else {
            editor_content.into_any_element()
        }
    }
}
