mod actions;
mod fs;
mod helpers;
mod http;
mod key_value;
mod project_panel;
mod tabs;

use crate::actions::{CreateFile, RenameFile};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::popover::Popover;
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::{Theme, *};
use std::path::PathBuf;

pub(crate) struct ApiClient {
    pub(crate) project_panel: Entity<project_panel::ProjectPanel>,
    pub(crate) tab_manager: Entity<tabs::TabManager>,
    // pub(crate) theme: Entity<SelectState<Vec<SharedString>>>,
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
            // theme,
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
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    window_decorations: Some(WindowDecorations::Client),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|view_cx| ApiClient::new(window, view_cx, default_theme));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
