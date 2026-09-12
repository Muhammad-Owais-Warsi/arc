use std::collections::HashMap;
use std::path::Path;

use gpui_kit::base::dock::{DockLayout, DockPlacement, InsertTarget, NodeId, PanelId};
use gpui_kit::component::IndexPath;
use gpui_kit::*;

use crate::dock::shell::DockShell;
use crate::dock::tabs::EmptyTab;
use crate::env_panel::EnvPanel;
use crate::env_playground::{EnvPlayground, EnvPlaygroundEvent};
use crate::fs;
use crate::helpers::next_id;
use crate::project_panel::ProjectPanel;
use crate::request_playground::{RequestPlayground, RequestPlaygroundEvent};
use crate::response_panel::ResponsePanel;
use crate::settings_panel::AppSettings;
use crate::stress_testing::StressTesting;
use crate::welcome::WelcomeScreen;

// ---------------------------------------------------------------------------
// TabManager: owns all center-tab state and logic.
//
// Follows Zed's Pane/Dock pattern: a real struct held as Entity<TabManager>
// by ApiClient, with its own impl block. It holds shared Entity<T> handles to
// shell, project_panel, env_panel, response, and welcome — the same objects
// ApiClient references — which is idiomatic GPUI (one underlying object,
// multiple cheap handles), not duplication.
//
// ApiClient holds one field: `tab_manager: Entity<TabManager>`.
// All tab state (request_tabs, env_tabs, welcome_panel, tab_nav) lives here
// and nowhere else.
// ---------------------------------------------------------------------------

struct RequestTabMeta {
    panel: PanelId,
    view: Entity<RequestPlayground>,
    path: Option<String>,
}

struct EnvTabMeta {
    panel: PanelId,
    view: Entity<EnvPlayground>,
}

pub struct TabManager {
    // Shared handles — one underlying object, referenced from here and ApiClient.
    shell: Entity<DockShell>,
    project_panel: Entity<ProjectPanel>,
    env_panel: Entity<EnvPanel>,
    response: Entity<ResponsePanel>,
    welcome: Entity<WelcomeScreen>,
    // Tab state owned exclusively by TabManager.
    request_tabs: HashMap<usize, RequestTabMeta>,
    env_tabs: Vec<EnvTabMeta>,
    welcome_panel: Option<PanelId>,
    // Browser-like nav history: every tab we open/activate is pushed;
    // back/forward walk the stack, skipping panels closed since.
    tab_nav: Vec<PanelId>,
    tab_nav_ix: usize,
}

impl TabManager {
    pub fn new(
        shell: Entity<DockShell>,
        project_panel: Entity<ProjectPanel>,
        env_panel: Entity<EnvPanel>,
        response: Entity<ResponsePanel>,
        welcome: Entity<WelcomeScreen>,
    ) -> Self {
        Self {
            shell,
            project_panel,
            env_panel,
            response,
            welcome,
            request_tabs: HashMap::new(),
            env_tabs: Vec::new(),
            welcome_panel: None,
            tab_nav: Vec::new(),
            tab_nav_ix: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Dock helpers
    // -----------------------------------------------------------------------

    fn dock_area(&self, cx: &App) -> Entity<gpui_kit::base::dock::DockArea> {
        self.shell.read(cx).area()
    }

    fn panel_alive(&self, panel: PanelId, cx: &App) -> bool {
        self.dock_area(cx).read(cx).panel(panel).is_some()
    }

    fn activate_dock_panel(&self, panel: PanelId, window: &mut Window, cx: &mut App) {
        // Resolve the panel's CURRENT slot and re-insert there so the strip
        // doesn't reshuffle on activation.
        let area = self.dock_area(cx);
        let slot = area
            .read(cx)
            .layout(DockPlacement::Center)
            .and_then(|tree| {
                let node = tree.find_panel_node(panel)?;
                let tabs = tree.find_node(node)?;
                let ix = match tabs.kind() {
                    gpui_kit::base::dock::PaneRef::Tabs { panels, .. } => {
                        panels.iter().position(|id| *id == panel)
                    }
                    _ => None,
                };
                Some((node, ix))
            });
        if let Some((node, ix)) = slot {
            self.shell.clone().update(cx, |shell, cx| {
                shell.move_panel(
                    panel,
                    InsertTarget::Tabs {
                        node,
                        ix,
                        activate: true,
                    },
                    window,
                    cx,
                );
            });
        }
    }

    fn add_center_panel<P: gpui_kit::base::dock::Panel>(
        &self,
        view: Entity<P>,
        target: Option<NodeId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PanelId {
        let pid = PanelId::from(view.entity_id());
        self.shell.clone().update(cx, |shell, cx| {
            shell.add_panel(view, DockPlacement::Center, window, cx);
        });
        if let Some(node) = target {
            self.shell.clone().update(cx, |shell, cx| {
                shell.move_panel(
                    pid,
                    InsertTarget::Tabs {
                        node,
                        ix: None,
                        activate: true,
                    },
                    window,
                    cx,
                );
            });
        }
        pid
    }

    fn remove_center_panel<P: gpui_kit::base::dock::Panel>(
        &self,
        view: Entity<P>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let area = self.dock_area(cx);
        area.update(cx, |area, cx| {
            area.remove_panel(view, window, cx);
        });
    }

    // -----------------------------------------------------------------------
    // Nav history
    // -----------------------------------------------------------------------

    fn nav_push(&mut self, panel: PanelId) {
        if self.tab_nav.last() == Some(&panel) {
            self.tab_nav_ix = self.tab_nav.len().saturating_sub(1);
            return;
        }
        self.tab_nav.truncate(self.tab_nav_ix + 1);
        self.tab_nav.push(panel);
        self.tab_nav_ix = self.tab_nav.len().saturating_sub(1);
    }

    fn drop_nav_panel(&mut self, panel: PanelId) {
        self.tab_nav.retain(|pid| *pid != panel);
        self.tab_nav_ix = self.tab_nav_ix.min(self.tab_nav.len().saturating_sub(1));
    }

    pub fn nav_back(&mut self, window: &mut Window, cx: &mut App) {
        while self.tab_nav_ix > 0 {
            self.tab_nav_ix -= 1;
            let pid = self.tab_nav[self.tab_nav_ix];
            if self.panel_alive(pid, cx) {
                self.activate_dock_panel(pid, window, cx);
                return;
            }
        }
    }

    pub fn nav_forward(&mut self, window: &mut Window, cx: &mut App) {
        while self.tab_nav_ix + 1 < self.tab_nav.len() {
            self.tab_nav_ix += 1;
            let pid = self.tab_nav[self.tab_nav_ix];
            if self.panel_alive(pid, cx) {
                self.activate_dock_panel(pid, window, cx);
                return;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tab open/close
    // -----------------------------------------------------------------------

    pub fn open_request_file(
        &mut self,
        node_id: usize,
        name: String,
        path: String,
        method: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(meta) = self.request_tabs.get(&node_id) {
            if self.panel_alive(meta.panel, cx) {
                let panel = meta.panel;
                self.nav_push(panel);
                self.activate_dock_panel(panel, window, cx);
                return;
            }
        }

        let playground = cx.new(|cx| RequestPlayground::new(window, cx));
        if method != "GET" {
            let methods: Vec<String> = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let row = methods.iter().position(|m| m == &method).unwrap_or(0);
            playground.update(cx, |pg, cx| {
                pg.method_entity().update(cx, |state, cx| {
                    state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
                });
            });
        }
        playground.update(cx, |pg, cx| {
            pg.set_tab_name(name, cx);
            pg.set_response_panel(self.response.clone());
        });
        let request = fs::request::read(Path::new(&path));
        playground.update(cx, |pg, cx| pg.load(window, cx, &request));
        playground.update(cx, |pg, _| pg.set_path(path.clone()));

        let pid = self.add_center_panel(playground.clone(), None, window, cx);
        self.request_tabs.insert(
            node_id,
            RequestTabMeta {
                panel: pid,
                view: playground.clone(),
                path: Some(path),
            },
        );

        let project_panel = self.project_panel.clone();
        cx.subscribe_in(
            &playground,
            window,
            move |_this: &mut Self, _, event, _window, cx| match event {
                RequestPlaygroundEvent::MethodChanged(method) => {
                    project_panel.update(cx, |pp, _| pp.set_node_method(node_id, method));
                }
                RequestPlaygroundEvent::ResponsePanelOpened => {}
            },
        )
        .detach();

        self.nav_push(pid);
        self.activate_dock_panel(pid, window, cx);
    }

    pub fn open_untitled_request(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_untitled_request_in(None, window, cx);
    }

    pub fn open_untitled_request_in(
        &mut self,
        target: Option<NodeId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let node_id = next_id();
        let playground = cx.new(|cx| RequestPlayground::new(window, cx));
        playground.update(cx, |pg, cx| {
            pg.set_tab_name("Untitled".to_string(), cx);
            pg.set_response_panel(self.response.clone());
        });
        let pid = self.add_center_panel(playground.clone(), target, window, cx);
        self.request_tabs.insert(
            node_id,
            RequestTabMeta {
                panel: pid,
                view: playground,
                path: None,
            },
        );
        self.nav_push(pid);
        self.activate_dock_panel(pid, window, cx);
    }

    pub fn rename_request_tab(
        &mut self,
        node_id: usize,
        new_name: String,
        new_path: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(meta) = self.request_tabs.get(&node_id) {
            meta.view.update(cx, |pg, cx| {
                pg.rename_file(new_name, new_path, cx);
            });
        }
    }

    pub fn close_request(&mut self, node_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(meta) = self.request_tabs.remove(&node_id) else {
            return;
        };
        if AppSettings::global(cx)
            .playground
            .request_playground
            .save_on_close
        {
            let content = meta.view.read(cx).current_content(cx);
            if let Some(path) = meta.view.read(cx).path() {
                fs::request::write(Path::new(&path), &content).ok();
            }
        }
        let method = meta.view.read(cx).stored_method(cx);
        self.project_panel
            .update(cx, |pp, _| pp.set_node_method(node_id, &method));
        self.drop_nav_panel(meta.panel);
        self.remove_center_panel(meta.view, window, cx);
    }

    pub fn open_env_tab(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        for meta in &self.env_tabs {
            if self.panel_alive(meta.panel, cx) && meta.view.read(cx).name(cx) == name {
                let panel = meta.panel;
                self.nav_push(panel);
                self.activate_dock_panel(panel, window, cx);
                return;
            }
        }
        let playground = cx.new(|cx| EnvPlayground::new(name, window, cx));
        let pid = self.add_center_panel(playground.clone(), None, window, cx);
        self.env_tabs.push(EnvTabMeta {
            panel: pid,
            view: playground.clone(),
        });

        let env_panel = self.env_panel.clone();
        cx.subscribe_in(
            &playground,
            window,
            move |_this: &mut Self, _, event, _window, cx| match event {
                EnvPlaygroundEvent::Renamed { .. } => {
                    env_panel.update(cx, |panel, cx| panel.refresh(cx));
                    cx.notify();
                }
            },
        )
        .detach();

        self.nav_push(pid);
        self.activate_dock_panel(pid, window, cx);
    }

    pub fn close_env_tab_by_name(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let found = self
            .env_tabs
            .iter()
            .enumerate()
            .find(|(_, meta)| {
                self.panel_alive(meta.panel, cx) && meta.view.read(cx).name(cx) == name
            })
            .map(|(ix, _)| ix);
        if let Some(ix) = found {
            let meta = self.env_tabs.remove(ix);
            self.remove_center_panel(meta.view, window, cx);
        }
    }

    pub fn add_stress_test_tab(
        &mut self,
        path: String,
        node_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let source = self
            .request_tabs
            .values()
            .find(|meta| meta.path.as_deref() == Some(path.as_str()))
            .map(|meta| meta.view.downgrade());
        let view = cx.new(|cx| StressTesting::new(source, path, node_name, window, cx));
        let pid = self.add_center_panel(view, None, window, cx);
        self.nav_push(pid);
        self.activate_dock_panel(pid, window, cx);
    }

    pub fn open_welcome_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pid) = self.welcome_panel {
            if self.panel_alive(pid, cx) {
                self.nav_push(pid);
                self.activate_dock_panel(pid, window, cx);
                return;
            }
        }
        let pid = self.add_center_panel(self.welcome.clone(), None, window, cx);
        self.welcome_panel = Some(pid);
        self.nav_push(pid);
        self.activate_dock_panel(pid, window, cx);
    }

    pub fn reset_center_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Untrack first: programmatic removals must NOT fire close effects.
        let requests: Vec<RequestTabMeta> = std::mem::take(&mut self.request_tabs)
            .into_values()
            .collect();
        let envs: Vec<EnvTabMeta> = std::mem::take(&mut self.env_tabs);
        let welcome = self.welcome_panel.take();
        self.tab_nav.clear();
        self.tab_nav_ix = 0;
        let area = self.dock_area(cx);
        area.update(cx, |area, cx| {
            for meta in requests {
                area.remove_panel(meta.view, window, cx);
            }
            for meta in envs {
                area.remove_panel(meta.view, window, cx);
            }
        });
        if welcome.is_some() {
            let welcome = self.welcome.clone();
            area.update(cx, |area, cx| {
                area.remove_panel(welcome, window, cx);
            });
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // Close hook (registry → handle_panel_closed)
    // -----------------------------------------------------------------------

    pub fn install_close_hook(&mut self, cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        let hook: crate::dock::tabs::CloseHook = std::rc::Rc::new(move |panel, cx| {
            weak.update(cx, |this, cx| this.handle_panel_closed(panel, cx))
                .ok();
        });
        self.shell
            .read(cx)
            .tab_registry()
            .set_close_hook(Some(hook));
    }

    fn handle_panel_closed(&mut self, panel: PanelId, cx: &mut App) {
        let found = self
            .request_tabs
            .iter()
            .find(|(_, meta)| meta.panel == panel)
            .map(|(id, _)| *id);
        if let Some(node_id) = found {
            if let Some(meta) = self.request_tabs.remove(&node_id) {
                if AppSettings::global(cx)
                    .playground
                    .request_playground
                    .save_on_close
                {
                    let content = meta.view.read(cx).current_content(cx);
                    if let Some(path) = meta.view.read(cx).path() {
                        fs::request::write(Path::new(&path), &content).ok();
                    }
                }
                let method = meta.view.read(cx).stored_method(cx);
                self.project_panel
                    .update(cx, |pp, _| pp.set_node_method(node_id, &method));
            }
        }
        if let Some(ix) = self.env_tabs.iter().position(|meta| meta.panel == panel) {
            self.env_tabs.remove(ix);
        }
        if self.welcome_panel == Some(panel) {
            self.welcome_panel = None;
        }
        self.drop_nav_panel(panel);
    }

    // -----------------------------------------------------------------------
    // Response aux slot
    // -----------------------------------------------------------------------

    pub fn sync_response_aux(&mut self, cx: &mut Context<Self>) {
        let show = self.response.read(cx).is_shown();
        let mounted = self.shell.read(cx).aux_info().is_some();
        let visible = self.shell.read(cx).aux_visible();
        if show && !mounted {
            let view: AnyView = self.response.clone().into();
            self.shell.update(cx, |shell, cx| {
                shell.set_aux_named("response", "Response", Some(view), cx);
            });
        } else if show && !visible {
            self.shell.update(cx, |shell, cx| shell.toggle_aux(cx));
        } else if !show && visible {
            self.shell.update(cx, |shell, cx| shell.toggle_aux(cx));
        }
    }

    // -----------------------------------------------------------------------
    // Event subscriptions
    // -----------------------------------------------------------------------

    /// Subscribe to panel/dock events that drive tab lifecycle.
    pub fn subscribe_dock_events(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let project_panel = self.project_panel.clone();
        cx.subscribe_in(
            &project_panel,
            window,
            |this: &mut Self, _, event, window, cx| match event {
                crate::project_panel::ProjectPanelEvent::FileActivated {
                    node_id,
                    name,
                    path,
                    method,
                } => {
                    this.project_panel.update(cx, |pp, cx| {
                        pp.set_active_node(Some(*node_id), cx);
                    });
                    this.open_request_file(
                        *node_id,
                        name.clone(),
                        path.clone(),
                        method.clone(),
                        window,
                        cx,
                    );
                }
                crate::project_panel::ProjectPanelEvent::FileRenamed {
                    node_id,
                    new_name,
                    new_path,
                } => {
                    this.rename_request_tab(*node_id, new_name.clone(), new_path.clone(), cx);
                }
                crate::project_panel::ProjectPanelEvent::FileDeleted { node_id, .. }
                | crate::project_panel::ProjectPanelEvent::FileTrashed { node_id, .. } => {
                    this.close_request(*node_id, window, cx);
                }
                crate::project_panel::ProjectPanelEvent::StressTestPlayground {
                    path,
                    node_name,
                } => {
                    this.add_stress_test_tab(path.clone(), node_name.clone(), window, cx);
                }
            },
        )
        .detach();

        let env_panel = self.env_panel.clone();
        cx.subscribe_in(
            &env_panel,
            window,
            |this: &mut Self, _, event, window, cx| match event {
                crate::env_panel::EnvPanelEvent::EnvActivated { name } => {
                    this.open_env_tab(name.clone(), window, cx);
                }
                crate::env_panel::EnvPanelEvent::EnvDeleted { name } => {
                    this.close_env_tab_by_name(name, window, cx);
                }
            },
        )
        .detach();

        // Sync the response aux slot whenever the response panel changes.
        cx.observe(&self.response, |this, _, cx| {
            this.sync_response_aux(cx);
        })
        .detach();
    }

    // -----------------------------------------------------------------------
    // Bootstrap
    // -----------------------------------------------------------------------

    pub fn seed_empty_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let empty = cx.new(EmptyTab::new);
        self.shell.clone().update(cx, |shell, cx| {
            shell.set_center(DockLayout::tabs().panel(empty).active_index(0), window, cx);
        });
    }
}
