use crate::dock::tabs::Playground;
use crate::theme::{apply_theme, get_active_theme, get_theme_config, theme_families};
use crate::ui::method_tag;
use crate::ui::theme_preview::theme_preview_tile;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, StyledExt, h_flex, v_flex};
use gpui_kit::img;
use gpui_kit::*;

pub struct WelcomeScreen {
    focus: FocusHandle,
}

impl WelcomeScreen {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
        }
    }

    /// One card per theme family (One / Ayu / Gruvbox), showing the variant
    /// that matches the current mode. Picking one applies and persists it.
    fn render_theme_families(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let active = get_active_theme(cx);
        let cards: Vec<AnyElement> = theme_families(cx)
            .into_iter()
            .filter_map(|(family, theme_name)| {
                let config = get_theme_config(cx, &theme_name)?;
                let selected = theme_name == active;
                let id = theme_name.to_string();
                Some(
                    v_flex()
                        .id(SharedString::from(format!("welcome-theme-{id}")))
                        .gap(px(6.))
                        .flex_none()
                        .items_center()
                        .cursor_pointer()
                        .child(theme_preview_tile(theme_name.clone(), config, selected, cx))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(family),
                        )
                        .on_click(cx.listener(move |_this, _, _, cx| {
                            apply_theme(theme_name.as_ref(), cx);
                        }))
                        .into_any_element(),
                )
            })
            .collect();

        v_flex()
            .gap(px(10.))
            .items_start()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child("Theme"),
            )
            .child(h_flex().gap(px(16.)).items_start().children(cards))
            .into_any_element()
    }
}

impl Panel for WelcomeScreen {
    fn panel_name(&self) -> &'static str {
        "welcome"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for WelcomeScreen {}

impl Focusable for WelcomeScreen {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Playground for WelcomeScreen {
    fn tab_label(&self, _cx: &App) -> SharedString {
        "Welcome".into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(method_tag("WELCOME").into_any_element())
    }
}

impl Render for WelcomeScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .overflow_y_scrollbar()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(32.))
            .px(px(24.))
            .py(px(32.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .child(img("logo/arc.svg").size(px(64.)).rounded(px(12.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_0()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_semibold()
                                    .text_center()
                                    .text_color(cx.theme().foreground)
                                    .child("Welcome to Arc"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .italic()
                                    .text_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("API client built for speed"),
                            ),
                    ),
            )
            .child(self.render_theme_families(cx))
    }
}
