use crate::ApiClient;
use crate::headers::{Headers, headers_from_json, render_headers_section, render_response_headers};
use crate::helpers::{build_method_tag, next_id};
use crate::http;
use crate::project_panel::{ProjectPanel, ProjectPanelEvent};
use crate::query_params::{QueryParams, query_params_from_json, render_query_params_section};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::resizable::{resizable_panel, v_resizable};
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::tab::{self, Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Tabs {
    pub(crate) id: usize,
    pub(crate) node_id: usize,
    pub(crate) name: String,
    pub(crate) method: Entity<SelectState<Vec<String>>>,
    pub(crate) url: Entity<InputState>,
    pub(crate) query_params: Vec<Entity<QueryParams>>,
    pub(crate) headers: Vec<Entity<Headers>>,
    pub(crate) pending: bool,
    pub(crate) dirty: bool,
    pub(crate) selected_editor_config: usize,
    pub(crate) selected_response_panel_config: usize,
    pub(crate) response_body: Entity<InputState>,
    pub(crate) response_headers: Vec<(String, String)>,
    pub(crate) show_response_panel: bool,
}

pub(crate) struct TabManager {
    pub(crate) tabs: HashMap<usize, Entity<Tabs>>,
    pub(crate) active_tab_id: Option<usize>,
    pub(crate) scroll_handle: ScrollHandle,
    sidebar: Entity<ProjectPanel>, // for collapse toggle + method write-back
    theme: Entity<SelectState<Vec<SharedString>>>, // for the footer's theme selector
}

impl TabManager {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<ApiClient>,
        panel: Entity<ProjectPanel>,
        theme: Entity<SelectState<Vec<SharedString>>>,
    ) -> Entity<Self> {
        let tm = cx.new(|_| Self {
            tabs: HashMap::new(),
            active_tab_id: None,
            scroll_handle: ScrollHandle::new(),
            sidebar: panel.clone(),
            theme,
        });

        cx.subscribe_in(&panel, window, {
            let tm = tm.clone();
            move |_api, _, event, window, cx| {
                let ProjectPanelEvent::FileActivated {
                    node_id,
                    name,
                    path,
                    method,
                } = event;
                tm.update(cx, |this, cx| {
                    this.activate_tab(
                        *node_id,
                        name.clone(),
                        path.clone(),
                        method.clone(),
                        window,
                        cx,
                    );
                });
            }
        })
        .detach();

        tm
    }

    fn activate_tab(
        &mut self,
        node_id: usize,
        name: String,
        path: String,
        method: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.contains_key(&node_id) {
            self.active_tab_id = Some(node_id);
            cx.notify();
            return;
        }

        let tab = add_tab(window, cx, node_id, name, method, self.sidebar.clone());

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                    let url_entity = tab.read(cx).url.clone();
                    url_entity.update(cx, |i, cx| i.set_value(url.to_string(), window, cx));
                }
                let qp = query_params_from_json(window, cx, tab.clone(), &value);
                let hd = headers_from_json(window, cx, tab.clone(), &value);
                tab.update(cx, |t, _| {
                    t.query_params = qp;
                    t.headers = hd;
                });
            }
        }

        self.tabs.insert(node_id, tab);
        self.active_tab_id = Some(node_id);
        cx.notify();
    }

    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tab) = self.active_tab_id.and_then(|id| self.tabs.get(&id)) else {
            return div().child("No tab open");
        };
        let tab = tab.clone();
        let tab_state = tab.read(cx);
        let method = tab_state.method.clone();
        let url = tab_state.url.clone();
        let is_dirty = tab_state.dirty;
        let response_body = tab_state.response_body.clone();
        let pending = tab_state.pending;

        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&method)))
            .child(div().flex_1().child(Input::new(&url)))
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(is_dirty, |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    }),
            )
            .child(
                Button::new("send")
                    .primary()
                    .icon(IconName::Network)
                    .label("Send")
                    .disabled(pending)
                    .loading(pending)
                    .on_click({
                        let url = url.clone();
                        let response_body = response_body.clone();
                        let tab = tab.clone();
                        cx.listener(move |_this: &mut TabManager, _, _window, cx| {
                            let url_str = url.read(cx).value().to_string();
                            let method_str = method
                                .read(cx)
                                .selected_value()
                                .unwrap_or(&"GET".to_string())
                                .clone();

                            let (query_params, headers) = tab.update(cx, |tab, cx| {
                                tab.show_response_panel = true;
                                tab.pending = true;
                                let qp = tab
                                    .query_params
                                    .iter()
                                    .filter(|qp| qp.read(cx).active)
                                    .map(|qp| {
                                        let s = qp.read(cx);
                                        (
                                            s.key.read(cx).value().to_string(),
                                            s.value.read(cx).value().to_string(),
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                let hd = tab
                                    .headers
                                    .iter()
                                    .filter(|h| h.read(cx).active)
                                    .map(|h| {
                                        let s = h.read(cx);
                                        (
                                            s.key.read(cx).value().to_string(),
                                            s.value.read(cx).value().to_string(),
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                cx.notify();
                                (qp, hd)
                            });

                            let response_body = response_body.clone();
                            let tab_for_spawn = tab.clone();

                            cx.spawn(async move |this, cx| {
                                let result = http::send_request(
                                    &url_str,
                                    &method_str,
                                    query_params,
                                    headers,
                                )
                                .await;
                                let _ = this.update_in(cx, |_this, window, cx| {
                                    response_body.update(cx, |state, cx| match result {
                                        Ok((body, resp_headers)) => {
                                            let formatted =
                                                serde_json::from_str::<serde_json::Value>(&body)
                                                    .ok()
                                                    .and_then(|v| {
                                                        serde_json::to_string_pretty(&v).ok()
                                                    })
                                                    .unwrap_or(body);
                                            state.set_value(formatted, window, cx);
                                            tab_for_spawn.update(cx, |tab, cx| {
                                                tab.response_headers = resp_headers;
                                                tab.pending = false;
                                                cx.notify();
                                            });
                                        }
                                        Err(err) => {
                                            state.set_value(format!("Error: {err}"), window, cx)
                                        }
                                    });
                                    cx.notify();
                                });
                            })
                            .detach();
                        })
                    }),
            )
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active_tab = !self.tabs.is_empty();

        div()
            .flex_none()
            .h(px(50.0))
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .flex()
            .items_center()
            .px(px(16.0))
            .child(if is_active_tab {
                h_flex().gap(rems(0.5)).child(
                    Button::new("toggle-response")
                        .ghost()
                        .small()
                        .icon(IconName::PanelBottom)
                        .tooltip("Response")
                        .on_click(cx.listener(|this: &mut TabManager, _, _window, cx| {
                            if let Some(tab) = this.active_tab_id.and_then(|id| this.tabs.get(&id))
                            {
                                tab.update(cx, |tab, _| {
                                    tab.show_response_panel = !tab.show_response_panel
                                });
                            }
                            cx.notify();
                        })),
                )
            } else {
                div()
            })
            .child(div().flex_1())
            .child(
                div()
                    .w(px(140.0))
                    .child(Select::new(&self.theme).appearance(false)),
            )
    }
}

pub fn add_tab(
    window: &mut Window,
    cx: &mut Context<TabManager>,
    node_id: usize,
    name: String,
    method: String,
    _sidebar: Entity<ProjectPanel>,
) -> Entity<Tabs> {
    let id = next_id();
    let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
    let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .into_iter()
        .map(String::from)
        .collect();
    let selected_method = methods.iter().position(|m| *m == method).unwrap_or(0);
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
    let response_body_state = cx.new(|cx| {
        InputState::new(window, cx)
            .code_editor("json")
            .line_number(true)
            .default_value("")
    });

    let tab = Tabs {
        id,
        node_id,
        name,
        method: method_state.clone(),
        url: url.clone(),
        query_params: vec![],
        headers: vec![],
        pending: false,
        dirty: false,
        selected_editor_config: 0,
        selected_response_panel_config: 0,
        response_body: response_body_state,
        response_headers: vec![],
        show_response_panel: false,
    };
    let tab_entity = cx.new(|_| tab);

    let url_tab_clone = tab_entity.clone();
    cx.subscribe_in(
        &url,
        window,
        move |_this: &mut TabManager, _, event, _window, cx| {
            if let InputEvent::Change = event {
                url_tab_clone.update(cx, |tab, cx| {
                    tab.dirty = true;
                    cx.notify();
                })
            }
        },
    )
    .detach();

    let method_tab_clone = tab_entity.clone();
    cx.subscribe_in(
        &method_state,
        window,
        move |this: &mut TabManager, _, event, _window, cx| {
            if let SelectEvent::Confirm(Some(new_method)) = event {
                let new_method = new_method.clone();
                let node_id = method_tab_clone.read(cx).node_id;
                let sidebar = this.sidebar.clone();
                method_tab_clone.update(cx, |tab, cx| {
                    tab.dirty = true;
                    cx.notify();
                });
                sidebar.update(cx, |s, _| s.set_node_method(node_id, &new_method));
            }
        },
    )
    .detach();

    tab_entity
}

pub fn render_editor_config(tm: &mut TabManager, cx: &mut Context<TabManager>) -> impl IntoElement {
    let selected = tm
        .active_tab_id
        .and_then(|id| tm.tabs.get(&id))
        .map(|tab| tab.read(cx).selected_editor_config)
        .unwrap_or(0);
    div()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div().px(px(24.)).child(
                TabBar::new("request-tabs")
                    .w_full()
                    .with_variant(tab::TabVariant::Underline)
                    .selected_index(selected)
                    .child(Tab::new().label("Params"))
                    .child(Tab::new().label("Authorization"))
                    .child(Tab::new().label("Headers"))
                    .child(Tab::new().label("Body"))
                    .child(Tab::new().label("Settings"))
                    .on_click(cx.listener(
                        move |this: &mut TabManager, idx: &usize, _window, cx| {
                            if let Some(tab) = this.active_tab_id.and_then(|id| this.tabs.get(&id))
                            {
                                tab.update(cx, |tab, _| tab.selected_editor_config = *idx);
                            }
                            cx.notify();
                        },
                    )),
            ),
        )
}

pub fn render_new_tab_button(_tm: &TabManager, cx: &mut Context<TabManager>) -> impl IntoElement {
    h_flex()
        .h_full()
        .items_center()
        .justify_center()
        .px_2()
        .child(
            Button::new("add-tab")
                .ghost()
                .xsmall()
                .icon(IconName::Plus)
                .tooltip("Add Tab")
                .on_click(cx.listener(|this: &mut TabManager, _event, window, cx| {
                    let tab_key = next_id();
                    let sidebar = this.sidebar.clone();
                    let tab = add_tab(
                        window,
                        cx,
                        tab_key,
                        "Untitled".into(),
                        "GET".into(),
                        sidebar,
                    );
                    this.tabs.insert(tab_key, tab);
                    this.active_tab_id = Some(tab_key);
                    cx.notify();
                })),
        )
}

pub fn render_tab(
    _tm: &TabManager,
    cx: &mut Context<TabManager>,
    node_id: usize,
    tab: &Entity<Tabs>,
) -> Tab {
    let tab_state = tab.read(cx);
    let tab_id = tab_state.id;
    let node_name = tab_state.name.clone();
    let method = tab_state
        .method
        .read(cx)
        .selected_value()
        .map(String::as_str)
        .unwrap_or("");
    let close_node_id = node_id;

    Tab::default()
        .px_1()
        .prefix(div().mr_1().child(build_method_tag(method)))
        .label(node_name)
        .suffix(
            Button::new(("close-tab", tab_id))
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .on_click(cx.listener(
                    move |this: &mut TabManager, _: &ClickEvent, _window, cx| {
                        this.tabs.remove(&close_node_id);
                        this.active_tab_id = this.tabs.keys().next().copied();
                        cx.notify();
                    },
                )),
        )
}

pub fn render_tab_bar(tm: &TabManager, cx: &mut Context<TabManager>) -> impl IntoElement {
    let tab_ids: Vec<usize> = tm.tabs.keys().copied().collect();
    let selected = tm
        .active_tab_id
        .and_then(|id| tab_ids.iter().position(|&k| k == id))
        .unwrap_or(0);
    let sidebar_collapsed = tm.sidebar.read(cx).collapsed();

    TabBar::new("tabs")
        .min_h(px(32.))
        .prefix(
            h_flex().px(px(8.)).items_center().child(
                SidebarToggleButton::new()
                    .collapsed(sidebar_collapsed)
                    .on_click(cx.listener(|this: &mut TabManager, _, _window, cx| {
                        this.sidebar.update(cx, |s, _| s.toggle_collapsed());
                        cx.notify();
                    })),
            ),
        )
        .selected_index(selected)
        .on_click(
            cx.listener(move |this: &mut TabManager, idx: &usize, _window, cx| {
                let tab_ids: Vec<usize> = this.tabs.keys().copied().collect();
                if let Some(&id) = tab_ids.get(*idx) {
                    this.active_tab_id = Some(id);
                    cx.notify();
                }
            }),
        )
        .track_scroll(&tm.scroll_handle)
        .suffix(render_new_tab_button(tm, cx))
        .children(
            tm.tabs
                .iter()
                .map(|(&node_id, tab)| render_tab(tm, cx, node_id, tab)),
        )
}

impl Render for TabManager {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_tab = self.active_tab_id.is_some();
        let show_response = self
            .active_tab_id
            .and_then(|id| self.tabs.get(&id))
            .map(|t| t.read(cx).show_response_panel)
            .unwrap_or(false);

        let main_content = if has_tab {
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
                        .child(self.render_editor(cx)),
                )
                .child(render_editor_config(self, cx))
                .child(
                    div().flex_1().overflow_y_scrollbar().px(px(24.)).child(
                        match self
                            .active_tab_id
                            .and_then(|id| self.tabs.get(&id))
                            .map(|tab| tab.read(cx).selected_editor_config)
                            .unwrap_or(0)
                        {
                            0 => render_query_params_section(self, cx).into_any_element(),
                            2 => render_headers_section(self, cx).into_any_element(),
                            _ => div().into_any_element(),
                        },
                    ),
                );

            if show_response {
                let tab = self.active_tab_id.and_then(|id| self.tabs.get(&id));

                let selected_response_config = tab
                    .as_ref()
                    .map(|t| t.read(cx).selected_response_panel_config)
                    .unwrap_or(0);

                let response_content = div()
                    .id("response-panel-vscroll")
                    .h_full()
                    .overflow_y_scrollbar()
                    .min_h(px(0.))
                    .v_flex()
                    .border_t_1()
                    .overflow_hidden()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .child(
                        h_flex()
                            .w_full()
                            .flex_none()
                            .px(px(24.))
                            .py_2()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(div().text_sm().font_semibold().child("Response"))
                            .child(
                                Button::new("close-response")
                                    .ghost()
                                    .tooltip("Close Response")
                                    .small()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(
                                        |this: &mut TabManager, _, _window, cx| {
                                            if let Some(tab) =
                                                this.active_tab_id.and_then(|id| this.tabs.get(&id))
                                            {
                                                tab.update(cx, |tab, _cx| {
                                                    tab.show_response_panel = false;
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                            ),
                    )
                    .child(
                        div().px(px(24.)).flex_none().child(
                            TabBar::new("response-config")
                                .w_full()
                                .with_variant(tab::TabVariant::Underline)
                                .selected_index(selected_response_config)
                                .on_click(cx.listener(
                                    move |this: &mut TabManager, idx: &usize, _window, cx| {
                                        if let Some(tab) =
                                            this.active_tab_id.and_then(|id| this.tabs.get(&id))
                                        {
                                            tab.update(cx, |tab, _cx| {
                                                tab.selected_response_panel_config = *idx;
                                            });
                                        }
                                        cx.notify();
                                    },
                                ))
                                .child(Tab::new().label("Body"))
                                .child(Tab::new().label("Headers")),
                        ),
                    )
                    .child(match selected_response_config {
                        0 => {
                            let response_body_state = tab.unwrap().read(cx).response_body.clone();
                            div()
                                .id("response-body-hscroll")
                                .flex_1()
                                .min_h(px(0.))
                                .min_w(px(0.))
                                .overflow_hidden()
                                .px(px(24.))
                                .child(
                                    Input::new(&response_body_state)
                                        .flex_1()
                                        .h_full()
                                        .appearance(false),
                                )
                                .into_any_element()
                        }
                        1 => {
                            let response_headers = tab.unwrap().read(cx).response_headers.clone();
                            div()
                                .flex_1()
                                .min_h(px(0.))
                                .min_w(px(0.))
                                .px(px(24.))
                                .child(render_response_headers(response_headers, cx))
                                .into_any_element()
                        }
                        _ => div().child("issue").into_any_element(),
                    });
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
                            .child(response_content),
                    )
                    .into_any_element()
            } else {
                editor_content.into_any_element()
            }
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No tab open")
                .into_any_element()
        };

        div()
            .flex_1()
            .h_full()
            .min_h(px(0.))
            .v_flex()
            .child(
                div()
                    .flex_none()
                    .overflow_x_hidden()
                    .child(render_tab_bar(self, cx)),
            )
            .child(div().flex_1().min_h(px(0.)).child(main_content))
            .child(self.render_footer(cx))
    }
}
