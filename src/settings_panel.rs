use crate::{
    helpers::{get_active_font, get_active_theme, get_fonts, get_theme_config, get_themes},
    icons::IconName,
};
use gpui::{
    App, Context, Entity, Global, IntoElement, ParentElement as _, Render, SharedString, Styled,
    Window, px,
};
use gpui_component::{
    Icon, Side, Sizable, Size, Theme,
    group_box::GroupBoxVariant,
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};

use crate::env::EnvironmentStore;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidebarDock {
    #[default]
    Left,
    Right,
}

impl SidebarDock {
    pub fn to_side(self) -> Side {
        match self {
            Self::Left => Side::Left,
            Self::Right => Side::Right,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppSettings {
    pub sidebar_dock: SidebarDock,
}

impl Global for AppSettings {}

impl AppSettings {
    pub fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }
}

pub struct SettingsPanel {
    store: Entity<EnvironmentStore>,
}

impl SettingsPanel {
    pub fn new(store: Entity<EnvironmentStore>, cx: &mut Context<Self>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> Entity<EnvironmentStore> {
        self.store.clone()
    }

    fn appearance_settings(cx: &Context<Self>) -> Vec<SettingGroup> {
        vec![
            SettingGroup::new().title("Appearance").items(vec![
                SettingItem::new(
                    "Theme",
                    SettingField::<SharedString>::dropdown(
                        get_themes(cx),
                        |cx: &App| get_active_theme(cx),
                        |name: SharedString, cx: &mut App| {
                            if let Some(theme_config) = get_theme_config(cx, &name) {
                                let mode = theme_config.mode;
                                let t = Theme::global_mut(cx);
                                if mode.is_dark() {
                                    t.dark_theme = theme_config.clone();
                                } else {
                                    t.light_theme = theme_config.clone();
                                }
                                Theme::change(mode, None, cx);
                                cx.refresh_windows();
                            }
                        },
                    )
                    .default_value("One Dark"),
                )
                .description("Select the application theme.")
                .disabled(false),
            ]),
            SettingGroup::new().title("Font").items(vec![
                SettingItem::new(
                    "Font Family",
                    SettingField::<SharedString>::scrollable_dropdown(
                        get_fonts(cx),
                        |cx: &App| get_active_font(cx),
                        |val: SharedString, cx: &mut App| {
                            Theme::global_mut(cx).font_family = val;
                            cx.refresh_windows();
                        },
                    )
                    .default_value(".ZedSans"),
                )
                .description("Select the font family.")
                .disabled(false),
                SettingItem::new(
                    "Font Size",
                    SettingField::number_input(
                        NumberFieldOptions {
                            min: 8.0,
                            max: 72.0,
                            ..Default::default()
                        },
                        |cx: &App| Theme::global(cx).font_size.as_f32() as f64,
                        |val: f64, cx: &mut App| {
                            Theme::global_mut(cx).font_size = px(val as f32);
                            cx.refresh_windows();
                        },
                    )
                    .default_value(16.0),
                )
                .description("Adjust the font size between 8 and 72.")
                .disabled(false),
            ]),
        ]
    }

    fn project_panel_settings() -> SettingPage {
        SettingPage::new("Project Panel")
            .resettable(true)
            .icon(Icon::new(IconName::PanelLeftOpen))
            .group(
                SettingGroup::new().item(
                    SettingItem::new(
                        "Dock Position",
                        SettingField::<SharedString>::dropdown(
                            vec![
                                ("left".into(), "Dock Left".into()),
                                ("right".into(), "Dock Right".into()),
                            ],
                            |cx: &App| {
                                let dock = AppSettings::global(cx).sidebar_dock;
                                SharedString::from(if dock == SidebarDock::Right {
                                    "right"
                                } else {
                                    "left"
                                })
                            },
                            |val: SharedString, cx: &mut App| {
                                AppSettings::global_mut(cx).sidebar_dock = if val == "right" {
                                    SidebarDock::Right
                                } else {
                                    SidebarDock::Left
                                };
                                cx.refresh_windows();
                            },
                        )
                        .default_value("left"),
                    )
                    .description("Dock the file sidebar on the left or right."),
                ),
            )
    }

    fn setting_pages(&self, cx: &Context<Self>) -> Vec<SettingPage> {
        let store = self.store.clone();

        vec![
            SettingPage::new("General")
                .resettable(true)
                .default_open(true)
                .icon(Icon::new(IconName::SlidersHorizontal))
                .groups(Self::appearance_settings(cx)),
            Self::project_panel_settings(),
            SettingPage::new("Environment")
                .resettable(true)
                .icon(Icon::new(IconName::Variable))
                .group(
                    SettingGroup::new().item(SettingItem::render(move |_, _, _| {
                        store.clone().into_any_element()
                    })),
                ),
            SettingPage::new("About")
                .resettable(true)
                .icon(Icon::new(IconName::Info))
                .group(
                    SettingGroup::new().item(SettingItem::render(|_options, _, cx| {
                        v_flex()
                            .gap_3()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Info))
                            .child("Arc API Client")
                            .into_any_element()
                    })),
                ),
        ]
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut header_style = gpui::StyleRefinement::default();
        header_style.flex_grow = Some(1.);
        header_style.flex_shrink = Some(1.);
        header_style.flex_basis = Some(gpui::relative(0.).into());

        Settings::new("arc-settings")
            .with_size(Size::default())
            .with_group_variant(GroupBoxVariant::Outline)
            .header_style(&header_style)
            .pages(self.setting_pages(cx))
    }
}
