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
mod response_panel;
mod settings_panel;
mod settings_window;
mod tab;
mod tab_manager;
mod themes_and_fonts;

use crate::actions::{CreateFile, CreateFolder, RenameItem};
use crate::assets::Assets;
use crate::env::EnvironmentStore;
use crate::footer::{Footer, FooterEvent};
use crate::list::WorkspaceListItem;
use crate::project_panel::{ProjectPanel, ProjectPanelEvent};
use crate::settings_window::SettingsWindow;
use crate::tab_manager::TabManagerEvent;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::list::{List, ListState};
use gpui_component::popover::Popover;
use gpui_component::{Theme, *};
use std::path::PathBuf;

use crate::icons::IconName;

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
        let tab_manager = cx.new(|cx| tab_manager::TabManager::new(window, cx));
        let footer = cx.new(|cx| Footer::new(window, cx));
        let env_store = cx.new(|cx| EnvironmentStore::new(window, cx));
        let workspace_list = cx.new(|cx| {
            ListState::new(WorkspaceListItem::new(project_panel.clone()), window, cx)
                .searchable(true)
        });

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
                TabManagerEvent::TabActivated(node_id) => {
                    project_panel.update(cx, |pp, cx| {
                        pp.set_active_node(*node_id);
                        cx.notify();
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
                    if let Some(pg) = tab_manager.read(cx).active_playground(cx) {
                        pg.update(cx, |pg, cx| {
                            pg.respone_panel_entity().update(cx, |panel, cx| {
                                panel.toggle(cx);
                            });
                        });
                    }
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

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_tabs = self.tab_manager.read(cx).has_tabs();
        self.footer
            .update(cx, |f, cx| f.set_show_toggle(has_tabs, cx));
        self.footer.clone()
    }

    fn section_label(label: &'static str, cx: &App) -> impl IntoElement {
        div()
            .px_2()
            .pt_1()
            .pb_0p5()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(label.to_string())
    }

    fn footer_action(label: &'static str, icon: Option<IconName>, cx: &App) -> impl IntoElement {
        h_flex()
            .id(label)
            .w_full()
            .px_2()
            .py_1()
            .gap_2()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(cx.theme().secondary_hover))
            .when_some(icon, |row, icon| {
                row.child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
            })
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(label.to_string()),
            )
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
            .child(
                TitleBar::new().h(px(40.)).bg(cx.theme().background).child(
                    h_flex().gap_2().items_center().px_2().w_full().child(
                        // In titlebar, trigger is the current workspace name:
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
