use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::{
    IndexPath,
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectState},
};
use gpui_kit::*;

use super::actions::CopyURL;
use crate::actions::CopyAsCode;
use crate::auth::{Auth, AuthEvent};
use crate::body::{Body, BodyEvent};
use crate::curl::curl::CurlRequest;
use crate::dock::tabs::Playground;
use crate::fs;
use crate::fs::request::{Auth as AuthContent, Body as BodyContent, KeyValue, RequestFileContent};
use crate::headers::{Headers, HeadersEvent};
use crate::ui::method_tag;
use crate::query_params::{QueryParams, QueryParamsEvent};
use crate::response::ResponsePanel;
use crate::settings_panel::AppSettings;
use crate::toast::{ToastRoot, ToastVariant};

pub struct RequestPlayground {
    path: Option<String>,
    tab_name: String,
    method: Entity<SelectState<Vec<String>>>,
    url: Entity<InputState>,
    auth: Entity<Auth>,
    query_params: Entity<QueryParams>,
    headers: Entity<Headers>,
    body: Entity<Body>,
    selected_config: usize,
    pending: Option<tokio::task::AbortHandle>,
    dirty: bool,
    response_panel: Entity<ResponsePanel>,
    snapshot: RequestFileContent,
    focus: FocusHandle,
}

pub enum RequestPlaygroundEvent {
    MethodChanged(String),
    ResponsePanelOpened,
    CopyAsCode,
}

impl EventEmitter<RequestPlaygroundEvent> for RequestPlayground {}

impl Panel for RequestPlayground {
    fn panel_name(&self) -> &'static str {
        "request"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for RequestPlayground {}

impl Focusable for RequestPlayground {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Playground for RequestPlayground {
    fn tab_label(&self, _cx: &App) -> SharedString {
        self.tab_name.clone().into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(method_tag(&self.method(cx)).into_any_element())
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
            pending: None,
            dirty: false,
            response_panel,
            snapshot: RequestFileContent::default(),
            path: None,
            tab_name: "Untitled".to_string(),
            focus: cx.focus_handle(),
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

        cx.subscribe_in(&url, window, |this: &mut Self, _, event, window, cx| {
            if let InputEvent::Change = event {
                let pasted_content = this.url.read(cx).value();
                if CurlRequest::is_curl_command(&pasted_content) {
                    match CurlRequest::parse(&pasted_content) {
                        Ok(parsed_content) => {
                            this.load(window, cx, &parsed_content);
                            this.evaluate_dirty(cx);
                            ToastRoot::show(
                                cx,
                                ToastVariant::Success,
                                "Imported cURL request successfully".into(),
                            );
                        }
                        Err(_e) => {
                            ToastRoot::show(cx, ToastVariant::Error, "cURL import failed".into());
                        }
                    }
                }
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

    pub fn path(&self) -> Option<String> {
        self.path.clone()
    }

    pub fn mark_dirty(&mut self, status: bool) {
        self.dirty = status
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn is_sending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn selected_tab(&self) -> usize {
        self.selected_config
    }

    pub fn select_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected_config = ix;
        cx.notify();
    }

    pub fn method_input(&self) -> Entity<SelectState<Vec<String>>> {
        self.method.clone()
    }

    pub fn url_input(&self) -> Entity<InputState> {
        self.url.clone()
    }

    pub fn auth_view(&self) -> Entity<Auth> {
        self.auth.clone()
    }

    pub fn query_view(&self) -> Entity<QueryParams> {
        self.query_params.clone()
    }

    pub fn headers_view(&self) -> Entity<Headers> {
        self.headers.clone()
    }

    pub fn body_view(&self) -> Entity<Body> {
        self.body.clone()
    }

    pub fn response_panel(&self) -> Entity<ResponsePanel> {
        self.response_panel.clone()
    }

    pub fn set_pending(&mut self, handle: tokio::task::AbortHandle) {
        self.pending = Some(handle);
    }

    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    pub fn method(&self, cx: &App) -> String {
        self.method
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "GET".to_string())
    }

    pub fn set_method(&mut self, method: &str, window: &mut Window, cx: &mut Context<Self>) {
        let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
            .into_iter()
            .map(String::from)
            .collect();
        let row = methods.iter().position(|m| m == method).unwrap_or(0);
        self.method.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
        });
    }

    fn evaluate_dirty(&mut self, cx: &mut Context<Self>) {
        let current = self.current_content(cx);
        self.dirty = current != self.snapshot;
        cx.notify();
    }

    pub fn current_content(&self, cx: &App) -> RequestFileContent {
        let (auth_type, username, password, token) = self.auth.read(cx).credentials(cx);
        RequestFileContent {
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
            auth: AuthContent {
                auth_type,
                username,
                password,
                token,
            },
            body: BodyContent {
                body_type: self.body.read(cx).body_type(cx),
                body: self.body.read(cx).value(cx),
            },
        }
    }

    pub fn set_response_panel(&mut self, panel: Entity<ResponsePanel>) {
        self.response_panel = panel;
    }

    pub fn set_tab_name(&mut self, name: String, cx: &mut Context<Self>) {
        if self.tab_name != name {
            self.tab_name = name;
            cx.notify();
        }
    }

    /// Full rename sync: tab label + file path + saved snapshot name, so a
    /// later save (or save-on-close) targets the new file with the new name
    /// instead of resurrecting the old one.
    pub fn rename_file(&mut self, name: String, path: String, cx: &mut Context<Self>) {
        let clean_name = name.strip_suffix(".json").unwrap_or(&name).to_string();
        self.tab_name = clean_name.clone();
        self.snapshot.name = clean_name;
        self.path = Some(path);
        cx.notify();
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Some(path);
    }

    pub fn stored_method(&self, cx: &App) -> String {
        if AppSettings::global(cx)
            .playground
            .request_playground
            .save_on_close
        {
            self.method(cx)
        } else {
            self.snapshot.method.clone()
        }
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            eprintln!("Cannot save: no file path for this tab");
            return;
        };

        let current = self.current_content(cx);
        match fs::request::write(std::path::Path::new(&path), &current) {
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
            self.set_method(method, window, cx);
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

    pub fn handle_copy_url(
        &mut self,
        _: &CopyURL,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.url.read(cx).value().into()));
    }

    pub fn handle_copy_as_code(
        &mut self,
        _: &CopyAsCode,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        _cx.emit(RequestPlaygroundEvent::CopyAsCode);
    }

    pub fn cancel_pending(&mut self) {
        if let Some(abort) = self.pending.take() {
            abort.abort();
        }
    }
}
