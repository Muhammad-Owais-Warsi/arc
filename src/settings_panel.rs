use crate::{
    fs,
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
use serde::{Deserialize, Serialize};

use crate::env::EnvironmentStore;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    pub name: String,
    pub mode: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            name: "Ayu Dark".into(),
            mode: "dark".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: ".ZedSans".into(),
            size: 16.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectPanelSettings {
    pub sidebar_dock: SidebarDock,
}

impl Default for ProjectPanelSettings {
    fn default() -> Self {
        Self {
            sidebar_dock: SidebarDock::Left,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestPlaygroundSettings {
    pub save_on_close: bool,
}

impl Default for RequestPlaygroundSettings {
    fn default() -> Self {
        Self {
            save_on_close: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    pub project_panel: ProjectPanelSettings,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            project_panel: ProjectPanelSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaygroundSettings {
    pub request_playground: RequestPlaygroundSettings,
}

impl Default for PlaygroundSettings {
    fn default() -> Self {
        Self {
            request_playground: RequestPlaygroundSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: ThemeSettings,
    pub font: FontSettings,
    pub panel: PanelSettings,
    pub playground: PlaygroundSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSettings::default(),
            font: FontSettings::default(),
            panel: PanelSettings::default(),
            playground: PlaygroundSettings::default(),
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

    pub fn get() -> Self {
        let content = fs::get_settings();
        serde_json::from_str(content.as_str()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::save_settings(&content);
        }
    }
}

pub struct SettingsPanel;

impl SettingsPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self
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
                                AppSettings::global_mut(cx).theme.name = name.to_string();
                                AppSettings::global_mut(cx).save();
                                cx.refresh_windows();
                            }
                        },
                    )
                    .default_value("Ayu Dark"),
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
                            Theme::global_mut(cx).font_family = val.clone();
                            AppSettings::global_mut(cx).font.family = val.to_string();
                            AppSettings::global_mut(cx).save();
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
                            AppSettings::global_mut(cx).font.size = val as f32;
                            AppSettings::global_mut(cx).save();
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
                                let dock = AppSettings::global(cx).panel.project_panel.sidebar_dock;
                                SharedString::from(if dock == SidebarDock::Right {
                                    "right"
                                } else {
                                    "left"
                                })
                            },
                            |val: SharedString, cx: &mut App| {
                                AppSettings::global_mut(cx).panel.project_panel.sidebar_dock =
                                    if val == "right" {
                                        SidebarDock::Right
                                    } else {
                                        SidebarDock::Left
                                    };
                                AppSettings::global_mut(cx).save();
                                cx.refresh_windows();
                            },
                        )
                        .default_value("left"),
                    )
                    .description("Dock the file sidebar on the left or right."),
                ),
            )
    }

    fn request_playground_settings() -> SettingPage {
        SettingPage::new("Request Playground")
            .resettable(true)
            .icon(Icon::new(IconName::Send))
            .group(
                SettingGroup::new().item(
                    SettingItem::new(
                        "Save on Close",
                        SettingField::<bool>::switch(
                            |cx: &App| {
                                AppSettings::global(cx)
                                    .playground
                                    .request_playground
                                    .save_on_close
                            },
                            |val: bool, cx: &mut App| {
                                AppSettings::global_mut(cx)
                                    .playground
                                    .request_playground
                                    .save_on_close = val;
                                AppSettings::global_mut(cx).save();
                                cx.refresh_windows();
                            },
                        )
                        .default_value(false),
                    )
                    .description(
                        "Automatically save the request to its file when the tab is closed.",
                    ),
                ),
            )
    }

    fn setting_pages(&self, cx: &Context<Self>) -> Vec<SettingPage> {
        vec![
            SettingPage::new("General")
                .resettable(true)
                .default_open(true)
                .icon(Icon::new(IconName::SlidersHorizontal))
                .groups(Self::appearance_settings(cx)),
            Self::project_panel_settings(),
            Self::request_playground_settings(),
            SettingPage::new("About")
                .icon(Icon::new(IconName::Info))
                .group(
                    SettingGroup::new().item(SettingItem::render(|_options, _, cx| {
                        v_flex()
                            .gap_3()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Info))
                            .child("Arc is a minimal and GPU-rendered API client built for speed")
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
