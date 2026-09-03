use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ColorName, Sizable};
use gpui_kit::component::{Theme, ThemeConfig, ThemeRegistry};
use gpui_kit::*;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn render_method_tag(method: &str) -> impl IntoElement {
    match method {
        "GET" => Tag::color(ColorName::Green).outline().child("GET").xsmall(),

        "POST" => Tag::color(ColorName::Blue).outline().child("POST").xsmall(),

        "PUT" => Tag::color(ColorName::Yellow)
            .outline()
            .child("PUT")
            .xsmall(),

        "PATCH" => Tag::color(ColorName::Orange)
            .outline()
            .child("PATCH")
            .xsmall(),

        "DELETE" => Tag::color(ColorName::Red)
            .outline()
            .child("DELETE")
            .xsmall(),

        "HEAD" => Tag::color(ColorName::Purple)
            .outline()
            .child("HEAD")
            .xsmall(),

        "OPTIONS" => Tag::color(ColorName::Gray)
            .outline()
            .child("OPTIONS")
            .xsmall(),

        _ => Tag::color(ColorName::Neutral)
            .outline()
            .child(method.to_string())
            .xsmall(),
    }
}

pub fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn get_themes(cx: &App) -> Vec<(SharedString, SharedString)> {
    ThemeRegistry::global(cx)
        .themes()
        .keys()
        .filter(|k| k.as_ref() != "Default Dark" && k.as_ref() != "Default Light")
        .cloned()
        .map(|k| (k.clone(), k.clone()))
        .collect()
}

pub fn get_active_theme(cx: &App) -> SharedString {
    Theme::global(cx).theme_name().clone()
}

pub fn get_theme_config(cx: &App, name: &SharedString) -> Option<Rc<ThemeConfig>> {
    ThemeRegistry::global(cx).themes().get(name).cloned()
}

pub fn get_fonts(cx: &App) -> Vec<(SharedString, SharedString)> {
    cx.text_system()
        .all_font_names()
        .into_iter()
        .map(|f| (f.clone().into(), f.into()))
        .collect()
}

pub fn get_active_font(cx: &App) -> SharedString {
    Theme::global(cx).font_family.clone()
}

pub fn format_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;

    let bytes = bytes as f64;

    if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}
