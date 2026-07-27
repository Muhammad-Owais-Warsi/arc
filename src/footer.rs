use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::{ActiveTheme, IconName, IndexPath, Sizable, Theme, ThemeRegistry};

pub struct Footer {
    theme: Entity<SelectState<Vec<SharedString>>>,
    show_toggle: bool,
}

#[derive(Clone, Debug)]
pub enum FooterEvent {
    ToggleResponse,
}

impl EventEmitter<FooterEvent> for Footer {}

impl Footer {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let themes: Vec<SharedString> =
            ThemeRegistry::global(cx).themes().keys().cloned().collect();
        let default_theme = SharedString::from("Ayu Dark");
        let default_idx = themes.iter().position(|t| *t == default_theme).unwrap_or(0);

        let theme = cx.new(|cx| {
            SelectState::new(
                themes,
                Some(IndexPath {
                    section: 0,
                    row: default_idx,
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
                    let t = Theme::global_mut(cx);
                    if mode.is_dark() {
                        t.dark_theme = theme_config;
                    } else {
                        t.light_theme = theme_config;
                    }
                    Theme::change(mode, None, cx);
                    cx.refresh_windows();
                }
            }
        })
        .detach();

        Self {
            theme,
            show_toggle: false,
        }
    }

    pub fn set_show_toggle(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_toggle = show;
        cx.notify();
    }
}

impl Render for Footer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .h(px(50.0))
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .flex()
            .items_center()
            .px(px(16.0))
            .when(self.show_toggle, |this| {
                this.child(
                    Button::new("toggle-response")
                        .ghost()
                        .small()
                        .icon(IconName::PanelBottom)
                        .tooltip("Response")
                        .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                            cx.emit(FooterEvent::ToggleResponse);
                        })),
                )
            })
            .child(div().flex_1())
            .child(
                div()
                    .w(px(140.0))
                    .child(Select::new(&self.theme).appearance(false)),
            )
    }
}
