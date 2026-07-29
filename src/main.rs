mod actions;
mod auth;
mod body;
mod footer;
mod fs;
mod headers;
mod helpers;
mod http;
mod playground;
mod project_panel;
mod query_params;
mod response_panel;
mod tab;
mod tab_manager;

use crate::actions::{CreateFile, RenameFile};
use crate::project_panel::{ProjectPanel, ProjectPanelEvent};
use crate::tab_manager::TabManagerEvent;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::popover::Popover;
use gpui_component::{Theme, *};
use std::path::PathBuf;

pub(crate) struct ApiClient {
    pub(crate) project_panel: Entity<project_panel::ProjectPanel>,
    pub(crate) tab_manager: Entity<tab_manager::TabManager>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = cx.new(|cx| ProjectPanel::new(window, cx));
        let tab_manager = cx.new(|cx| tab_manager::TabManager::new(window, cx));

        cx.subscribe_in(&project_panel, window, {
            let tab_manager = tab_manager.clone();
            move |_, _, event, window, cx| match event {
                ProjectPanelEvent::FileActivated {
                    node_id,
                    name,
                    path,
                    method,
                } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.activate_tab(
                            *node_id,
                            name.clone(),
                            path.clone(),
                            method.clone(),
                            window,
                            cx,
                        );
                    });
                }
                ProjectPanelEvent::FileRenamed { node_id, new_name } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.rename_tab(*node_id, new_name.clone(), cx);
                    });
                }
            }
        })
        .detach();

        cx.subscribe_in(&tab_manager, window, {
            let project_panel = project_panel.clone();
            let tab_manager = tab_manager.clone();
            move |_, _, event, _window, cx| match event {
                TabManagerEvent::MethodChanged(node_id, method) => {
                    project_panel.update(cx, |pp, _| pp.set_node_method(*node_id, method));
                }
                TabManagerEvent::SidebarToggle(collapsed) => {
                    project_panel.update(cx, |pp, cx| pp.set_collapsed(*collapsed, cx));
                }
                TabManagerEvent::ResponseToggle => {
                    if let Some(pg) = tab_manager.read(cx).active_playground(cx) {
                        pg.update(cx, |pg, cx| {
                            pg.respone_panel_entity().update(cx, |panel, cx| {
                                panel.toggle(cx);
                            });
                        });
                    }
                } // TabManagerEvent::TabClosed(_node_id) => {}
            }
        })
        .detach();

        Self {
            project_panel,
            tab_manager,
        }
    }
}

impl ApiClient {
    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_create_file(action, window, cx));
    }

    pub fn handle_rename(
        &mut self,
        action: &RenameFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_rename_file(action, window, cx));
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let names = self.project_panel.read(cx).workspace_names();
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_rename))
            .child(
                TitleBar::new().h(px(40.)).bg(cx.theme().background).child(
                    h_flex().gap_2().items_center().px_2().w_full().child(
                        // In titlebar, trigger is the current workspace name:
                        Popover::new("workspace-switcher")
                            // .appearance(false)
                            .anchor(Anchor::TopLeft)
                            .trigger(
                                Button::new("workspace")
                                    .ghost()
                                    .small()
                                    .label(self.project_panel.read(cx).get_selected_workspace()),
                            )
                            .content(move |_, window, cx| {
                                v_flex()
                                    .gap_1()
                                    .min_w(px(180.))
                                    .children(names.iter().map(|ws| {
                                        div()
                                            .id(ws.clone())
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .hover(|s| s.bg(cx.theme().secondary_hover))
                                            .text_sm()
                                            .text_color(cx.theme().foreground)
                                            .child(ws.clone())
                                            .on_click(cx.listener(move |_, _, window, cx| {
                                                // switch workspace
                                            }))
                                    }))
                            }),
                    ),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .child(self.project_panel.clone())
                    .child(self.tab_manager.clone()),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        let theme_name = SharedString::from("One Dark");
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
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|view_cx| ApiClient::new(window, view_cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
