use gpui::{
    App, Context, Entity, Global, IntoElement, ParentElement as _, Render, SharedString, Styled,
    Window, prelude::FluentBuilder as _, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, Size, Theme, ThemeRegistry,
    group_box::GroupBoxVariant,
    h_flex,
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};

use crate::env::EnvironmentStore;

#[derive(Clone)]
pub struct AppSettings {
    pub resettable: bool,
    pub disabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            resettable: true,
            disabled: false,
        }
    }
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
    group_variant: GroupBoxVariant,
    size: Size,
}

impl SettingsPanel {
    pub fn new(store: Entity<EnvironmentStore>, cx: &mut Context<Self>) -> Self {
        cx.set_global::<AppSettings>(AppSettings::default());
        Self {
            store,
            group_variant: GroupBoxVariant::Outline,
            size: Size::default(),
        }
    }

    pub fn store(&self) -> Entity<EnvironmentStore> {
        self.store.clone()
    }

    fn theme_options(cx: &App) -> Vec<(SharedString, SharedString)> {
        ThemeRegistry::global(cx)
            .themes()
            .keys()
            .filter(|k| k.as_ref() != "Default Dark" && k.as_ref() != "Default Light")
            .cloned()
            .map(|k| (k.clone(), k.clone()))
            .collect()
    }

    fn font_options(cx: &App) -> Vec<(SharedString, SharedString)> {
        cx.text_system()
            .all_font_names()
            .into_iter()
            .map(|f| (f.clone().into(), f.into()))
            .collect()
    }

    fn setting_pages(&self, cx: &Context<Self>) -> Vec<SettingPage> {
        let default_settings = AppSettings::default();
        let resettable = AppSettings::global(cx).resettable;
        let disabled = AppSettings::global(cx).disabled;

        let store = self.store.clone();
        let env_names: Vec<(SharedString, SharedString)> = self
            .store
            .read(cx)
            .environments
            .iter()
            .map(|e| (e.name.clone().into(), e.name.clone().into()))
            .collect();

        let store_for_dropdown = store.clone();
        let set_env = store.clone();
        let env_group = SettingGroup::new()
            .title("Environment")
            .item(SettingItem::new(
                "Active Environment",
                SettingField::<SharedString>::dropdown(
                    env_names,
                    move |cx| {
                        set_env
                            .read(cx)
                            .active_name
                            .clone()
                            .unwrap_or_default()
                            .into()
                    },
                    move |name, cx| {
                        let name = name.to_string();
                        store_for_dropdown.update(cx, |s, cx| {
                            s.active_name = Some(name);
                            cx.notify();
                        });
                    },
                ),
            ));

        let env_group = if let Some(env) = self.store.read(cx).active() {
            let mut vars: Vec<(String, String)> = env
                .variables
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            vars.sort();
            vars.into_iter().fold(env_group, |group, (key, _value)| {
                let store_for_field = store.clone();
                let store_for_set = store.clone();
                let key_for_get = key.clone();
                let key_for_set = key.clone();
                group.item(SettingItem::new(
                    key.clone(),
                    SettingField::<SharedString>::input(
                        move |cx| {
                            store_for_field
                                .read(cx)
                                .active()
                                .and_then(|e| e.get(&key_for_get))
                                .unwrap_or_default()
                                .into()
                        },
                        move |value, cx| {
                            let value = value.to_string();
                            store_for_set.update(cx, |s, cx| {
                                if let Some(env) = s.active_mut() {
                                    env.variables.insert(key_for_set.clone(), value);
                                    cx.notify();
                                }
                            });
                        },
                    ),
                ))
            })
        } else {
            env_group
        };

        vec![
            SettingPage::new("General")
                .resettable(resettable)
                .default_open(true)
                .icon(Icon::new(IconName::Settings2))
                .groups(vec![
                    SettingGroup::new().title("Appearance").items(vec![
                        SettingItem::new(
                            "Theme",
                            SettingField::<SharedString>::dropdown(
                                Self::theme_options(cx),
                                |cx: &App| Theme::global(cx).theme_name().clone(),
                                |name: SharedString, cx: &mut App| {
                                    let registry = ThemeRegistry::global(cx);
                                    if let Some(theme_config) =
                                        registry.themes().get(&name).cloned()
                                    {
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
                                },
                            )
                            .default_value("One Dark"),
                        )
                        .description("Select the application theme.")
                        .disabled(disabled),
                    ]),
                    SettingGroup::new().title("Font").items(vec![
                        SettingItem::new(
                            "Font Family",
                            SettingField::<SharedString>::scrollable_dropdown(
                                Self::font_options(cx),
                                |cx: &App| Theme::global(cx).font_family.clone(),
                                |val: SharedString, cx: &mut App| {
                                    Theme::global_mut(cx).font_family = val;
                                    cx.refresh_windows();
                                },
                            )
                            .default_value(".ZedSans"),
                        )
                        .description("Select the font family.")
                        .disabled(disabled),
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
                        .disabled(disabled),
                    ]),
                ]),
            SettingPage::new("Environment")
                .resettable(resettable)
                .icon(Icon::new(IconName::Cpu))
                .group(env_group),
            SettingPage::new("About")
                .resettable(resettable)
                .icon(Icon::new(IconName::Info))
                .group(
                    SettingGroup::new().item(SettingItem::render(|_options, _, cx| {
                        v_flex()
                            .gap_3()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::GalleryVerticalEnd).size_16())
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

        Settings::new("app-settings")
            .with_size(self.size)
            .with_group_variant(self.group_variant)
            .header_style(&header_style)
            .pages(self.setting_pages(cx))
    }
}
