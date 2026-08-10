mod actions;
pub mod assets;
mod auth;
mod body;
mod env;
mod footer;
mod fs;
mod headers;
mod helpers;
mod http;
mod icons;
mod list;
mod playground;
mod project_panel;
mod query_params;
mod request_playground;
mod response_panel;
mod settings_panel;
mod settings_window;
mod stress_testing;
mod tab;
mod tab_manager;

use crate::actions::{CreateFile, CreateFolder, RenameItem, StressTestPlayground};
use crate::assets::Assets;
use crate::env::EnvironmentStore;
use crate::footer::{Footer, FooterEvent};
use crate::list::{WorkspaceListItem, WorkspaceListItemEvent};
use crate::project_panel::{ProjectPanel, ProjectPanelEvent};
use crate::settings_window::SettingsWindow;
// use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::list::{List, ListState};
use gpui_component::popover::Popover;
use gpui_component::{Theme, *};
use std::path::PathBuf;

// use crate::icons::IconName;

pub struct ApiClient {
    project_panel: Entity<project_panel::ProjectPanel>,
    tab_manager: Entity<tab_manager::TabManager>,
    footer: Entity<Footer>,
    env_store: Entity<EnvironmentStore>,
    workspace_list: Entity<ListState<WorkspaceListItem>>,
    settings_window: Option<WeakEntity<SettingsWindow>>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = cx.new(|cx| ProjectPanel::new(window, cx));
        let tab_manager =
            cx.new(|cx| tab_manager::TabManager::new(window, cx, project_panel.clone()));
        let footer = cx.new(|cx| Footer::new(window, cx));
        let env_store = cx.new(|cx| EnvironmentStore::new(window, cx));
        let workspace_list = cx.new(|cx| {
            ListState::new(WorkspaceListItem::new(project_panel.clone()), window, cx)
                .searchable(true)
        });

        let pp = project_panel.clone();
        cx.spawn(async move |_, cx| {
            let dirs = cx
                .background_executor()
                .spawn(ProjectPanel::list_workspace_dirs())
                .await;

            let selected_ix = pp.update(cx, |pp, cx| pp.set_workspaces(dirs, cx));
            if let Some(path) = pp.update(cx, |pp, _| pp.get_workspace(selected_ix).map(|(_, p)| p))
            {
                let tree = cx
                    .background_executor()
                    .spawn(
                        async move { ProjectPanel::read_dir_to_nodes(std::path::Path::new(&path)) },
                    )
                    .await;

                pp.update(cx, |pp, cx| {
                    pp.set_workspace_tree(selected_ix, tree, cx);
                    cx.notify();
                });
            }
        })
        .detach();

        cx.subscribe_in(&workspace_list, window, {
            let project_panel = project_panel.clone();
            let tab_manager = tab_manager.clone();
            move |_, _, event, window, cx| match event {
                WorkspaceListItemEvent::WorkspaceChanged(name) => {
                    let ix = project_panel.update(cx, |pp, cx| pp.reset(window, cx, name.clone()));
                    tab_manager.update(cx, |tb, cx| tb.reset(window, cx));

                    let pp = project_panel.clone();
                    cx.spawn(async move |_, cx| {
                        let path = pp.update(cx, |pp, _| pp.get_workspace(ix).map(|(_, p)| p));
                        if let Some(path) = path {
                            let tree = cx
                                .background_executor()
                                .spawn(async move {
                                    ProjectPanel::read_dir_to_nodes(std::path::Path::new(&path))
                                })
                                .await;
                            pp.update(cx, |pp, cx| {
                                pp.set_workspace_tree(ix, tree, cx);
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }
            }
        })
        .detach();

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
                        tm.activate_request_tab(
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
                ProjectPanelEvent::StressTestPlayground { path, node_name } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.add_stress_test_tab(window, cx, path.clone(), node_name.clone());
                    });
                }
            }
        })
        .detach();

        cx.subscribe_in(&footer, window, {
            let tab_manager = tab_manager.clone();
            let env_store = env_store.clone();
            move |this: &mut Self, _, event, _window, cx| match event {
                FooterEvent::ToggleResponse => {
                    tab_manager.update(cx, |tm, cx| tm.toggle_active_response(cx));
                }
                FooterEvent::ToggleSettings(_) => {
                    if this
                        .settings_window
                        .as_ref()
                        .is_none_or(|w| w.upgrade().is_none())
                    {
                        open_settings_window(cx.entity(), env_store.clone(), cx);
                    }
                }
            }
        })
        .detach();

        Self {
            project_panel,
            tab_manager,
            footer,
            env_store,
            workspace_list,
            settings_window: None,
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

    pub fn handle_create_folder(
        &mut self,
        action: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_create_folder(action, window, cx));
    }

    pub fn handle_rename(
        &mut self,
        action: &RenameItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_rename_item(action, window, cx));
    }

    pub fn handle_stress_test_playground(
        &mut self,
        action: &StressTestPlayground,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.activate_stress_test_playground(action, window, cx));
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_tabs = self.tab_manager.read(cx).has_tabs();
        self.footer
            .update(cx, |f, cx| f.set_show_toggle(has_tabs, cx));
        self.footer.clone()
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let project_panel = self.project_panel.clone();
        let workspace_list = self.workspace_list.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_create_folder))
            .on_action(cx.listener(Self::handle_rename))
            .on_action(cx.listener(Self::handle_stress_test_playground))
            .child(
                TitleBar::new().h(px(40.)).bg(cx.theme().background).child(
                    h_flex().gap_2().items_center().px_2().w_full().child(
                        Popover::new("workspace-switcher")
                            .anchor(Anchor::TopLeft)
                            .trigger(
                                Button::new("workspace")
                                    .ghost()
                                    .small()
                                    .label(project_panel.read(cx).get_selected_workspace()),
                            )
                            .content(move |_, _window, cx| {
                                v_flex()
                                    .w(px(260.))
                                    .py_1()
                                    .gap_0()
                                    .child(List::new(&workspace_list).max_h(px(320.)))
                                    .child(div().h(px(1.)).w_full().my_1().bg(cx.theme().border))
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
            .child(self.render_footer(cx))
    }
}

fn open_settings_window(
    api_client: Entity<ApiClient>,
    env_store: Entity<EnvironmentStore>,
    cx: &mut App,
) {
    let window_bounds = WindowBounds::centered(size(px(960.), px(680.)), cx);
    cx.spawn(async move |cx| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_decorations: Some(WindowDecorations::Client),
                window_bounds: Some(window_bounds),
                ..Default::default()
            },
            |window, cx| {
                let settings = cx.new(|cx| SettingsWindow::new(env_store, window, cx));
                api_client.update(cx, |client, cx| {
                    client.settings_window = Some(settings.downgrade());
                    cx.notify();
                });
                cx.new(|cx| Root::new(settings, window, cx))
            },
        )
        .expect("Failed to open settings window");
    })
    .detach();
}
fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        gpui_component::init(cx);

        let theme_name = SharedString::from("One Dark");
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }
        if let Err(err) =
            ThemeRegistry::watch_dir(PathBuf::from("./assets/themes"), cx, move |cx| {
                if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                    Theme::global_mut(cx).apply_config(&theme);
                }
            })
        {
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
