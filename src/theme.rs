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
