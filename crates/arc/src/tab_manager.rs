use crate::footer::{Footer, FooterEvent};
use crate::helpers::next_id;
use crate::playground::{Playground, PlaygroundEvent};
use crate::tab::{TabEvent, Tabs};
use gpui::*;
use ui::tab::{Tab, TabBar};
use ui::{ActiveTheme as _, button::*, *};

use crate::icons::IconName;
use std::collections::HashMap;

pub enum TabManagerEvent {
    MethodChanged(usize, String),
    ResponseToggle,
    TabActivated(Option<usize>),
}

impl EventEmitter<TabManagerEvent> for TabManager {}

pub struct TabManager {
    tabs: HashMap<usize, Entity<Tabs>>,
    active_tab_id: Option<usize>,
    scroll_handle: ScrollHandle,
    footer: Entity<Footer>,
}

impl TabManager {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let footer = cx.new(|cx| Footer::new(window, cx));

        let tm = Self {
            tabs: HashMap::new(),
            active_tab_id: None,
            scroll_handle: ScrollHandle::new(),
            footer: footer.clone(),
        };

        let footer_clone = footer.clone();
        cx.subscribe_in(&footer_clone, window, |_, _, event, _window, cx| {
            let FooterEvent::ToggleResponse = event;
            cx.emit(TabManagerEvent::ResponseToggle);
        })
        .detach();

        tm
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

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                tab.update(cx, |t, cx| {
                    t.playground()
                        .update(cx, |pg, cx| pg.load(window, cx, &value));
                });
            }
        }

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

        let tab_entity = cx.new(|_cx| Tabs::new(id, node_id, name, playground.clone()));

        let pg = playground.clone();
        cx.subscribe_in(&pg, window, move |_, _, event, _window, cx| {
            let PlaygroundEvent::MethodChanged(method) = event;
            cx.emit(TabManagerEvent::MethodChanged(node_id, method.clone()));
        })
        .detach();

        cx.subscribe_in(
            &tab_entity,
            window,
            |this: &mut Self, _, event, _window, cx| {
                let TabEvent::Close(node_id) = event;
                this.tabs.remove(node_id);
                let next = this.tabs.keys().next().copied();
                this.active_tab_id = next;
                cx.emit(TabManagerEvent::TabActivated(next));
                cx.notify();
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
            .h(px(40.))
            .large()
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

    fn render_footer(&self, has_tabs: bool, cx: &mut Context<Self>) -> AnyElement {
        self.footer
            .update(cx, |f, cx| f.set_show_toggle(has_tabs, cx));
        self.footer.clone().into_any_element()
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
            .child(self.render_footer(has_tab, cx))
    }
}
