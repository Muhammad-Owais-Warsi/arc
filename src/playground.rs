use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{ActiveTheme, h_flex};
use gpui_component::{Disableable, IconName};
use gpui_component::{
    IndexPath, Sizable, StyledExt,
    input::{Input, InputEvent, InputState},
    resizable::{resizable_panel, v_resizable},
    select::{Select, SelectEvent, SelectState},
    tab::{self, Tab, TabBar},
};

use crate::headers::Headers;
use crate::http;
use crate::query_params::QueryParams;
use crate::response_panel::ResponsePanel;

pub enum PlaygroundEvent {
    SendRequest,
    MethodChanged(String),
}

impl EventEmitter<PlaygroundEvent> for Playground {}

pub struct Playground {
    pub method: Entity<SelectState<Vec<String>>>,
    pub url: Entity<InputState>,
    pub query_params: Entity<QueryParams>,
    pub headers: Entity<Headers>,
    pub body: Entity<InputState>,
    pub selected_config: usize,
    pub pending: bool,
    pub dirty: bool,
    pub response_panel: Entity<ResponsePanel>,
}

impl Playground {
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
        let body = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("json")
        });
        let query_params = cx.new(|_| QueryParams::new());
        let headers = cx.new(|_| Headers::new());
        let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));

        let this = Self {
            method: method_state.clone(),
            url: url.clone(),
            query_params,
            headers,
            body,
            selected_config: 0,
            pending: false,
            dirty: false,
            response_panel,
        };

        cx.subscribe_in(
            &method_state,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let SelectEvent::Confirm(_) = event {
                    this.dirty = true;
                    let method = this.method(cx);
                    cx.emit(PlaygroundEvent::MethodChanged(method));
                }
            },
        )
        .detach();

        cx.subscribe_in(&url, window, |this: &mut Self, _, event, _window, cx| {
            if let InputEvent::Change = event {
                this.dirty = true;
                cx.notify();
            }
        })
        .detach();

        this
    }

    pub fn method(&self, cx: &App) -> String {
        self.method
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "GET".to_string())
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
    }

    pub fn send_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url_str = self.url.read(cx).value().to_string();
        let method_str = self
            .method
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or_else(|| "GET".to_string());
        let query_params = self.query_params.read(cx).active_params(cx);
        let headers = self.headers.read(cx).active_headers(cx);
        let response_panel = self.response_panel.clone();

        response_panel.update(cx, |panel, cx| panel.open(cx));

        let rp = response_panel;
        cx.spawn(async move |this, cx| {
            let result = http::send_request(&url_str, &method_str, query_params, headers).await;
            let _ = this.update_in(cx, |_this, window, cx| {
                match result {
                    Ok((body, resp_headers)) => {
                        let formatted = serde_json::from_str::<serde_json::Value>(&body)
                            .ok()
                            .and_then(|v| serde_json::to_string_pretty(&v).ok())
                            .unwrap_or(body);
                        rp.update(cx, |p, cx| {
                            p.set_response(formatted, resp_headers, window, cx);
                        });
                    }
                    Err(err) => {
                        rp.update(cx, |p, cx| {
                            p.set_response(format!("Error: {err}"), vec![], window, cx);
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
            .child(div().flex_1().child(Input::new(&self.url)))
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(self.dirty, |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    }),
            )
            .child(
                Button::new("send")
                    .primary()
                    .icon(IconName::Network)
                    .label("Send")
                    .disabled(self.pending)
                    .loading(self.pending)
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
                        .child(Tab::new().label("Settings"))
                        .on_click(cx.listener(|this: &mut Self, idx: &usize, _window, cx| {
                            this.selected_config = *idx;
                            cx.notify();
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_config_content(&self, _cx: &mut Context<Self>) -> AnyElement {
        match self.selected_config {
            0 => self.query_params.clone().into_any_element(),
            2 => self.headers.clone().into_any_element(),
            3 => self.render_body(),
            _ => div().into_any_element(),
        }
    }

    fn render_body(&self) -> AnyElement {
        div()
            .size_full()
            .v_flex()
            .child(
                div()
                    .flex_basis(DefiniteLength::Fraction(0.75))
                    .flex_grow(1.)
                    .min_h(px(0.))
                    .border_1()
                    .rounded_md()
                    .overflow_hidden()
                    .child(Input::new(&self.body).size_full().appearance(false)),
            )
            .child(
                div()
                    .flex_basis(DefiniteLength::Fraction(0.25))
                    .flex_grow(1.)
                    .min_h(px(0.)),
            )
            .into_any_element()
    }
}

impl Render for Playground {
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
