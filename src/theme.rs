use crate::settings::AppSettings;
use gpui_kit::component::{Theme, ThemeConfig, ThemeRegistry};
use gpui_kit::*;
use std::rc::Rc;

/// Zed's theme order: dark families first, then lights. The registry is
/// a HashMap, so without this the picker reshuffles on every launch.
const THEME_ORDER: &[&str] = &[
    "Ayu Dark",
    "Ayu Mirage",
    "Gruvbox Dark",
    "Gruvbox Dark Hard",
    "Gruvbox Dark Soft",
    "One Dark",
    "Ayu Light",
    "Gruvbox Light",
    "Gruvbox Light Hard",
    "Gruvbox Light Soft",
    "One Light",
];

fn theme_sort_key(name: &str) -> (usize, String) {
    match THEME_ORDER.iter().position(|n| *n == name) {
        Some(ix) => (ix, String::new()),
        None => (THEME_ORDER.len(), name.to_string()),
    }
}

pub fn get_themes(cx: &App) -> Vec<(SharedString, SharedString)> {
    let mut names: Vec<SharedString> = ThemeRegistry::global(cx)
        .themes()
        .keys()
        .filter(|k| k.as_ref() != "Default Dark" && k.as_ref() != "Default Light")
        .cloned()
        .collect();
    names.sort_by(|a, b| theme_sort_key(a.as_ref()).cmp(&theme_sort_key(b.as_ref())));
    names.into_iter().map(|k| (k.clone(), k.clone())).collect()
}

pub fn get_active_theme(cx: &App) -> SharedString {
    Theme::global(cx).theme_name().clone()
}

pub fn get_theme_config(cx: &App, name: &SharedString) -> Option<Rc<ThemeConfig>> {
    ThemeRegistry::global(cx).themes().get(name).cloned()
}

/// The theme families offered on first run, in display order.
pub const THEME_FAMILIES: &[&str] = &["One", "Ayu", "Gruvbox"];

/// Pick each family's theme matching the current mode, falling back to any
/// installed theme from that family. Returns `(family label, theme name)`.
pub fn theme_families(cx: &App) -> Vec<(String, SharedString)> {
    let mode = Theme::global(cx).mode;
    let installed = get_themes(cx);

    THEME_FAMILIES
        .iter()
        .filter_map(|family| {
            let matches_mode = installed.iter().find(|(name, _)| {
                name.starts_with(family)
                    && get_theme_config(cx, name).is_some_and(|config| config.mode == mode)
            });
            matches_mode
                .or_else(|| installed.iter().find(|(name, _)| name.starts_with(family)))
                .map(|(name, _)| (family.to_string(), name.clone()))
        })
        .collect()
}

/// Apply a theme by name and persist it, re-applying the saved font because a
/// theme change resets typography.
pub fn apply_theme(name: &str, cx: &mut App) {
    let key: SharedString = name.into();
    let Some(theme_config) = get_theme_config(cx, &key) else {
        return;
    };

    let mode = theme_config.mode;
    let theme = Theme::global_mut(cx);
    if mode.is_dark() {
        theme.dark_theme = theme_config.clone();
    } else {
        theme.light_theme = theme_config.clone();
    }
    Theme::change(mode, None, cx);

    let app_settings = AppSettings::global(cx).clone();
    let theme = Theme::global_mut(cx);
    theme.font_family = app_settings.font.family.clone().into();
    theme.font_size = px(app_settings.font.size);

    AppSettings::global_mut(cx).theme.name = name.to_string();
    AppSettings::global_mut(cx).theme.mode = mode.name().to_string();
    AppSettings::global_mut(cx).save();
    cx.refresh_windows();
}
