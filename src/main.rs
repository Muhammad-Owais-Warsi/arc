mod helpers;
mod query_params;
mod tabs;
use crate::helpers::build_method_tag;
use crate::tabs::{Tabs, add_tab, render_tab_bar};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::Theme;
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{button::*, *};
use std::path::PathBuf;

#[derive(Clone)]
struct Node {
    path: String,
    name: String,
    method: String,
    children: Vec<Node>,
    is_file: bool,
}

pub(crate) struct ApiClient {
    pub(crate) nodes: Vec<Node>,
    pub(crate) tabs: Vec<Tabs>,
    pub(crate) active_tab: Option<usize>,
    pub(crate) scroll_handle: ScrollHandle,
    pub(crate) theme: Entity<SelectState<Vec<SharedString>>>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) selected_editor_config: usize,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>, default_theme: SharedString) -> Self {
        let nodes = vec![Node {
            path: "/api".into(),
            name: "API Client".into(),
            method: String::new(),
            is_file: false,
            children: vec![
                Node {
                    path: "/api/get_users".into(),
                    name: "Get Users".into(),
                    method: "GET".into(),
                    is_file: true,
                    children: vec![],
                },
                Node {
                    path: "/api/user".into(),
                    name: "Create User".into(),
                    method: "POST".into(),
                    is_file: false,
                    children: vec![Node {
                        path: "/api/user/create".into(),
                        name: "Create User".into(),
                        method: "GET".into(),
                        is_file: true,
                        children: vec![],
                    }],
                },
                Node {
                    path: "/api/update_user".into(),
                    name: "Update User".into(),
                    method: "PUT".into(),
                    is_file: true,
                    children: vec![],
                },
            ],
        }];

        let themes: Vec<SharedString> =
            ThemeRegistry::global(cx).themes().keys().cloned().collect();

        let default_theme_idx = themes.iter().position(|t| *t == default_theme).unwrap_or(0);

        let theme = cx.new(|cx| {
            SelectState::new(
                themes,
                Some(IndexPath {
                    section: 0,
                    row: default_theme_idx,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        cx.subscribe_in(&theme, window, |_, _, event, _window, cx| {
            if let SelectEvent::Confirm(Some(name)) = event {
                let registry = ThemeRegistry::global(cx);
                if let Some(theme_config) = registry.themes().get(name).cloned() {
                    let mode = theme_config.mode;
                    let theme = Theme::global_mut(cx);
                    if mode.is_dark() {
                        theme.dark_theme = theme_config;
                    } else {
                        theme.light_theme = theme_config;
                    }

                    Theme::change(mode, None, cx);
                    cx.refresh_windows();
                }
            }
        })
        .detach();
        let mut this = Self {
            nodes,
            tabs: Vec::new(),
            active_tab: None,
            scroll_handle: ScrollHandle::new(),
            theme,
            sidebar_collapsed: false,
            selected_editor_config: 0,
        };

        let tab = add_tab(window, cx, "get_req", "GET".to_string());
        this.active_tab = Some(tab.id);
        this.tabs.push(tab);

        this
    }

    fn render_node(&self, node: &Node, cx: &mut Context<Self>) -> SidebarMenuItem {
        let is_file = node.is_file;
        let name = node.name.clone();
        let method = node.method.clone();

        let method_for_suffix = method.clone();
        let method_for_click = method.clone();
        let name_for_click = name.clone();

        let mut item = SidebarMenuItem::new(name.clone()).suffix(move |_, _| {
            if is_file {
                div().child(build_method_tag(method_for_suffix.as_str()))
            } else {
                div()
            }
        });

        if is_file {
            item = item.on_click(
                cx.listener(move |this: &mut ApiClient, _event, window, cx| {
                    let tab = add_tab(window, cx, &name_for_click, method_for_click.clone());
                    this.active_tab = Some(tab.id);
                    this.tabs.push(tab);
                    cx.notify();
                }),
            );
        }

        if node.children.is_empty() {
            item
        } else {
            let mut children = Vec::new();

            for child in &node.children {
                children.push(self.render_node(child, cx));
            }

            item.children(children)
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_collapsed = self.sidebar_collapsed;

        Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.sidebar_collapsed)
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap(rems(0.75))
                        .child(IconName::Palette)
                        .when(!icon_collapsed, |this| {
                            this.child(div().flex_1().child("workspace"))
                        }),
                ),
            )
            .child(
                SidebarGroup::new("Explorer").child(
                    SidebarMenu::new().children(
                        self.nodes
                            .iter()
                            .map(|child| Self::render_node(&self, child, cx)),
                    ),
                ),
            )
    }
    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .h(px(50.0))
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .flex()
            .items_center()
            .px(px(16.))
            .child(
                h_flex()
                    .w_full()
                    .gap(rems(0.5))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Ready"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .w(px(140.))
                            .child(Select::new(&self.theme).appearance(false)),
                    ),
            )
    }

    fn render_editor_config(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div().w_full().child(
            TabBar::new("request-tabs")
                // .gap_10()
                .with_variant(tab::TabVariant::Underline)
                .selected_index(self.selected_editor_config)
                .child(Tab::new().label("Params"))
                .child(Tab::new().label("Authorization"))
                .child(Tab::new().label("Headers"))
                .child(Tab::new().label("Body"))
                .child(Tab::new().label("Settings"))
                .on_click(
                    cx.listener(move |this: &mut ApiClient, idx: &usize, _window, cx| {
                        this.selected_editor_config = *idx
                    }),
                ),
        )
    }

    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tab) = self
            .active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
        else {
            return div().child("No tab open");
        };

        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&tab.method)))
            .child(div().flex_1().child(Input::new(&tab.url)))
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(tab.dirty, |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    }),
            )
            .child(Button::new("send").primary().label("Send"))
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .v_flex()
                    .child(
                        div()
                            .flex_none()
                            .overflow_x_hidden()
                            .child(render_tab_bar(self, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .v_flex()
                            .gap(rems(1.))
                            .px(px(24.))
                            .pt(rems(1.))
                            .child(self.render_editor(cx))
                            .child(self.render_editor_config(cx))
                            .child(match self.selected_editor_config {
                                0 => query_params::render_query_params_section(self, cx)
                                    .into_any_element(),
                                _ => div().into_any_element(),
                            }),
                    )
                    .child(self.render_footer(cx)),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let theme_name = SharedString::from("Ayu Dark");
        let default_theme = theme_name.clone();
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }

        if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                Theme::global_mut(cx).apply_config(&theme);
            }
        }) {
            eprintln!("Failed to watch themes directory: {}", err);
        }

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|view_cx| ApiClient::new(window, view_cx, default_theme));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
