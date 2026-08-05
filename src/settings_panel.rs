use crate::icons::IconName;
use gpui::{
    App, Context, Entity, IntoElement, ParentElement as _, Render, SharedString, Styled, Window,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    Icon, Sizable, Size, Theme,
    group_box::GroupBoxVariant,
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};

use crate::env::EnvironmentStore;
use crate::themes_and_fonts::ThemesAndFonts;

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
                        ThemesAndFonts::get_themes(cx),
                        |cx: &App| ThemesAndFonts::get_active_theme(cx),
                        |name: SharedString, cx: &mut App| {
                            if let Some(theme_config) = ThemesAndFonts::get_theme_config(cx, &name)
                            {
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
                        ThemesAndFonts::get_fonts(cx),
                        |cx: &App| ThemesAndFonts::get_active_font(cx),
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

    fn setting_pages(&self, cx: &Context<Self>) -> Vec<SettingPage> {
        let store = self.store.clone();

        vec![
            SettingPage::new("General")
                .resettable(true)
                .default_open(true)
                .icon(Icon::new(IconName::Settings2))
                .groups(Self::appearance_settings(cx)),
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
