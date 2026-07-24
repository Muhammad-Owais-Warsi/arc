mod actions;
mod fs;
mod headers;
mod helpers;
mod http;
mod project_panel;
mod query_params;
mod tabs;

use crate::actions::{CreateFile, RenameFile};
use gpui::*;
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::{Theme, *};
use std::path::PathBuf;

pub(crate) struct ApiClient {
    pub(crate) project_panel: Entity<project_panel::ProjectPanel>,
    pub(crate) tab_manager: Entity<tabs::TabManager>,
    pub(crate) theme: Entity<SelectState<Vec<SharedString>>>,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>, default_theme: SharedString) -> Self {
        let themes: Vec<SharedString> =
            ThemeRegistry::global(cx).themes().keys().cloned().collect();
        let default_theme_idx = themes.iter().position(|t| *t == default_theme).unwrap_or(0);

        let theme = cx.new(|cx| {
            SelectState::new(
                themes,
                Some(IndexPath {
                    section: 0,
                    row: default_theme_idx,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        cx.subscribe_in(&theme, window, |_, _, event, _window, cx| {
            if let SelectEvent::Confirm(Some(name)) = event {
                let registry = ThemeRegistry::global(cx);
                if let Some(theme_config) = registry.themes().get(name).cloned() {
                    let mode = theme_config.mode;
                    let theme = Theme::global_mut(cx);
                    if mode.is_dark() {
                        theme.dark_theme = theme_config;
                    } else {
                        theme.light_theme = theme_config;
                    }
                    Theme::change(mode, None, cx);
                    cx.refresh_windows();
                }
            }
        })
        .detach();

        let project_panel = project_panel::ProjectPanel::new(window, cx);
        let tab_manager = tabs::TabManager::new(window, cx, project_panel.clone(), theme.clone());

        Self {
            project_panel,
            tab_manager,
            theme,
        }
    }
}

impl ApiClient {
    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel.update(cx, |s, cx| s.handle_create_file(action, cx));
    }

    pub fn handle_rename(
        &mut self,
        action: &RenameFile,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_panel.update(cx, |s, cx| s.handle_rename(action, cx));
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_rename))
            .child(self.project_panel.clone())
            .child(self.tab_manager.clone())
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        let theme_name = SharedString::from("Ayu Dark");
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
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|view_cx| ApiClient::new(window, view_cx, default_theme));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
