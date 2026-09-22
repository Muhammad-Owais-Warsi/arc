use super::model::{AppSettings, SidebarDock};
use crate::{
    ApiClient,
    theme::{get_active_theme, get_theme_config, get_themes},
    ui::IconName,
};
use gpui_kit::component::{
    Icon, IndexPath, Sizable, Size, Theme,
    combobox::{Combobox, ComboboxEvent, ComboboxState},
    group_box::GroupBoxVariant,
    searchable_list::{SearchableListItem, SearchableVec},
    setting::{NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};
use gpui_kit::*;
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled, Window, px,
};

#[derive(Clone)]
struct FontItem {
    family: String,
}

impl FontItem {
    fn display_name(&self) -> SharedString {
        match self.family.as_str() {
            "Lilex" => "ZedMono".into(),
            "IBMPlexSans" => "ZedSans".into(),
            other => other.into(),
        }
    }
}

impl SearchableListItem for FontItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.display_name()
    }

    fn value(&self) -> &Self::Value {
        &self.family
    }
}

type FontSelect = ComboboxState<SearchableVec<FontItem>>;

pub struct SettingsPanel {
    font_state: Option<Entity<FontSelect>>,
    client: Option<WeakEntity<ApiClient>>,
}

impl SettingsPanel {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            font_state: None,
            client: None,
        }
    }

    pub fn set_client(&mut self, client: WeakEntity<ApiClient>) {
        self.client = Some(client);
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
            |_this: &mut SettingsPanel,
             _,
             event: &ComboboxEvent<SearchableVec<FontItem>>,
             _,
             cx| {
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

    fn panels_settings(&self) -> SettingPage {
        let project_client = self.client.clone();
        let env_client = self.client.clone();
        SettingPage::new("Panels")
            .resettable(true)
            .icon(Icon::new(IconName::PanelLeftOpen))
            .groups(vec![
                SettingGroup::new().title("File Panel").item(
                    SettingItem::new(
                        "Dock Position",
                        SettingField::<SharedString>::dropdown(
                            vec![
                                ("left".into(), "Dock Left".into()),
                                ("right".into(), "Dock Right".into()),
                            ],
                            |cx: &App| {
                                let dock = AppSettings::global(cx).panel.file_panel.sidebar_dock;
                                SharedString::from(if dock == SidebarDock::Right {
                                    "right"
                                } else {
                                    "left"
                                })
                            },
                            move |val: SharedString, cx: &mut App| {
                                let dock = if val == "right" {
                                    SidebarDock::Right
                                } else {
                                    SidebarDock::Left
                                };
                                match project_client.clone().and_then(|c| c.upgrade()) {
                                    Some(client) => client.update(cx, |c, cx| {
                                        c.set_file_dock(dock, cx);
                                    }),
                                    None => {
                                        AppSettings::global_mut(cx).panel.file_panel.sidebar_dock =
                                            dock;
                                        AppSettings::global_mut(cx).save();
                                    }
                                }
                            },
                        )
                        .default_value("left"),
                    )
                    .description("Dock the file sidebar on the left or right."),
                ),
                SettingGroup::new().title("Environment Panel").item(
                    SettingItem::new(
                        "Dock Position",
                        SettingField::<SharedString>::dropdown(
                            vec![
                                ("left".into(), "Dock Left".into()),
                                ("right".into(), "Dock Right".into()),
                            ],
                            |cx: &App| {
                                let dock = AppSettings::global(cx).panel.env_panel.sidebar_dock;
                                SharedString::from(if dock == SidebarDock::Right {
                                    "right"
                                } else {
                                    "left"
                                })
                            },
                            move |val: SharedString, cx: &mut App| {
                                let dock = if val == "right" {
                                    SidebarDock::Right
                                } else {
                                    SidebarDock::Left
                                };
                                match env_client.clone().and_then(|c| c.upgrade()) {
                                    Some(client) => client.update(cx, |c, cx| {
                                        c.set_env_dock(dock, cx);
                                    }),
                                    None => {
                                        AppSettings::global_mut(cx).panel.env_panel.sidebar_dock =
                                            dock;
                                        AppSettings::global_mut(cx).save();
                                    }
                                }
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
            self.panels_settings(),
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
