mod actions;
pub mod assets;
mod auth;
mod body;
mod config_fs;
mod env_fs;
mod env_panel;
mod env_playground;
mod footer;
mod headers;
mod helpers;
mod http_client;
mod http_request;
mod http_response;
mod icons;
mod playground;
mod project_panel;
mod query_params;
mod request_fs;
mod request_playground;
mod response_panel;
mod settings_panel;
mod settings_window;
mod stress_engine;
mod stress_testing;
mod tab;
mod tab_manager;
mod welcome;
use std::rc::Rc;

use crate::actions::{
    CopyEnvironmentVariables, CopySettings, DockEnvPanelLeft, DockEnvPanelRight, DockSidebarLeft,
    DockSidebarRight, OpenEnvironmentVariables, OpenSettings, QuitArc, ThemeChange,
};
use crate::assets::Assets;
use crate::config_fs::ConfigFileSystem;
use crate::env_fs::EnvFileSystem;
use crate::footer::{Footer, FooterEvent};
use crate::helpers::{get_active_theme, get_theme_config, get_themes};
use crate::icons::IconName;
use crate::project_panel::{DirTree, ProjectPanel};
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
    env_panel: Entity<env_panel::EnvPanel>,
    workspace_palette: Entity<CommandState>,
    workspace_palette_open: bool,
    environment_pallete_open: bool,
    environment_pallete: Entity<CommandState>,
    active_environment: String,
    workspaces: Vec<(String, String)>,
    selected_workspace: Option<usize>,
    settings_window: Option<(WeakEntity<SettingsWindow>, AnyWindowHandle)>,
    welcome: Entity<WelcomeScreen>,
    theme: Entity<CommandState>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = cx.new(|cx| ProjectPanel::new(window, cx));
        let env_panel = cx.new(|cx| env_panel::EnvPanel::new(window, cx));

        let workspace_palette = cx.new(|cx| CommandState::new(window, cx));
        let environment_pallete = cx.new(|cx| CommandState::new(window, cx));

        let tab_manager = cx.new(|cx| {
            tab_manager::TabManager::new(window, cx, project_panel.clone(), env_panel.clone())
        });
        let footer = cx.new(|cx| Footer::new(window, cx));
        let welcome = cx.new(|cx| WelcomeScreen::new(window, cx));

        let theme_switcher = cx.new(|cx| CommandState::new(window, cx));

        Self {
            project_panel,
            tab_manager,
            footer,
            env_panel,
            workspace_palette,
            workspace_palette_open: false,
            environment_pallete_open: false,
            environment_pallete,
            active_environment: "Local".to_string(),
            workspaces: Vec::new(),
            selected_workspace: None,
            settings_window: None,
            welcome,
            theme: theme_switcher,
        }
    }

    fn init(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.start_walkdir(window, cx);
        self.footer_event_handler(window, cx);
        self.init_footer_state(cx);
    }

    fn init_footer_state(&mut self, cx: &mut Context<Self>) {
        let pp_dock = AppSettings::global(cx).panel.project_panel.sidebar_dock;
        let ep_dock = AppSettings::global(cx).panel.env_panel.sidebar_dock;
        self.footer
            .update(cx, |f, cx| f.set_project_panel_dock(pp_dock, cx));
        self.footer
            .update(cx, |f, cx| f.set_env_panel_dock(ep_dock, cx));
    }

    fn switch_workspace_to(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((name, path)) = self.workspaces.get(ix).cloned() else {
            return;
        };
        self.active_environment = EnvFileSystem::validate_active_environment();
        self.selected_workspace = Some(ix);
        ConfigFileSystem::save_workspace_config(&name, &path);
        self.workspace_palette.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(ix)), window, cx);
        });
        self.tab_manager.update(cx, |tb, cx| tb.reset(window, cx));
        self.env_panel.update(cx, |ep, cx| ep.refresh(cx));

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
        self.selected_workspace = None;

        if let Some((name, path)) = ConfigFileSystem::read_workspace_config() {
            if let Some(ix) = self
                .workspaces
                .iter()
                .position(|(n, p)| *n == name && *p == path)
            {
                self.switch_workspace_to(ix, window, cx);
                self.workspace_palette.update(cx, |state, cx| {
                    state.set_selected_index(Some(IndexPath::new(ix)), window, cx);
                });
                return;
            }
        }

        let tab_manager = self.tab_manager.clone();
        let welcome = self.welcome.clone();
        tab_manager.update(cx, |tm, cx| {
            tm.open_welcome_tab(window, cx, welcome);
        });
    }

    fn footer_event_handler(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(&self.footer, window, {
            let tab_manager = self.tab_manager.clone();
            move |this: &mut Self, _, event, _window, cx| match event {
                FooterEvent::ToggleResponse => {
                    tab_manager.update(cx, |tm, cx| {
                        tm.toggle_active_response(cx);
                        if let Some(content) = tm.active_playground(cx) {
                            if let Some(panel) = content.response_panel(cx) {
                                let collapsed = !panel.read(cx).is_shown();
                                this.footer.update(cx, |f, cx| {
                                    f.set_response_collapsed(collapsed, cx);
                                });
                            }
                        }
                    });
                }
                FooterEvent::ToggleProjectPanel => {
                    this.project_panel.update(cx, |pp, cx| {
                        let collapsed = !pp.is_collapsed();
                        pp.set_collapsed(collapsed, cx);
                    });
                    let collapsed = this.project_panel.read(cx).is_collapsed();
                    this.footer
                        .update(cx, |f, cx| f.set_project_panel_collapsed(collapsed, cx));
                }
                FooterEvent::ToggleEnvPanel => {
                    this.env_panel.update(cx, |ep, cx| {
                        let collapsed = !ep.is_collapsed();
                        ep.set_collapsed(collapsed, cx);
                    });
                    let collapsed = this.env_panel.read(cx).is_collapsed();
                    this.footer
                        .update(cx, |f, cx| f.set_env_panel_collapsed(collapsed, cx));
                }
            }
        })
        .detach();

        cx.observe(&self.tab_manager, |this, _, cx| {
            let show_toggle = this
                .tab_manager
                .read(cx)
                .active_playground(cx)
                .as_ref()
                .and_then(|p| p.response_panel(cx))
                .is_some();
            this.footer
                .update(cx, |f, cx| f.set_show_toggle(show_toggle, cx));

            let response_collapsed = this
                .tab_manager
                .read(cx)
                .active_playground(cx)
                .as_ref()
                .and_then(|p| p.response_panel(cx))
                .map(|panel| !panel.read(cx).is_shown())
                .unwrap_or(true);
            this.footer
                .update(cx, |f, cx| f.set_response_collapsed(response_collapsed, cx));
        })
        .detach();
    }

    fn handle_dock_sidebar_left(
        &mut self,
        _: &DockSidebarLeft,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        AppSettings::global_mut(cx).panel.project_panel.sidebar_dock = SidebarDock::Left;
        AppSettings::global_mut(cx).save();
        self.footer
            .update(cx, |f, cx| f.set_project_panel_dock(SidebarDock::Left, cx));
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
        self.footer
            .update(cx, |f, cx| f.set_project_panel_dock(SidebarDock::Right, cx));
        cx.refresh_windows();
    }

    fn handle_dock_env_panel_left(
        &mut self,
        _: &DockEnvPanelLeft,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        AppSettings::global_mut(cx).panel.env_panel.sidebar_dock = SidebarDock::Left;
        AppSettings::global_mut(cx).save();
        self.footer
            .update(cx, |f, cx| f.set_env_panel_dock(SidebarDock::Left, cx));
        cx.refresh_windows();
    }

    fn handle_dock_env_panel_right(
        &mut self,
        _: &DockEnvPanelRight,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        AppSettings::global_mut(cx).panel.env_panel.sidebar_dock = SidebarDock::Right;
        AppSettings::global_mut(cx).save();
        self.footer
            .update(cx, |f, cx| f.set_env_panel_dock(SidebarDock::Right, cx));
        cx.refresh_windows();
    }

    fn render_footer(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        self.footer.clone()
    }

    fn handle_open_settings(
        &mut self,
        _: &OpenSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((sw, aw)) = self.settings_window.clone() {
            if sw.upgrade().is_some()
                && cx
                    .update_window(aw, |_, window, _cx| {
                        window.activate_window();
                    })
                    .is_ok()
            {
                return;
            }
        }
        open_settings_window(cx.entity(), cx);
    }

    fn handle_quit_arc(&mut self, _: &QuitArc, _window: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn handle_copy_settings(
        &mut self,
        _: &CopySettings,
        _window: &mut Window,
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
            let first_env = tm.env_names(cx).first().cloned();
            if let Some(name) = first_env {
                tm.open_env_tab(name, window, cx);
            }
        });
    }

    fn handle_copy_environment_variables(
        &mut self,
        _: &CopyEnvironmentVariables,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let content = EnvFileSystem::read_environment_variables();
        cx.write_to_clipboard(ClipboardItem::new_string(content.trim().to_string()));
    }

    fn handle_theme_change(
        &mut self,
        _: &ThemeChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = self.theme.clone();
        let committed_theme = get_active_theme(cx).to_string();
        let themes = Rc::new(
            get_themes(cx)
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
        );
        let items: Vec<CommandItem> = themes
            .iter()
            .map(|name| {
                CommandItem::new()
                    .label(name.as_ref())
                    .checked(name.as_ref() == committed_theme)
            })
            .collect();
        window.open_dialog(cx, move |dialog, _, _cx| {
            let state = state.clone();
            let items = items.clone();
            let themes = themes.clone();
            let cancel_committed = committed_theme.clone();
            let preview = |name: &str, window: &mut Window, cx: &mut App| {
                let name = SharedString::from(name);
                if let Some(theme_config) = get_theme_config(cx, &name) {
                    let mode = theme_config.mode;
                    let t = Theme::global_mut(cx);
                    if mode.is_dark() {
                        t.dark_theme = theme_config.clone();
                    } else {
                        t.light_theme = theme_config.clone();
                    }
                    Theme::change(mode, Some(window), cx);
                    let app_settings = AppSettings::global(cx).clone();
                    let t = Theme::global_mut(cx);
                    t.font_family = app_settings.font.family.clone().into();
                    t.font_size = px(app_settings.font.size);
                    window.refresh();
                }
            };
            let cancel_restore = cancel_committed.clone();
            dialog
                .close_button(false)
                .overlay_closable(true)
                .overlay(true)
                .p_0()
                .on_cancel(move |_, window, cx| {
                    let restore = cancel_restore.clone();
                    window.defer(cx, move |window, cx| {
                        preview(&restore, window, cx);
                    });
                    true
                })
                .content(move |content, _, _| {
                    let select_themes = themes.clone();
                    let confirm_themes = themes.clone();
                    let preview = preview;
                    content.child(
                        Command::new(&state)
                            .bordered(false)
                            .placeholder("Select Theme...")
                            .items(items.clone())
                            .on_select(move |index, window, cx| {
                                if let Some(name) = select_themes.get(index.row) {
                                    preview(name.as_ref(), window, cx);
                                }
                            })
                            .on_confirm(move |index, window, cx| {
                                if let Some(name) = confirm_themes.get(index.row) {
                                    preview(name.as_ref(), window, cx);
                                    AppSettings::global_mut(cx).theme.name = name.to_string();
                                    if let Some(theme_config) = get_theme_config(cx, name) {
                                        AppSettings::global_mut(cx).theme.mode =
                                            theme_config.mode.name().to_string();
                                    }
                                    AppSettings::global_mut(cx).save();
                                }
                                window.close_dialog(cx);
                            }),
                    )
                })
        });
    }

    fn render_titlebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = self.settings_window.clone();
        let this = cx.entity();
        let workspace_palette = self.workspace_palette.clone();
        let env_palette = self.environment_pallete.clone();
        let env_panel = self.env_panel.clone();
        let workspace_name = self
            .selected_workspace
            .and_then(|ix| self.workspaces.get(ix))
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "no workspace".to_string());
        let active_env = self.active_environment.clone();

        TitleBar::new()
            .h(px(32.))
            .bg(cx.theme().title_bar)
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
                                    .menu("Copy Environment Variables", Box::new(actions::CopyEnvironmentVariables))
                                    .separator()
                                    .menu("Select Theme...", Box::new(actions::ThemeChange))
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
                                            move |_, _window, _cx| {
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
                                                            let path = match ConfigFileSystem::create_workspace(
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
                    )
                    .child({
                        let this = this.clone();
                        let palette = env_palette.clone();
                        let ep = env_panel.clone();
                        Popover::new("environment-picker")
                            .open(self.environment_pallete_open)
                            .on_open_change({
                                let this = this.clone();
                                let palette = palette.clone();
                                move |is_open, window, cx| {
                                    if *is_open {
                                        palette.update(cx, |palette, cx| {
                                            palette.set_query("", window, cx);
                                        });
                                    }
                                    this.update(cx, |this, cx| {
                                        this.environment_pallete_open = *is_open;
                                        cx.notify();
                                    });
                                }
                            })
                            .trigger(
                                Button::new("env-trigger")
                                    .ghost()
                                    .small()
                                    .label(active_env.clone())
                                    .tooltip("Switch Environment"),
                            )
                            .content(move |_, _, cx| {
                                let client = this.read(cx);
                                let envs = ep.read(cx).envs.clone();
                                let active = client.active_environment.clone();
                                let items: Vec<CommandItem> = envs
                                    .iter()
                                    .enumerate()
                                    .map(|(_i, name)| {
                                        CommandItem::new()
                                            .label(name.clone())
                                            .icon(IconName::Variable)
                                            .checked(*name == active)
                                    })
                                    .collect();
                                Command::new(&palette)
                                    .bordered(false)
                                    .placeholder("Search or switch environment")
                                    .w(px(260.))
                                    .items(items)
                                    .separator()
                                    .footer({
                                        let this = this.clone();
                                        let palette = palette.clone();
                                        let ep = ep.clone();
                                        move |_, _, _| {
                                            Button::new("add-env")
                                                .ghost()
                                                .label("Add Environment")
                                                .icon(IconName::Plus)
                                                .w_full()
                                                .justify_start()
                                                .on_click({
                                                    let this = this.clone();
                                                    let palette = palette.clone();
                                                    let ep = ep.clone();
                                                    move |_, _window, cx| {
                                                        let name = palette
                                                            .read(cx)
                                                            .query(cx)
                                                            .to_string();
                                                        let name = name.trim().to_string();
                                                        if name.is_empty() {
                                                            return;
                                                        }
                                                        let mut envs: Vec<
                                                            crate::env_playground::Environment,
                                                        > = serde_json::from_str(
                                                            &EnvFileSystem::read_environment_variables(),
                                                        )
                                                        .unwrap_or_default();
                                                        if envs.iter().any(|e| e.name == name) {
                                                            return;
                                                        }
                                                        envs.push(crate::env_playground::Environment {
                                                            name: name.clone(),
                                                            variables: Vec::new(),
                                                        });
                                                        let content = serde_json::to_string_pretty(
                                                            &envs,
                                                        )
                                                        .unwrap_or_default();
                                                        let _ = EnvFileSystem::save_environment_variables(&content);
                                                        ep.update(cx, |panel, cx| panel.refresh(cx));
                                                        this.update(cx, |this, cx| {
                                                            this.active_environment = name;
                                                            cx.notify();
                                                        });
                                                    }
                                                })
                                                .into_any_element()
                                        }
                                    })
                                    .on_confirm({
                                        let this = this.clone();
                                        let ep = ep.clone();
                                        move |index, _window, cx| {
                                            let envs = ep.read(cx).envs.clone();
                                            if let Some(name) = envs.get(index.row) {
                                                let name = name.clone();
                                                EnvFileSystem::save_active_environment(&name);
                                                ep.update(cx, |panel, cx| panel.refresh(cx));
                                                this.update(cx, |this, cx| {
                                                    this.active_environment = name;
                                                    this.environment_pallete_open = false;
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                    .into_any_element()
                            })
                    }),
            )
    }
}

impl Render for ApiClient {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(cx.listener(Self::handle_copy_settings))
            .on_action(cx.listener(Self::handle_quit_arc))
            .on_action(cx.listener(Self::handle_open_settings))
            .on_action(cx.listener(Self::handle_dock_sidebar_left))
            .on_action(cx.listener(Self::handle_dock_sidebar_right))
            .on_action(cx.listener(Self::handle_dock_env_panel_left))
            .on_action(cx.listener(Self::handle_dock_env_panel_right))
            .on_action(cx.listener(Self::handle_open_environment_variables))
            .on_action(cx.listener(Self::handle_copy_environment_variables))
            .on_action(cx.listener(Self::handle_theme_change))
            .child(self.render_titlebar(cx))
            .child({
                let pp = self.project_panel.clone();
                let tm = self.tab_manager.clone();
                let ep = self.env_panel.clone();

                let pp_left =
                    AppSettings::global(cx).panel.project_panel.sidebar_dock == SidebarDock::Left;
                let ep_left =
                    AppSettings::global(cx).panel.env_panel.sidebar_dock == SidebarDock::Left;

                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .when(pp_left, |this| this.child(pp.clone()))
                    .when(ep_left, |this| this.child(ep.clone()))
                    .child(tm)
                    .when(!ep_left, |this| this.child(ep.clone()))
                    .when(!pp_left, |this| this.child(pp.clone()))
            })
            .child(self.render_footer(cx))
            .children(dialog_layer)
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
        let _ = ConfigFileSystem::init_setup();

        for font_file in [
            "fonts/lilex/Lilex-Regular.ttf",
            "fonts/lilex/Lilex-Bold.ttf",
            "fonts/lilex/Lilex-Italic.ttf",
            "fonts/lilex/Lilex-BoldItalic.ttf",
            "fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf",
            "fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf",
            "fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf",
            "fonts/ibm-plex-sans/IBMPlexSans-SemiBoldItalic.ttf",
        ] {
            if let Some(file) = Assets::get(font_file) {
                let _ = cx.text_system().add_fonts(vec![file.data]);
            }
        }
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
        theme.mono_font_family = ".ZedMono".into();
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
