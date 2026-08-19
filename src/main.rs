mod actions;
pub mod assets;
mod auth;
mod body;
mod env;
mod footer;
mod fs;
mod headers;
mod helpers;
mod http_client;
mod http_request;
mod http_response;
mod icons;
mod playground;
mod project_panel;
mod query_params;
mod request_playground;
mod response_panel;
mod settings_panel;
mod settings_window;
mod stress_engine;
mod stress_testing;
mod tab;
mod tab_manager;
mod welcome;

use crate::actions::{
    CopyPath, CopyRelativePath, CopySettings, CreateFile, CreateFolder, DeleteItem,
    DockSidebarLeft, DockSidebarRight, OpenEnvironmentVariables, OpenSettings, QuitArc, RenameItem,
    StressTestPlayground, TrashItem,
};
use crate::assets::Assets;
use crate::env::EnvironmentStore;
use crate::footer::{Footer, FooterEvent};
use crate::icons::IconName;
use crate::project_panel::{DirTree, ProjectPanel, ProjectPanelEvent};
use crate::settings_panel::{AppSettings, SidebarDock};
use crate::settings_window::SettingsWindow;
use crate::welcome::WelcomeScreen;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::command::{Command, CommandItem, CommandState};
use gpui_component::menu::DropdownMenu;
use gpui_component::popover::Popover;
use gpui_component::{Theme, *};

pub struct ApiClient {
    project_panel: Entity<project_panel::ProjectPanel>,
    tab_manager: Entity<tab_manager::TabManager>,
    footer: Entity<Footer>,
    env_store: Entity<EnvironmentStore>,
    workspace_palette: Entity<CommandState>,
    workspace_palette_open: bool,
    workspaces: Vec<(String, String)>,
    selected_workspace: Option<usize>,
    settings_window: Option<(WeakEntity<SettingsWindow>, AnyWindowHandle)>,
    welcome: Entity<WelcomeScreen>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = cx.new(|cx| ProjectPanel::new(window, cx));
        let env_store = cx.new(|cx| EnvironmentStore::new(window, cx));

        let workspace_palette = cx.new(|cx| CommandState::new(window, cx));

        let tab_manager = cx.new(|cx| {
            tab_manager::TabManager::new(window, cx, project_panel.clone(), env_store.clone())
        });
        let footer = cx.new(|cx| Footer::new(window, cx));
        let welcome = cx.new(|cx| WelcomeScreen::new(window, cx));

        Self {
            project_panel,
            tab_manager,
            footer,
            env_store,
            workspace_palette,
            workspace_palette_open: false,
            workspaces: Vec::new(),
            selected_workspace: None,
            settings_window: None,
            welcome,
        }
    }

    fn init(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.start_walkdir(window, cx);
        self.footer_event_handler(window, cx);
        self.project_panel_event_handler(window, cx);
    }

    fn switch_workspace_to(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((name, path)) = self.workspaces.get(ix).cloned() else {
            return;
        };
        self.selected_workspace = Some(ix);
        self.workspace_palette.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(ix)), window, cx);
        });
        self.tab_manager.update(cx, |tb, cx| tb.reset(window, cx));

        let project_panel = self.project_panel.clone();
        cx.spawn(async move |_, cx| {
            let tree_path = path.clone();
            let tree =
                cx.background_executor()
                    .spawn(async move {
                        ProjectPanel::read_dir_to_nodes(std::path::Path::new(&tree_path))
                    })
                    .await;
            project_panel.update(cx, |pp, cx| {
                pp.set_tree(name, path, tree, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn start_walkdir(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspaces = ProjectPanel::list_workspace_dirs()
            .into_iter()
            .map(|(name, path)| (name, path.to_string_lossy().to_string()))
            .collect();
        self.selected_workspace = self.workspaces.first().map(|_| 0);

        let project_panel = self.project_panel.clone();
        if let Some((name, path)) = self
            .selected_workspace
            .and_then(|ix| self.workspaces.get(ix))
            .cloned()
        {
            cx.spawn(async move |_, cx| {
                let tree_path = path.clone();
                let tree = cx
                    .background_executor()
                    .spawn(async move {
                        ProjectPanel::read_dir_to_nodes(std::path::Path::new(&tree_path))
                    })
                    .await;

                project_panel.update(cx, |pp, cx| {
                    pp.set_tree(name, path, tree, cx);
                });
            })
            .detach();
        } else {
            let tab_manager = self.tab_manager.clone();
            let welcome = self.welcome.clone();
            tab_manager.update(cx, |tm, cx| {
                tm.open_welcome_tab(window, cx, welcome);
            });
        }
    }

    fn footer_event_handler(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(&self.footer, window, {
            let tab_manager = self.tab_manager.clone();
            move |this: &mut Self, _, event, _window, cx| match event {
                FooterEvent::ToggleResponse => {
                    tab_manager.update(cx, |tm, cx| tm.toggle_active_response(cx));
                }
            }
        })
        .detach();
    }

    fn project_panel_event_handler(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(&self.project_panel, window, {
            let tab_manager = self.tab_manager.clone();
            let project_panel = self.project_panel.clone();
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
                ProjectPanelEvent::FileDeleted {
                    node_id,
                    path: _,
                    is_file: _,
                } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.close_tab(*node_id, &project_panel, cx);
                    });
                }
                ProjectPanelEvent::FileTrashed { node_id, path: _ } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.close_tab(*node_id, &project_panel, cx);
                    });
                }
                ProjectPanelEvent::StressTestPlayground { path, node_name } => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.add_stress_test_tab(window, cx, path.clone(), node_name.clone());
                    });
                }
                ProjectPanelEvent::CopyPath { path } => {
                    cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
                }
                ProjectPanelEvent::CopyRelativePath { path } => {
                    cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
                }
            }
        })
        .detach();
    }

    fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_create_file(action, window, cx));
    }

    fn handle_create_folder(
        &mut self,
        action: &CreateFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_create_folder(action, window, cx));
    }

    fn handle_rename(&mut self, action: &RenameItem, window: &mut Window, cx: &mut Context<Self>) {
        self.project_panel
            .update(cx, |s, cx| s.handle_rename_item(action, window, cx));
    }

    fn handle_delete_item(
        &mut self,
        action: &DeleteItem,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_delete_item(action, cx));
    }

    fn handle_trash_item(
        &mut self,
        action: &TrashItem,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel
            .update(cx, |s, cx| s.handle_trash_item(action, cx));
    }

    fn handle_stress_test_playground(
        &mut self,
        action: &StressTestPlayground,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel.update(cx, |s, cx| {
            s.activate_stress_test_playground(action, window, cx)
        });
    }

    fn handle_dock_sidebar_left(
        &mut self,
        _: &DockSidebarLeft,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        AppSettings::global_mut(cx).panel.project_panel.sidebar_dock = SidebarDock::Left;
        AppSettings::global_mut(cx).save();
        cx.refresh_windows();
    }

    fn handle_dock_sidebar_right(
        &mut self,
        _: &DockSidebarRight,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        AppSettings::global_mut(cx).panel.project_panel.sidebar_dock = SidebarDock::Right;
        AppSettings::global_mut(cx).save();
        cx.refresh_windows();
    }

    fn handle_copy_path(
        &mut self,
        action: &CopyPath,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(action.path.clone()));
    }

    fn handle_copy_relative_path(
        &mut self,
        action: &CopyRelativePath,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(action.path.clone()));
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let show_toggle = self
            .tab_manager
            .read(cx)
            .active_playground(cx)
            .and_then(|p| p.response_panel(cx))
            .is_some();
        self.footer
            .update(cx, |f, cx| f.set_show_toggle(show_toggle, cx));
        self.footer.clone()
    }

    fn handle_open_settings(
        &mut self,
        _: &OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(cx.entity(), cx);
    }

    fn handle_quit_arc(&mut self, _: &QuitArc, window: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn handle_copy_settings(
        &mut self,
        _: &CopySettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let content = AppSettings::get();
        if let Ok(json_string) = serde_json::to_string_pretty(&content) {
            cx.write_to_clipboard(ClipboardItem::new_string(json_string));
        }
    }

    fn handle_open_environment_variables(
        &mut self,
        _: &OpenEnvironmentVariables,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_manager.update(cx, |tm, cx| {
            tm.add_env_tab(window, cx);
        });
    }

    fn render_titlebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = self.settings_window.clone();
        let this = cx.entity();
        let workspace_palette = self.workspace_palette.clone();
        let workspace_name = self
            .selected_workspace
            .and_then(|ix| self.workspaces.get(ix))
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "no workspace".to_string());

        TitleBar::new()
            .h(px(32.))
            .on_close_window(move |_, _, cx| {
                if let Some((_, settings_handle)) = settings_window.clone() {
                    cx.update_window(settings_handle, |_root, window, _cx| {
                        window.remove_window();
                    })
                    .ok();
                }
            })
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .gap_0p5()
                    .child(
                        Button::new("menu")
                            .icon(IconName::Menu)
                            .ghost()
                            .small()
                            .tooltip("Open Application Menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.min_w(px(270.))
                                    .link(
                                        "About Arc",
                                        "https://github.com/Muhammad-Owais-Warsi/arc",
                                    )
                                    .separator()
                                    .menu("Open Settings", Box::new(actions::OpenSettings))
                                    .menu("Copy Settings", Box::new(actions::CopySettings))
                                    .separator()
                                    .menu(
                                        "Open Environment Variables",
                                        Box::new(actions::OpenEnvironmentVariables),
                                    )
                                    .separator()
                                    .menu("Quit Arc", Box::new(actions::QuitArc))
                            }),
                    )
                    .child(
                        Popover::new("workspace-picker")
                            .open(self.workspace_palette_open)
                            .on_open_change({
                                let this = this.clone();
                                let palette = workspace_palette.clone();
                                move |is_open, window, cx| {
                                    if *is_open {
                                        palette.update(cx, |palette, cx| {
                                            palette.set_query("", window, cx);
                                        });
                                    }
                                    this.update(cx, |this, cx| {
                                        this.workspace_palette_open = *is_open;
                                        cx.notify();
                                    });
                                }
                            })
                            .trigger(
                                Button::new("workspace")
                                    .ghost()
                                    .small()
                                    .label(workspace_name.clone())
                                    .tooltip("Switch Workspace"),
                            )
                            .content({
                                let this = this.clone();
                                let palette = workspace_palette.clone();
                                move |_, _, cx| {
                                    let client = this.read(cx);
                                    let items = client
                                        .workspaces
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (name, _))| {
                                            CommandItem::new()
                                                .label(name.clone())
                                                .icon(IconName::BriefcaseBusiness)
                                                .checked(Some(i) == client.selected_workspace)
                                        })
                                        .collect::<Vec<_>>();
                                    Command::new(&palette)
                                        .bordered(false)
                                        .placeholder("Search or switch workspace")
                                        .w(px(260.))
                                        .items(items)
                                        .separator()
                                        .footer({
                                            let this = this.clone();
                                            let palette = palette.clone();
                                            move |_, window, cx| {
                                                Button::new("add-workspace")
                                                    .ghost()
                                                    .label("Add Workspace")
                                                    .icon(IconName::Plus)
                                                    .w_full()
                                                    .justify_start()
                                                    .on_click({
                                                        let this = this.clone();
                                                        let palette = palette.clone();
                                                        move |_, window, cx| {
                                                            let name = palette
                                                                .read(cx)
                                                                .query(cx)
                                                                .to_string();
                                                            let name = name.trim().to_string();
                                                            if name.is_empty() {
                                                                return;
                                                            }
                                                            let path = match fs::create_workspace(
                                                                &name,
                                                            ) {
                                                                Ok(path) => path,
                                                                Err(err) => {
                                                                    eprintln!(
                                                                        "Failed to create workspace: {err}"
                                                                    );
                                                                    return;
                                                                }
                                                            };
                                                            this.update(cx, |this, cx| {
                                                                if this
                                                                    .workspaces
                                                                    .iter()
                                                                    .any(|(n, _)| n == &name)
                                                                {
                                                                    return;
                                                                }
                                                                let ix = this.workspaces.len();
                                                                this.workspaces.push((
                                                                    name.clone(),
                                                                    path.clone(),
                                                                ));
                                                                this.project_panel.update(
                                                                    cx,
                                                                    |pp, cx| {
                                                                        pp.set_tree(
                                                                            name.clone(),
                                                                            path,
                                                                            DirTree {
                                                                                root_ids: Vec::new(),
                                                                                nodes: std::collections::HashMap::new(),
                                                                            },
                                                                            cx,
                                                                        );
                                                                    },
                                                                );
                                                                this.switch_workspace_to(
                                                                    ix, window, cx,
                                                                );
                                                                cx.notify();
                                                            });
                                                        }
                                                    })
                                                    .into_any_element()
                                            }
                                        })
                                        .on_confirm({
                                            let this = this.clone();
                                            move |index, window, cx| {
                                                this.update(cx, |this, cx| {
                                                    this.workspace_palette_open = false;
                                                    this.switch_workspace_to(
                                                        index.row, window, cx,
                                                    );
                                                    cx.notify();
                                                });
                                            }
                                        })
                                        .into_any_element()
                                }
                            }),
                    ),
            )
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::handle_copy_settings))
            .on_action(cx.listener(Self::handle_quit_arc))
            .on_action(cx.listener(Self::handle_open_settings))
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_create_folder))
            .on_action(cx.listener(Self::handle_rename))
            .on_action(cx.listener(Self::handle_delete_item))
            .on_action(cx.listener(Self::handle_stress_test_playground))
            .on_action(cx.listener(Self::handle_copy_path))
            .on_action(cx.listener(Self::handle_copy_relative_path))
            .on_action(cx.listener(Self::handle_dock_sidebar_left))
            .on_action(cx.listener(Self::handle_dock_sidebar_right))
            .on_action(cx.listener(Self::handle_trash_item))
            .on_action(cx.listener(Self::handle_open_environment_variables))
            .child(self.render_titlebar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .when(
                        AppSettings::global(cx).panel.project_panel.sidebar_dock
                            == SidebarDock::Right,
                        |this| {
                            this.child(self.tab_manager.clone())
                                .child(self.project_panel.clone())
                        },
                    )
                    .when(
                        AppSettings::global(cx).panel.project_panel.sidebar_dock
                            == SidebarDock::Left,
                        |this| {
                            this.child(self.project_panel.clone())
                                .child(self.tab_manager.clone())
                        },
                    ),
            )
            .child(self.render_footer(cx))
    }
}

fn open_settings_window(api_client: Entity<ApiClient>, cx: &mut App) {
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
                let window_handle = window.window_handle();
                let settings = cx.new(|cx| SettingsWindow::new(window, cx));
                api_client.update(cx, |client, cx| {
                    client.settings_window = Some((settings.downgrade(), window_handle));
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
        let _ = fs::ensure_config_dir();
        cx.set_global::<AppSettings>(AppSettings::get());
        AppSettings::global(cx).save();

        let theme_name = SharedString::from(AppSettings::global(cx).theme.name.clone());
        for theme_file in ["themes/one.json", "themes/ayu.json"] {
            if let Some(file) = Assets::get(theme_file) {
                if let Ok(content) = std::str::from_utf8(file.data.as_ref()) {
                    let _ = ThemeRegistry::global_mut(cx).load_themes_from_str(content);
                }
            }
        }
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            let mode = theme.mode;
            let t = Theme::global_mut(cx);
            if mode.is_dark() {
                t.dark_theme = theme.clone();
            } else {
                t.light_theme = theme.clone();
            }
            Theme::change(mode, None, cx);
        }
        let settings = AppSettings::global(cx).clone();
        let theme = Theme::global_mut(cx);
        theme.font_family = settings.font.family.into();
        theme.font_size = px(settings.font.size);
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        let mut client = ApiClient::new(window, cx);
                        client.init(window, cx);
                        client
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
