use gpui::{App, SharedString};
use gpui_component::{Theme, ThemeConfig, ThemeRegistry};
use std::rc::Rc;

pub struct ThemesAndFonts;

impl ThemesAndFonts {
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
}
