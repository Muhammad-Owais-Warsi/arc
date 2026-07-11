use crate::ApiClient;
use crate::helpers::{build_method_tag, next_id};
use crate::query_params::QueryParams;
use gpui::*;
use gpui_component::input::InputState;
use gpui_component::select::SelectState;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};

pub struct Tabs {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) method: Entity<SelectState<Vec<String>>>,
    pub(crate) url: Entity<InputState>,
    pub(crate) query_params: Vec<Entity<QueryParams>>,
    pub(crate) pending: bool,
}

pub fn add_tab(
    window: &mut Window,
    cx: &mut Context<ApiClient>,
    name: &str,
    method: String,
) -> Tabs {
    let id = next_id();
    let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
    let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .into_iter()
        .map(String::from)
        .collect();
    let selected_method = methods.iter().position(|m| *m == method).unwrap_or(0);
    let method = cx.new(|cx| {
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

    Tabs {
        id,
        name: name.into(),
        path: String::new(),
        method,
        url,
        query_params: vec![],
        pending: false,
    }
}

pub fn render_new_tab_button(_api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    div()
        // .border_l_1()
        .border_color(cx.theme().border)
        .h_full()
        .px_2()
        .items_center()
        .justify_center()
        .child(
            Button::new("add-tab")
                .ghost()
                .small()
                .icon(IconName::Plus)
                .on_click(cx.listener(|this: &mut ApiClient, _event, window, cx| {
                    let tab = add_tab(window, cx, "new", "GET".to_string());
                    this.active_tab = Some(tab.id);
                    this.tabs.push(tab);
                    cx.notify();
                })),
        )
}

pub fn render_tab(_api: &ApiClient, cx: &mut Context<ApiClient>, tab: &Tabs) -> Tab {
    let id = tab.id;
    let name = tab.name.clone();

    let method = tab
        .method
        .read(cx)
        .selected_value()
        .map(String::as_str)
        .unwrap_or("");

    Tab::default()
        .px_1()
        .prefix(div().mr_1().child(build_method_tag(method)))
        .label(name)
        .suffix(
            Button::new(("close-tab", id))
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .on_click(
                    cx.listener(move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                        this.tabs.retain(|t| t.id != id);
                        this.active_tab = this.tabs.last().map(|t| t.id);
                        cx.notify();
                    }),
                ),
        )
}

pub fn render_tab_bar(api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    let selected = api
        .active_tab
        .and_then(|id| api.tabs.iter().position(|t| t.id == id))
        .unwrap_or(0);

    let sidebar_collapsed = api.sidebar_collapsed;

    TabBar::new("tabs")
        .prefix(
            h_flex().px(px(8.)).child(
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
                if let Some(tab) = this.tabs.get(*idx) {
                    this.active_tab = Some(tab.id);
                    cx.notify();
                }
            }),
        )
        .track_scroll(&api.scroll_handle)
        .suffix(render_new_tab_button(api, cx))
        .children(api.tabs.iter().map(|tab| render_tab(&api, cx, tab)))
}
