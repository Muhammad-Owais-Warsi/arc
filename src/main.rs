mod actions;
pub mod assets;
mod auth;
mod body;
mod curl;
mod dock;

mod env_panel;
mod env_playground;
mod footer;
pub mod fs;
mod headers;
mod helpers;
mod http_client;
mod http_request;
mod http_response;
mod icons;
mod project_panel;
mod query_params;

mod request_playground;
mod response_panel;
mod settings_panel;
mod settings_window;
mod stress_engine;
mod stress_testing;
mod tab_manager;
mod titlebar;
mod welcome;
use std::rc::Rc;

use crate::actions::{
    CopySettings, DockEnvPanelLeft, DockEnvPanelRight, DockSidebarLeft, DockSidebarRight,
    OpenSettings, QuitArc, ThemeChange,
};
use crate::assets::Assets;
use crate::dock::shell::{DockShell, DockShellEvent, FixedPanel, Side};
use crate::dock::tabs::TabChromeRegistry;
use crate::footer::{Footer, FooterEvent};

use crate::helpers::{get_active_theme, get_theme_config, get_themes};

use crate::project_panel::{DirTree, ProjectPanel};
use crate::response_panel::ResponsePanel;
use crate::settings_panel::{AppSettings, SidebarDock};
use crate::settings_window::SettingsWindow;
use crate::tab_manager::TabManager;
use crate::titlebar::TitleBarView;
use crate::welcome::WelcomeScreen;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::{Theme, *};
use gpui_kit::*;

/// Stable ids for the two fixed sidebar panels. Kept as named constants so the
/// shell is addressed by symbol rather than scattered string literals.
const PROJECT_PANEL_ID: &str = "project-panel";
const ENV_PANEL_ID: &str = "environment-panel";

pub struct ApiClient {
    project_panel: Entity<project_panel::ProjectPanel>,
    footer: Entity<Footer>,
    pub(crate) env_panel: Entity<env_panel::EnvPanel>,
    shell: Entity<DockShell>,
    response: Entity<ResponsePanel>,
    tab_manager: Entity<TabManager>,
    titlebar: Entity<TitleBarView>,
    pub(crate) workspaces: Vec<(String, String)>,
    pub(crate) selected_workspace: Option<usize>,
    pub(crate) settings_window: Option<(WeakEntity<SettingsWindow>, AnyWindowHandle)>,
    theme: Entity<CommandState>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project_panel = cx.new(|cx| ProjectPanel::new(window, cx));
        let env_panel = cx.new(|cx| env_panel::EnvPanel::new(window, cx));
        let response = cx.new(|cx| ResponsePanel::new(window, cx));

        let workspace_palette = cx.new(|cx| CommandState::new(window, cx));
        let env_palette = cx.new(|cx| CommandState::new(window, cx));

        let mut registry = TabChromeRegistry::new();
        registry.register::<crate::request_playground::RequestPlayground>("request");
        registry.register::<crate::env_playground::EnvPlayground>("env");
        registry.register::<crate::stress_testing::StressTesting>("stress");
        registry.register::<crate::welcome::WelcomeScreen>("welcome");
        let shell = cx.new(|cx| DockShell::new(window, cx, Rc::new(registry), None));
        let footer = cx.new(|cx| Footer::new(window, cx));
        let welcome = cx.new(|cx| WelcomeScreen::new(window, cx));

        let theme_switcher = cx.new(|cx| CommandState::new(window, cx));

        let tab_manager = cx.new(|_| {
            TabManager::new(
                shell.clone(),
                project_panel.clone(),
                env_panel.clone(),
                response.clone(),
                welcome,
            )
        });

        let titlebar = cx.new(|_| TitleBarView::new(workspace_palette, env_palette));

        let this = Self {
            project_panel,
            footer,
            env_panel,
            shell,
            response,
            tab_manager,
            titlebar,
            workspaces: Vec::new(),
            selected_workspace: None,
            settings_window: None,
            theme: theme_switcher,
        };

        // Fixed side panels follow the saved dock settings. Both start
        // closed, matching the old sidebar defaults.
        let pp_dock = AppSettings::global(cx).panel.project_panel.sidebar_dock;
        let ep_dock = AppSettings::global(cx).panel.env_panel.sidebar_dock;
        this.shell.update(cx, |shell, cx| {
            shell.set_panel(
                match pp_dock {
                    SidebarDock::Left => Side::Left,
                    SidebarDock::Right => Side::Right,
                },
                FixedPanel::new(
                    PROJECT_PANEL_ID,
                    "Project",
                    this.project_panel.clone().into(),
                ),
                cx,
            );
            shell.set_panel(
                match ep_dock {
                    SidebarDock::Left => Side::Left,
                    SidebarDock::Right => Side::Right,
                },
                FixedPanel::new(ENV_PANEL_ID, "Environments", this.env_panel.clone().into()),
                cx,
            );
            shell.set_panel_open(PROJECT_PANEL_ID, false, cx);
            shell.set_panel_open(ENV_PANEL_ID, false, cx);
        });

        this
    }

    /// Install the strip [+] action. Deferred until the entity exists so
    /// the closure can upgrade a weak handle and route through the tracked
    /// tab helpers (dedup/activate stay correct for +-created tabs).
    /// (Tab prefix left alone for now — no bar_prefix installed.)
    pub fn install_add_tab(&mut self, cx: &mut Context<Self>) {
        use crate::dock::skin::AddTabAction;
        let weak = cx.weak_entity();
        let action: AddTabAction = std::rc::Rc::new(move |_area, target, window, cx| {
            weak.update(cx, |this, cx| {
                this.tab_manager.update(cx, |tabs, cx| {
                    tabs.open_untitled_request_in(target, window, cx);
                });
            })
            .ok();
        });
        self.shell.update(cx, |shell, cx| {
            shell.set_add_tab(Some(action), cx);
        });
    }

    fn init(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Wire the ApiClient weak handle into TitleBarView now that the entity exists.
        let weak = cx.weak_entity();
        self.titlebar.update(cx, |tb, _| tb.set_client(weak));
        self.tab_manager.update(cx, |tabs, cx| {
            tabs.seed_empty_center(window, cx);
            tabs.install_close_hook(cx);
            tabs.subscribe_dock_events(window, cx);
        });
        self.sync_footer_from_shell(cx);
        // Mirror response state into the footer whenever the response changes.
        cx.observe(&self.response, |this, _, cx| {
            this.sync_footer_from_shell(cx);
        })
        .detach();
        let shell = self.shell.clone();
        cx.subscribe_in(
            &shell,
            window,
            |this: &mut Self, _, _: &DockShellEvent, _, cx| {
                this.sync_footer_from_shell(cx);
                cx.notify();
            },
        )
        .detach();
        self.start_walkdir(window, cx);
        self.footer_event_handler(window, cx);
        self.init_footer_state(cx);
    }

    fn sync_footer_from_shell(&mut self, cx: &mut Context<Self>) {
        let vis = self
            .shell
            .read(cx)
            .visibility(PROJECT_PANEL_ID, ENV_PANEL_ID);
        let has_response = self.response.read(cx).has_response();
        self.footer.update(cx, |f, cx| {
            f.set_project_panel_collapsed(!vis.left_open, cx);
            f.set_env_panel_collapsed(!vis.right_open, cx);
            f.set_response_collapsed(!vis.aux_visible, cx);
            f.set_show_toggle(has_response, cx);
        });
    }

    fn init_footer_state(&mut self, cx: &mut Context<Self>) {
        let pp_dock = AppSettings::global(cx).panel.project_panel.sidebar_dock;
        let ep_dock = AppSettings::global(cx).panel.env_panel.sidebar_dock;
        self.footer
            .update(cx, |f, cx| f.set_project_panel_dock(pp_dock, cx));
        self.footer
            .update(cx, |f, cx| f.set_env_panel_dock(ep_dock, cx));
    }

    pub(crate) fn switch_workspace_to(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((name, path)) = self.workspaces.get(ix).cloned() else {
            return;
        };
        self.selected_workspace = Some(ix);
        fs::workspace::save(&name, &path);
        self.titlebar.update(cx, |tb, cx| {
            tb.sync_workspace_selection(ix, window, cx);
        });
        self.tab_manager
            .update(cx, |tabs, cx| tabs.reset_center_tabs(window, cx));
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

    /// Add a new workspace by name, create its directory, and switch to it.
    pub(crate) fn add_workspace(
        &mut self,
        name: String,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspaces.iter().any(|(n, _)| n == &name) {
            return;
        }
        let ix = self.workspaces.len();
        self.workspaces.push((name.clone(), path.clone()));
        self.project_panel.update(cx, |pp, cx| {
            pp.set_tree(
                name,
                path,
                DirTree {
                    root_ids: Vec::new(),
                    nodes: std::collections::HashMap::new(),
                },
                cx,
            );
        });
        self.switch_workspace_to(ix, window, cx);
    }

    fn start_walkdir(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspaces = ProjectPanel::list_workspace_dirs()
            .into_iter()
            .map(|(name, path)| (name, path.to_string_lossy().to_string()))
            .collect();
        self.selected_workspace = None;

        if let Some((name, path)) = fs::workspace::read() {
            if let Some(ix) = self
                .workspaces
                .iter()
                .position(|(n, p)| *n == name && *p == path)
            {
                self.switch_workspace_to(ix, window, cx);
                return;
            }
        }

        self.tab_manager
            .update(cx, |tabs, cx| tabs.open_welcome_tab(window, cx));
    }

    fn footer_event_handler(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.subscribe_in(&self.footer, window, {
            move |this: &mut Self, _, event, _window, cx| match event {
                FooterEvent::ToggleResponse => {
                    this.response.update(cx, |panel, cx| panel.toggle(cx));
                }
                FooterEvent::ToggleProjectPanel => {
                    this.shell.update(cx, |shell, cx| {
                        shell.toggle_panel(PROJECT_PANEL_ID, cx);
                    });
                }
                FooterEvent::ToggleEnvPanel => {
                    this.shell.update(cx, |shell, cx| {
                        shell.toggle_panel(ENV_PANEL_ID, cx);
                    });
                }
            }
        })
        .detach();
    }

    fn set_project_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        AppSettings::global_mut(cx).panel.project_panel.sidebar_dock = dock;
        AppSettings::global_mut(cx).save();
        self.footer
            .update(cx, |f, cx| f.set_project_panel_dock(dock, cx));
        self.shell.update(cx, |shell, cx| {
            shell.set_panel_side(
                PROJECT_PANEL_ID,
                match dock {
                    SidebarDock::Left => Side::Left,
                    SidebarDock::Right => Side::Right,
                },
                cx,
            );
        });
    }

    fn set_env_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        AppSettings::global_mut(cx).panel.env_panel.sidebar_dock = dock;
        AppSettings::global_mut(cx).save();
        self.footer
            .update(cx, |f, cx| f.set_env_panel_dock(dock, cx));
        self.shell.update(cx, |shell, cx| {
            shell.set_panel_side(
                ENV_PANEL_ID,
                match dock {
                    SidebarDock::Left => Side::Left,
                    SidebarDock::Right => Side::Right,
                },
                cx,
            );
        });
    }

    fn handle_dock_sidebar_left(
        &mut self,
        _: &DockSidebarLeft,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_project_dock(SidebarDock::Left, cx);
    }

    fn handle_dock_sidebar_right(
        &mut self,
        _: &DockSidebarRight,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_project_dock(SidebarDock::Right, cx);
    }

    fn handle_dock_env_panel_left(
        &mut self,
        _: &DockEnvPanelLeft,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_env_dock(SidebarDock::Left, cx);
    }

    fn handle_dock_env_panel_right(
        &mut self,
        _: &DockEnvPanelRight,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_env_dock(SidebarDock::Right, cx);
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
            .on_action(cx.listener(Self::handle_theme_change))
            .child(self.titlebar.clone())
            .child({
                let shell = self.shell.clone();

                div()
                    .flex_1()
                    .min_h(px(0.))
                    .w_full()
                    .overflow_hidden()
                    .child(shell)
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
    let app = gpui_kit::application().with_assets(Assets);
    app.run(move |cx| {
        gpui_kit::init(cx);
        let _ = fs::workspace::init();

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

        let theme_name = SharedString::from(AppSettings::global(cx).theme.name.clone());
        for theme_file in ["themes/one.json", "themes/ayu.json", "themes/gruvbox.json"] {
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
                    view.update(cx, |client, cx| client.install_add_tab(cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
