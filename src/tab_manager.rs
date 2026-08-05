use crate::fs::read_request_file;
use crate::helpers::next_id;
use crate::playground::{Playground, PlaygroundEvent};
use crate::tab::{TabEvent, Tabs};
use gpui::*;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};

use crate::icons::IconName;
use std::collections::HashMap;
use std::path::Path;

pub enum TabManagerEvent {
    MethodChanged(usize, String),
    SidebarToggle(bool),
    // TabClosed(usize),
    TabActivated(Option<usize>),
}

impl EventEmitter<TabManagerEvent> for TabManager {}

pub struct TabManager {
    tabs: HashMap<usize, Entity<Tabs>>,
    active_tab_id: Option<usize>,
    scroll_handle: ScrollHandle,
    sidebar_collapsed: bool,
}

impl TabManager {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            tabs: HashMap::new(),
            active_tab_id: None,
            scroll_handle: ScrollHandle::new(),
            sidebar_collapsed: false,
        }
    }

    pub fn activate_tab(
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

        let tab = self.add_tab(window, cx, node_id, name, method);

        let request = read_request_file(Path::new(&path));
        tab.update(cx, |t, cx| {
            t.playground()
                .update(cx, |pg, cx| pg.load(window, cx, &request));
        });

        self.tabs.insert(node_id, tab);
        self.active_tab_id = Some(node_id);
        cx.notify();
    }

    pub fn rename_tab(&mut self, node_id: usize, new_name: String, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.get(&node_id) {
            tab.update(cx, |t, _| t.update_name(new_name));
            cx.notify();
        }
    }

    pub fn has_tabs(&self) -> bool {
        self.active_tab_id.is_some()
    }

    pub fn active_playground(&self, cx: &App) -> Option<Entity<Playground>> {
        self.active_tab_id
            .and_then(|id| self.tabs.get(&id))
            .map(|tab| tab.read(cx).playground())
    }

    fn add_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        node_id: usize,
        name: String,
        method: String,
    ) -> Entity<Tabs> {
        let id = next_id();
        let playground = cx.new(|cx| Playground::new(window, cx));

        if method != "GET" {
            let methods: Vec<String> =
                vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                    .into_iter()
                    .map(String::from)
                    .collect();
            let row = methods.iter().position(|m| *m == method).unwrap_or(0);
            playground.update(cx, |pg, cx| {
                pg.method_entity().update(cx, |state, cx| {
                    state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
                })
            });
        }

        let tab_entity = cx.new(|cx| Tabs::new(id, node_id, name, playground.clone()));

        let pg = playground.clone();
        cx.subscribe_in(&pg, window, move |_, _, event, _window, cx| {
            if let PlaygroundEvent::MethodChanged(method) = event {
                cx.emit(TabManagerEvent::MethodChanged(node_id, method.clone()));
            }
        })
        .detach();

        cx.subscribe_in(
            &tab_entity,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let TabEvent::Close(node_id) = event {
                    this.tabs.remove(&node_id);
                    this.active_tab_id = this
                        .tabs
                        .keys()
                        .copied()
                        .filter(|id| *id != *node_id) // safe even though it's already removed; harmless no-op
                        .max();
                    cx.emit(TabManagerEvent::TabActivated(this.active_tab_id));
                    cx.notify();
                }
            },
        )
        .detach();

        tab_entity
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let tab_ids: Vec<usize> = self.tabs.keys().copied().collect();
        let selected = self
            .active_tab_id
            .and_then(|id| tab_ids.iter().position(|&k| k == id))
            .unwrap_or(0);

        let tabs: Vec<Tab> = self
            .tabs
            .iter()
            .map(|(_, tab)| tab.update(cx, |this, cx| this.to_tab_element(cx)))
            .collect();

        TabBar::new("tabs")
            .h(px(32.))
            .with_size(gpui_component::Size::Large)
            .prefix(
                h_flex().px(px(8.)).items_center().child(
                    SidebarToggleButton::new()
                        .collapsed(self.sidebar_collapsed)
                        .on_click(cx.listener(|this: &mut Self, _, _window, cx| {
                            this.sidebar_collapsed = !this.sidebar_collapsed;
                            cx.emit(TabManagerEvent::SidebarToggle(this.sidebar_collapsed));
                            cx.notify();
                        })),
                ),
            )
            .selected_index(selected)
            .on_click(
                cx.listener(move |this: &mut Self, idx: &usize, _window, cx| {
                    let tab_ids: Vec<usize> = this.tabs.keys().copied().collect();
                    if let Some(&id) = tab_ids.get(*idx) {
                        this.active_tab_id = Some(id);
                        cx.emit(TabManagerEvent::TabActivated(Some(id)));
                        cx.notify();
                    }
                }),
            )
            .track_scroll(&self.scroll_handle)
            .suffix(self.render_new_tab_button(cx))
            .children(tabs)
            .into_any_element()
    }

    fn render_new_tab_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .on_click(cx.listener(|this: &mut Self, _event, window, cx| {
                        let tab_key = next_id();
                        let tab =
                            this.add_tab(window, cx, tab_key, "Untitled".into(), "GET".into());
                        this.tabs.insert(tab_key, tab);
                        this.active_tab_id = Some(tab_key);
                        cx.notify();
                    })),
            )
    }
}

impl Render for TabManager {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_tab = self.active_tab_id.is_some();

        let main_content = if has_tab {
            self.active_tab_id
                .and_then(|id| self.tabs.get(&id))
                .map(|tab| tab.read(cx).playground().clone().into_any_element())
                .unwrap_or_else(|| div().into_any_element())
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
                    .child(self.render_tab_bar(cx)),
            )
            .child(div().flex_1().min_h(px(0.)).child(main_content))
    }
}
