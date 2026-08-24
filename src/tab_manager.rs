use crate::env_panel::{EnvPanel, EnvPanelEvent};
use crate::env_playground::{EnvPlayground, EnvPlaygroundEvent};
use crate::helpers::next_id;
use crate::playground::PlaygroundHandle;
use crate::project_panel::{ProjectPanel, ProjectPanelEvent};
use crate::request_fs::RequestFileSystem;
use crate::request_playground::{RequestPlayground, RequestPlaygroundEvent};
use crate::settings_panel::AppSettings;
use crate::stress_testing::StressTesting;
use crate::tab::{TabEvent, Tabs};
use crate::welcome::WelcomeScreen;
use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};

use crate::icons::IconName;
use indexmap::IndexMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

const WELCOME_NODE_ID: usize = usize::MAX - 2;

pub struct TabManager {
    project_panel: Entity<ProjectPanel>,
    env_panel: Entity<EnvPanel>,
    tabs: IndexMap<usize, Entity<Tabs>>,
    active_tab_id: Option<usize>,
    scroll_handle: ScrollHandle,
}

impl TabManager {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        project_panel: Entity<ProjectPanel>,
        env_panel: Entity<EnvPanel>,
    ) -> Self {
        cx.subscribe_in(
            &project_panel,
            window,
            |this: &mut Self, _, event, window, cx| match event {
                ProjectPanelEvent::FileActivated {
                    node_id,
                    name,
                    path,
                    method,
                } => {
                    this.activate_request_tab(
                        *node_id,
                        name.clone(),
                        path.clone(),
                        method.clone(),
                        window,
                        cx,
                    );
                }
                ProjectPanelEvent::FileRenamed { node_id, new_name } => {
                    this.rename_tab(*node_id, new_name.clone(), cx);
                }
                ProjectPanelEvent::FileDeleted { node_id, .. }
                | ProjectPanelEvent::FileTrashed { node_id, .. } => {
                    let pp = this.project_panel.clone();
                    this.close_tab(*node_id, &pp, cx);
                }
                ProjectPanelEvent::StressTestPlayground { path, node_name } => {
                    this.add_stress_test_tab(window, cx, path.clone(), node_name.clone());
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &env_panel,
            window,
            |this: &mut Self, _, event, window, cx| match event {
                EnvPanelEvent::EnvActivated { name } => {
                    this.open_env_tab(name.clone(), window, cx);
                }
            },
        )
        .detach();

        Self {
            project_panel,
            env_panel,
            tabs: IndexMap::new(),
            active_tab_id: None,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn reset(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.clear();
        self.active_tab_id = None;
        self.scroll_handle = ScrollHandle::new();

        cx.notify();
    }

    pub fn activate_request_tab(
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

        let (playground, tab) = self.add_request_tab(window, cx, node_id, name.clone(), method);
        let request = RequestFileSystem::read_request(Path::new(&path));
        playground.update(cx, |pg, cx| pg.load(window, cx, &request));
        playground.update(cx, |pg, cx| pg.set_path(path.clone()));

        // let tab = self.add_tab(window, cx, node_id, name.clone(), Box::new(playground));

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

    pub fn close_tab(
        &mut self,
        node_id: usize,
        project_panel: &Entity<ProjectPanel>,
        cx: &mut Context<Self>,
    ) {
        self.tabs.shift_remove(&node_id);
        self.activate_neighbor(node_id, project_panel, cx);
        cx.notify();
    }

    fn neighbor_of(&self, closed_id: usize) -> Option<usize> {
        match self.tabs.get_index_of(&closed_id) {
            Some(ix) if ix > 0 => self.tabs.get_index(ix - 1).map(|(id, _)| *id),
            Some(_) => self.tabs.get_index(0).map(|(id, _)| *id),
            None => self.tabs.last().map(|(id, _)| *id),
        }
    }

    fn activate_neighbor(
        &mut self,
        closed_id: usize,
        project_panel: &Entity<ProjectPanel>,
        cx: &mut Context<Self>,
    ) {
        self.active_tab_id = self.neighbor_of(closed_id);
        let active = self.active_tab_id;
        project_panel.update(cx, |pp, cx| {
            pp.set_active_node(active);
            cx.notify();
        });
    }

    pub fn has_tabs(&self) -> bool {
        self.active_tab_id.is_some()
    }

    pub fn env_names(&self, cx: &App) -> Vec<String> {
        self.env_panel.read(cx).envs.clone()
    }

    pub fn active_playground(&self, cx: &App) -> Option<Box<dyn PlaygroundHandle>> {
        self.active_tab_id
            .and_then(|id| self.tabs.get(&id))
            .map(|tab| tab.read(cx).playground())
    }

    pub fn toggle_active_response(&mut self, cx: &mut Context<Self>) {
        if let Some(content) = self.active_playground(cx) {
            if let Some(panel) = content.response_panel(cx) {
                panel.update(cx, |panel, cx| panel.toggle(cx));
            }
        }
    }

    pub fn add_stress_test_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        path: String,
        node_name: String,
    ) {
        let tab_key = next_id();

        let source = self
            .active_playground(cx)
            .and_then(|content| content.entity().downcast::<RequestPlayground>().ok())
            .map(|pg| pg.downgrade());

        let stress_test_playground = cx.new(|cx| StressTesting::new(source, path, window, cx));

        let content: Box<dyn PlaygroundHandle> = stress_test_playground.clone_box();
        let tab_entity = cx.new(|cx| Tabs::new(tab_key, tab_key, node_name, content));

        cx.subscribe_in(
            &tab_entity,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let TabEvent::Close(node_id) = event {
                    this.tabs.shift_remove(node_id);
                    let pp = this.project_panel.clone();
                    this.activate_neighbor(*node_id, &pp, cx);
                    cx.notify();
                }
            },
        )
        .detach();

        self.tabs.insert(tab_key, tab_entity);
        self.active_tab_id = Some(tab_key);
        cx.notify();
    }

    pub fn open_welcome_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        welcome: Entity<WelcomeScreen>,
    ) {
        let node_id = WELCOME_NODE_ID;
        if self.tabs.contains_key(&node_id) {
            self.active_tab_id = Some(node_id);
            cx.notify();
            return;
        }

        let content: Box<dyn PlaygroundHandle> = welcome.clone_box();
        let tab_entity =
            cx.new(|cx| Tabs::new(WELCOME_NODE_ID, WELCOME_NODE_ID, "Welcome".into(), content));

        cx.subscribe_in(
            &tab_entity,
            window,
            move |this: &mut Self, _, event, _window, cx| {
                if let TabEvent::Close(node_id) = event {
                    this.tabs.shift_remove(node_id);
                    let pp = this.project_panel.clone();
                    this.activate_neighbor(*node_id, &pp, cx);
                    cx.notify();
                }
            },
        )
        .detach();

        self.tabs.insert(node_id, tab_entity);
        self.active_tab_id = Some(node_id);
        cx.notify();
    }

    fn env_name_to_id(name: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish() as usize
    }

    pub fn open_env_tab(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let node_id = Self::env_name_to_id(&name);

        if self.tabs.contains_key(&node_id) {
            self.active_tab_id = Some(node_id);
            cx.notify();
            return;
        }

        let playground = cx.new(|cx| EnvPlayground::new(name.clone(), window, cx));
        let content: Box<dyn PlaygroundHandle> = playground.clone_box();
        let tab_entity = cx.new(|cx| Tabs::new(node_id, node_id, name.clone(), content));

        cx.subscribe_in(
            &playground,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let EnvPlaygroundEvent::Deleted { name } = event {
                    let node_id = Self::env_name_to_id(name);
                    this.tabs.shift_remove(&node_id);
                    if this.active_tab_id == Some(node_id) {
                        this.active_tab_id = this.tabs.keys().next().copied();
                    }
                    this.env_panel.update(cx, |panel, cx| panel.refresh(cx));
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &tab_entity,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let TabEvent::Close(node_id) = event {
                    this.tabs.shift_remove(node_id);
                    let pp = this.project_panel.clone();
                    this.activate_neighbor(*node_id, &pp, cx);
                    cx.notify();
                }
            },
        )
        .detach();

        self.tabs.insert(node_id, tab_entity);
        self.active_tab_id = Some(node_id);
        cx.notify();
    }

    fn add_request_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        node_id: usize,
        name: String,
        method: String,
    ) -> (Entity<RequestPlayground>, Entity<Tabs>) {
        let id = next_id();
        let playground = cx.new(|cx| RequestPlayground::new(window, cx));

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

        let tab_entity = cx.new(|cx| Tabs::new(id, node_id, name, playground.clone_box()));

        let pg = playground.clone();
        cx.subscribe_in(
            &pg,
            window,
            move |this: &mut Self, _, event, _window, cx| {
                if let RequestPlaygroundEvent::MethodChanged(method) = event {
                    this.project_panel
                        .update(cx, |pp, _| pp.set_node_method(node_id, method));
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &tab_entity,
            window,
            move |this: &mut Self, _, event, _window, cx| {
                if let TabEvent::Close(node_id) = event {
                    let save_on_close = AppSettings::global(cx)
                        .playground
                        .request_playground
                        .save_on_close;

                    if save_on_close {
                        let content = playground.read(cx).current_content(cx);
                        if let Some(path) = playground.read(cx).path() {
                            RequestFileSystem::write(Path::new(&path), &content).ok();
                        }
                    }

                    let method = playground.read(cx).stored_method(cx);
                    this.project_panel.update(cx, |pp, _| {
                        pp.set_node_method(*node_id, &method);
                    });

                    this.tabs.shift_remove(node_id);
                    let pp = this.project_panel.clone();
                    this.activate_neighbor(*node_id, &pp, cx);
                    cx.notify();
                }
            },
        )
        .detach();

        (pg, tab_entity)
    }

    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
            .prefix(h_flex().px(px(8.)).items_center())
            .selected_index(selected)
            .on_click(
                cx.listener(move |this: &mut Self, idx: &usize, _window, cx| {
                    let tab_ids: Vec<usize> = this.tabs.keys().copied().collect();
                    if let Some(&id) = tab_ids.get(*idx) {
                        this.active_tab_id = Some(id);
                        let active = this.active_tab_id;
                        this.project_panel.update(cx, |pp, cx| {
                            pp.set_active_node(active);
                            cx.notify();
                        });
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
                        let (playground, tab) = this.add_request_tab(
                            window,
                            cx,
                            tab_key,
                            "Untitled".to_string(),
                            "GET".into(),
                        );
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
                .map(|tab| tab.read(cx).playground().render_into())
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
            .child(div().flex_1().v_flex().min_h(px(0.)).child(main_content))
    }
}
