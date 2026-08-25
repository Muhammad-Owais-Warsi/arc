use crate::{
    config_fs::ConfigFileSystem,
    helpers::{get_active_theme, get_theme_config, get_themes},
    icons::IconName,
};
use gpui::{
    App, AppContext, Context, Entity, Global, IntoElement, ParentElement as _, Render,
    SharedString, Styled, Window, px,
};
use gpui_component::{
    Icon, IndexPath, Side, Sizable, Size, Theme,
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    group_box::GroupBoxVariant,
    searchable_list::{SearchableListItem, SearchableVec},
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct FontItem {
    family: String,
}

fn font_display_name(family: &str) -> SharedString {
    match family {
        "Lilex" => "ZedMono",
        "IBMPlexSans" => "ZedSans",
        other => other,
    }
    .into()
}

impl SearchableListItem for FontItem {
    type Value = String;

    fn title(&self) -> SharedString {
        font_display_name(&self.family)
    }

    fn value(&self) -> &Self::Value {
        &self.family
    }
}

type FontSelect = ComboboxState<SearchableVec<FontItem>>;

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
            name: "One Dark".into(),
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
            family: "Lilex".into(),
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
pub struct EnvPanelSettings {
    pub sidebar_dock: SidebarDock,
}

impl Default for EnvPanelSettings {
    fn default() -> Self {
        Self {
            sidebar_dock: SidebarDock::Right,
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
    pub env_panel: EnvPanelSettings,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            project_panel: ProjectPanelSettings::default(),
            env_panel: EnvPanelSettings::default(),
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
        let content = ConfigFileSystem::read_settings();
        serde_json::from_str(content.as_str()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = ConfigFileSystem::save_settings(&content);
        }
    }
}

pub struct SettingsPanel {
    font_state: Option<Entity<FontSelect>>,
}

impl SettingsPanel {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self { font_state: None }
    }

    fn ensure_font_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.font_state.is_some() {
            return;
        }

        let fonts: Vec<FontItem> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .map(|f| FontItem { family: f })
            .collect();
        let entity: Entity<FontSelect> = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(fonts),
                vec![IndexPath::default()],
                window,
                cx,
            )
            .searchable(true)
        });

        cx.subscribe_in(
            &entity,
            window,
            |_this: &mut SettingsPanel, _, event: &ComboboxEvent<SearchableVec<FontItem>>, _, cx| {
                if let ComboboxEvent::Confirm(selected) = event {
                    if let Some(val) = selected.first() {
                        let val = val.clone();
                        Theme::global_mut(cx).font_family = val.clone().into();
                        AppSettings::global_mut(cx).font.family = val.clone();
                        AppSettings::global_mut(cx).save();
                        cx.refresh_windows();
                    }
                }
            },
        )
        .detach();

        self.font_state = Some(entity);
    }

    fn appearance_settings(
        font_state: Entity<FontSelect>,
        cx: &Context<Self>,
    ) -> Vec<SettingGroup> {
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
                                let app_settings = AppSettings::global(cx).clone();
                                let t = Theme::global_mut(cx);

                                // theme change resets the font, re-applying it here
                                t.font_family = app_settings.font.family.clone().into();
                                t.font_size = px(app_settings.font.size);
                                AppSettings::global_mut(cx).theme.name = name.to_string();
                                AppSettings::global_mut(cx).save();
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
                    SettingField::<SharedString>::render(move |_options, _window, _cx| {
                        Combobox::new(&font_state)
                            .placeholder("Search and select a font")
                            .search_placeholder("Search fonts...")
                            // .with_size(Size::Medium)
                            .w(px(240.))
                    }),
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

    fn panels_settings() -> SettingPage {
        SettingPage::new("Panels")
            .resettable(true)
            .icon(Icon::new(IconName::PanelLeftOpen))
            .groups(vec![
                SettingGroup::new()
                    .title("Project Panel")
                    .item(
                        SettingItem::new(
                            "Dock Position",
                            SettingField::<SharedString>::dropdown(
                                vec![
                                    ("left".into(), "Dock Left".into()),
                                    ("right".into(), "Dock Right".into()),
                                ],
                                |cx: &App| {
                                    let dock =
                                        AppSettings::global(cx).panel.project_panel.sidebar_dock;
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
                SettingGroup::new()
                    .title("Environment Panel")
                    .item(
                        SettingItem::new(
                            "Dock Position",
                            SettingField::<SharedString>::dropdown(
                                vec![
                                    ("left".into(), "Dock Left".into()),
                                    ("right".into(), "Dock Right".into()),
                                ],
                                |cx: &App| {
                                    let dock =
                                        AppSettings::global(cx).panel.env_panel.sidebar_dock;
                                    SharedString::from(if dock == SidebarDock::Right {
                                        "right"
                                    } else {
                                        "left"
                                    })
                                },
                                |val: SharedString, cx: &mut App| {
                                    AppSettings::global_mut(cx).panel.env_panel.sidebar_dock =
                                        if val == "right" {
                                            SidebarDock::Right
                                        } else {
                                            SidebarDock::Left
                                        };
                                    AppSettings::global_mut(cx).save();
                                    cx.refresh_windows();
                                },
                            )
                            .default_value("right"),
                        )
                        .description("Dock the environment sidebar on the left or right."),
                    ),
            ])
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

    fn setting_pages(
        &self,
        font_state: &Entity<FontSelect>,
        cx: &Context<Self>,
    ) -> Vec<SettingPage> {
        vec![
            SettingPage::new("General")
                .resettable(true)
                .default_open(true)
                .icon(Icon::new(IconName::SlidersHorizontal))
                .groups(Self::appearance_settings(font_state.clone(), cx)),
            Self::panels_settings(),
            Self::request_playground_settings(),
            SettingPage::new("About")
                .icon(Icon::new(IconName::Info))
                .group(
                    SettingGroup::new().item(SettingItem::render(|_options, _, _cx| {
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

        self.ensure_font_state(window, cx);
        let font_state = self.font_state.as_ref().unwrap().clone();
        let active_font = Theme::global(cx).font_family.to_string();
        font_state.update(cx, |s, cx| {
            s.set_selected_values(&[active_font], window, cx);
        });

        Settings::new("arc-settings")
            .with_size(Size::default())
            .with_group_variant(GroupBoxVariant::Outline)
            .header_style(&header_style)
            .pages(self.setting_pages(&font_state, cx))
    }
}
