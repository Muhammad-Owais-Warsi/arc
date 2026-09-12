use std::path::Path;

use gpui_kit::base::dock::{DockEvent, DockLayout, DockPlacement, InsertTarget, NodeId, PanelId};
use gpui_kit::component::IndexPath;
use gpui_kit::*;

use crate::dock::tabs::EmptyTab;
use crate::env_playground::{EnvPlayground, EnvPlaygroundEvent};
use crate::fs;
use crate::helpers::next_id;
use crate::request_playground::{RequestPlayground, RequestPlaygroundEvent};
use crate::settings_panel::AppSettings;
use crate::stress_testing::StressTesting;

// ---------------------------------------------------------------------------
// Dock-tab helpers on ApiClient. The old TabManager (IndexMap of Tabs
// wrappers, manual TabBar, history) is gone: center tabs are dock panels,
// the strip is the skin's TabBar. This keeps the behaviors that matter:
// open/dedup/activate, rename sync, save-on-close write-through, response
// sharing, and workspace reset. Tab selection itself lives in the dock.
// ---------------------------------------------------------------------------

pub struct RequestTabMeta {
    pub panel: PanelId,
    pub view: Entity<RequestPlayground>,
    pub path: Option<String>,
}

pub struct EnvTabMeta {
    pub panel: PanelId,
    pub view: Entity<EnvPlayground>,
}

// Simple browser-like tab navigation (replaces the old TabManager
// history): every tab WE open/activate is pushed; back/forward walk the
// stack, skipping panels closed since. Strip clicks bypass the stack —
// kept simple on purpose.


// Tab back/forward history (manual strip era). Commented out until an
// external history (Vec<PanelId> synced to dock active changes) is built.
// struct TabHistory {
//     history: Vec<usize>,
//     history_index: usize,
// }
// fn push_history(...) / remove_from_history(...) / can_back() / can_forward()
// / back() / forward() — see git history of this file.

impl crate::ApiClient {
    fn dock_area(&self, cx: &App) -> Entity<gpui_kit::base::dock::DockArea> {
        self.shell.read(cx).area()
    }

    fn panel_alive(&self, panel: PanelId, cx: &App) -> bool {
        self.dock_area(cx).read(cx).panel(panel).is_some()
    }

    fn activate_dock_panel(&self, panel: PanelId, window: &mut Window, cx: &mut App) {
        // Resolve the panel's CURRENT slot and re-insert there: ix: None
        // would append at the end and visibly reshuffle the strip.
        let area = self.dock_area(cx);
        let slot = area.read(cx).layout(DockPlacement::Center).and_then(|tree| {
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
            let shell = self.shell.clone();
            shell.update(cx, |shell, cx| {
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

    fn nav_push(&mut self, panel: PanelId) {
        if self.tab_nav.last() == Some(&panel) {
            self.tab_nav_ix = self.tab_nav.len().saturating_sub(1);
            return;
        }
        self.tab_nav.truncate(self.tab_nav_ix + 1);
        self.tab_nav.push(panel);
        self.tab_nav_ix = self.tab_nav.len().saturating_sub(1);
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

    fn add_center_panel<P>(
        &self,
        view: Entity<P>,
        target: Option<gpui_kit::base::dock::NodeId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PanelId
    where
        P: gpui_kit::base::dock::Panel,
    {
        let pid = PanelId::from(view.entity_id());
        let shell = self.shell.clone();
        shell.update(cx, |shell, cx| {
            shell.add_panel(view, DockPlacement::Center, window, cx);
        });
        if let Some(node) = target {
            let shell = self.shell.clone();
            shell.update(cx, |shell, cx| {
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

    fn remove_center_panel<P>(
        &self,
        view: Entity<P>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        P: gpui_kit::base::dock::Panel,
    {
        let area = self.dock_area(cx);
        area.update(cx, |area, cx| {
            area.remove_panel(view, window, cx);
        });
    }

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
            let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                .into_iter()
                .map(String::from)
                .collect();
            let row = methods.iter().position(|m| *m == method).unwrap_or(0);
            playground.update(cx, |pg, cx| {
                pg.method_entity().update(cx, |state, cx| {
                    state.set_selected_index(
                        Some(IndexPath::default().row(row)),
                        window,
                        cx,
                    );
                })
            });
        }
        playground.update(cx, |pg, cx| {
            pg.set_tab_name(name.clone(), cx);
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

        cx.subscribe_in(
            &playground,
            window,
            move |this: &mut Self, _, event, _window, cx| match event {
                RequestPlaygroundEvent::MethodChanged(method) => {
                    this.project_panel
                        .update(cx, |pp, _| pp.set_node_method(node_id, method));
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

    fn forget_request_tab(&mut self, node_id: usize) -> Option<RequestTabMeta> {
        self.request_tabs.remove(&node_id)
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

        let playground = cx.new(|cx| EnvPlayground::new(name.clone(), window, cx));
        let pid = self.add_center_panel(playground.clone(), None, window, cx);
        self.env_tabs.push(EnvTabMeta {
            panel: pid,
            view: playground.clone(),
        });

        cx.subscribe_in(
            &playground,
            window,
            |this: &mut Self, _, event, _window, cx| match event {
                EnvPlaygroundEvent::Renamed { .. } => {
                    this.env_panel.update(cx, |panel, cx| panel.refresh(cx));
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
            .map(|(ix, meta)| (ix, meta.panel, meta.view.clone()))
            .find(|(_, panel, view)| {
                self.panel_alive(*panel, cx) && view.read(cx).name(cx) == name
            })
            .map(|(ix, _, _)| ix);
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
        // Untrack first: programmatic removals must NOT run close effects.
        let requests: Vec<RequestTabMeta> =
            std::mem::take(&mut self.request_tabs).into_values().collect();
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

    /// Close side-effects for user-closed ([x]) tabs, discovered via
    /// `DockEvent::LayoutChanged`: save-on-close write-through + method
    /// sync back to the project tree. Programmatic removals untrack first,
    /// so only real closes land here.
    pub fn reconcile_closed_tabs(&mut self, cx: &mut Context<Self>) {
        let gone: Vec<usize> = self
            .request_tabs
            .iter()
            .filter(|(_, meta)| !self.panel_alive(meta.panel, cx))
            .map(|(id, _)| *id)
            .collect();
        for node_id in gone {
            if let Some(meta) = self.request_tabs.remove(&node_id) {
                let save_on_close = AppSettings::global(cx)
                    .playground
                    .request_playground
                    .save_on_close;
                if save_on_close {
                    let content = meta.view.read(cx).current_content(cx);
                    if let Some(path) = meta.view.read(cx).path() {
                        fs::request::write(Path::new(&path), &content).ok();
                    }
                }
                let method = meta.view.read(cx).stored_method(cx);
                self.project_panel.update(cx, |pp, _| {
                    pp.set_node_method(node_id, &method);
                });
            }
        }
        let mut ix = 0;
        while ix < self.env_tabs.len() {
            let alive = {
                let meta = &self.env_tabs[ix];
                self.panel_alive(meta.panel, cx)
            };
            if alive {
                ix += 1;
            } else {
                self.env_tabs.remove(ix);
            }
        }
        if let Some(pid) = self.welcome_panel {
            if !self.panel_alive(pid, cx) {
                self.welcome_panel = None;
            }
        }
        let live_nav: Vec<PanelId> = self
            .tab_nav
            .iter()
            .copied()
            .filter(|pid| self.panel_alive(*pid, cx))
            .collect();
        self.tab_nav = live_nav;
        self.tab_nav_ix = self
            .tab_nav_ix
            .min(self.tab_nav.len().saturating_sub(1));
    }

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
            // Mounted but hidden (toggled off earlier): re-show it.
            self.shell.update(cx, |shell, cx| {
                shell.toggle_aux(cx);
            });
        } else if !show && visible {
            self.shell.update(cx, |shell, cx| {
                shell.toggle_aux(cx);
            });
        }
    }

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
                    if let Some(meta) = this.forget_request_tab(*node_id) {
                        let view = meta.view.clone();
                        this.remove_center_panel(view, window, cx);
                    }
                }
                crate::project_panel::ProjectPanelEvent::StressTestPlayground { path, node_name } => {
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

        let area = self.dock_area(cx);
        cx.subscribe_in(
            &area,
            window,
            |this: &mut Self, _, event, _window, cx| match event {
                DockEvent::LayoutChanged => {
                    this.reconcile_closed_tabs(cx);
                }
                DockEvent::DragDrop { .. } => {}
            },
        )
        .detach();

        cx.observe(&self.response, |this, _, cx| {
            this.sync_response_aux(cx);
            // Response state drives the footer toggle too (show_toggle on
            // first data, collapsed mirror) — not just shell changes.
            this.sync_footer_from_shell(cx);
        })
        .detach();
    }

    pub fn seed_empty_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let empty = cx.new(EmptyTab::new);
        let shell = self.shell.clone();
        shell.update(cx, |shell, cx| {
            shell.set_center(
                DockLayout::tabs().panel(empty).active_index(0),
                window,
                cx,
            );
        });
    }
}
