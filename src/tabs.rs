use crate::ApiClient;
use crate::helpers::build_method_tag;
use gpui::*;
use gpui_component::input::InputEvent;
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};

#[derive(Clone)]
pub struct Tabs {
    pub(crate) id: usize,
    pub(crate) node_id: usize,
    pub(crate) method: Entity<SelectState<Vec<String>>>,
    pub(crate) url: Entity<InputState>,
    pub(crate) query_params: Vec<Entity<QueryParams>>,
    pub(crate) pending: bool,
    pub(crate) dirty: bool,
    pub(crate) selected_editor_config: usize,
    pub(crate) response_panel: Entity<InputState>,
    pub(crate) show_response_panel: bool,
}

pub fn add_tab(
    window: &mut Window,
    cx: &mut Context<ApiClient>,
    node_id: usize,
    method: String,
) -> Entity<Tabs> {
    let id = next_id();
    let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
    let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .into_iter()
        .map(String::from)
        .collect();
    let method = cx.new(|cx| {
        SelectState::new(
            methods,
            Some(IndexPath {
                section: 0,
                row: 0,
                column: 0,
            }),
            window,
            cx,
        )
    });
    let response_panel = cx.new(|cx| {
        gpui_component::input::InputState::new(window, cx)
            .code_editor("json")
            .line_number(true)
            .default_value("")
    });

    let tab = Tabs {
        id,
        node_id,
        method: method.clone(),
        url: url.clone(),
        query_params: vec![],
        pending: false,
        dirty: false,
        selected_editor_config: 0,
        response_panel: response_panel_state,
        show_response_panel: false,
    };

    let tab_entity = cx.new(|_cx| tab);

    let url_tab_clone = tab_entity.clone();
    cx.subscribe_in(
        &url,
        window,
        move |_this: &mut ApiClient, _, event, _window, cx| {
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
        &method,
        window,
        move |this: &mut ApiClient, _, event, _window, cx| {
            if let SelectEvent::Confirm(Some(new_method)) = event {
                let new_method = new_method.clone();

                method_tab_clone.update(cx, |tab, cx| {
                    tab.dirty = true;

                    let node_id = tab.node_id;
                    for ws in &mut this.workspaces {
                        if update_node_method(&mut ws.nodes, node_id, &new_method) {
                            break;
                        }
                    }

                    cx.notify();
                });
            }
        },
    })
}

pub fn render_editor_config(api: &mut ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    let selected = api
        .active_tab_id
        .and_then(|id| api.tabs.get(&id))
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
                        move |this: &mut ApiClient, idx: &usize, _window, cx| {
                            if let Some(tab) = this.active_tab_id.and_then(|id| this.tabs.get(&id))
                            {
                                tab.update(cx, |node, _cx| {
                                    node.set_selected_editor_config(*idx);
                                });
                            }
                            cx.notify();
                        },
                    )),
            ),
        )
}

pub fn render_new_tab_button(_api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
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
                .on_click(cx.listener(|this: &mut ApiClient, _event, window, cx| {
                    let tab_key = next_id();
                    let tab = add_tab(window, cx, tab_key, "GET".to_string());
                    this.tabs.insert(tab_key, tab);
                    this.active_tab_id = Some(tab_key);
                    cx.notify();
                })),
        )
}

pub fn render_tab(api: &ApiClient, cx: &mut Context<ApiClient>, node_id: usize, tab: &Entity<Tabs>) -> Tab {
    let tab_state = tab.read(cx);
    let tab_id = tab_state.id;

    let node_name = api
        .workspaces
        .iter()
        .find_map(|ws| ws.nodes.get(&node_id))
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "Untitled".to_string());

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
                .on_click(
                    cx.listener(move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                        this.tabs.remove(&close_node_id);
                        this.active_tab_id = this.tabs.keys().next().copied();
                        cx.notify();
                    }),
                ),
        )
}

pub fn render_tab_bar(api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    let tab_ids: Vec<usize> = api.tabs.keys().copied().collect();
    let selected = api
        .active_tab_id
        .and_then(|id| tab_ids.iter().position(|&k| k == id))
        .unwrap_or(0);

    let sidebar_collapsed = api.sidebar_collapsed;

    TabBar::new("tabs")
        .min_h(px(32.))
        .prefix(
            h_flex().px(px(8.)).items_center().child(
                SidebarToggleButton::new()
                    .collapsed(sidebar_collapsed)
                    .on_click(cx.listener(move |this: &mut ApiClient, _, _window, cx| {
                        this.sidebar_collapsed = !this.sidebar_collapsed;
                        cx.notify();
                    })),
            ),
        )
        .selected_index(selected)
        .on_click(
            cx.listener(move |this: &mut ApiClient, idx: &usize, _window, cx| {
                let tab_ids: Vec<usize> = this.tabs.keys().copied().collect();
                if let Some(&id) = tab_ids.get(*idx) {
                    this.active_tab_id = Some(id);
                    cx.notify();
                }
            }),
        )
        .track_scroll(&api.scroll_handle)
        .suffix(render_new_tab_button(api, cx))
        .children(api.tabs.iter().map(|(&node_id, tab)| {
            render_tab(api, cx, node_id, tab)
        }))
}
