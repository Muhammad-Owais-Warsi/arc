use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::IndexPath;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::Theme;
use gpui_kit::*;

use super::shell::{OverlayPosition, OverlayRequest, OverlayRoot};
use crate::helpers::{get_active_theme, get_theme_config, get_themes};
use crate::settings_panel::AppSettings;

pub fn preview_theme(name: &str, window: &mut Window, cx: &mut App) {
    let name = SharedString::from(name);
    if let Some(theme_config) = get_theme_config(cx, &name) {
        let mode = theme_config.mode;
        let t = Theme::global_mut(cx);
        if mode.is_dark() {
            t.dark_theme = theme_config.clone();
        } else {
            t.light_theme = theme_config.clone();
        }
        Theme::change(mode, Some(window), cx);
        let app_settings = AppSettings::global(cx).clone();
        let t = Theme::global_mut(cx);
        t.font_family = app_settings.font.family.clone().into();
        t.font_size = px(app_settings.font.size);
        window.refresh();
    }
}

pub fn open(state: Entity<CommandState>, window: &mut Window, cx: &mut App) {
    let committed = get_active_theme(cx).to_string();
    let themes: Rc<Vec<SharedString>> = Rc::new(
        get_themes(cx)
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
    );
    let items: Rc<Vec<CommandItem>> = Rc::new(
        themes
            .iter()
            .map(|name| {
                CommandItem::new()
                    .label(name.as_ref())
                    .checked(name.as_ref() == committed)
            })
            .collect(),
    );
    let restore = committed.clone();
    let on_cancel: Rc<dyn Fn(&mut Window, &mut App)> = Rc::new(move |window, cx| {
        let restore = restore.clone();
        window.defer(cx, move |window, cx| {
            preview_theme(&restore, window, cx);
        });
    });
    let content_state = state.clone();
    let seen: Rc<RefCell<(String, Option<IndexPath>)>> =
        Rc::new(RefCell::new((String::new(), None)));
    let content: Rc<dyn Fn() -> AnyElement> = Rc::new(move || {
        let select_themes = themes.clone();
        let confirm_themes = themes.clone();
        let query_state = content_state.clone();
        let seen_guard = seen.clone();
        let items = items.clone();
        Command::new(&content_state)
            .bordered(false)
            .placeholder("Select Theme...")
            .max_h(px(480.))
            .items((*items).clone())
            .on_select(move |index, window, cx| {
                let row = index.row;
                let query = query_state.read(cx).query(cx).to_string();
                let mut seen = seen_guard.borrow_mut();
                if seen.0 != query {
                    *seen = (query, None);
                    return;
                }
                *seen = (query, Some(index));
                drop(seen);
                if let Some(name) = select_themes.get(row) {
                    preview_theme(name.as_ref(), window, cx);
                }
            })
            .on_confirm(move |index, window, cx| {
                if let Some(name) = confirm_themes.get(index.row) {
                    preview_theme(name.as_ref(), window, cx);
                    AppSettings::global_mut(cx).theme.name = name.to_string();
                    if let Some(theme_config) = get_theme_config(cx, name) {
                        AppSettings::global_mut(cx).theme.mode =
                            theme_config.mode.name().to_string();
                    }
                    AppSettings::global_mut(cx).save();
                }
                OverlayRoot::hide(cx, window, false);
            })
            .into_any_element()
    });
    let autofocus_state = state.clone();
    let autofocus: Rc<dyn Fn(&mut Window, &mut App)> = Rc::new(move |window, cx| {
        autofocus_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    });
    OverlayRoot::show(
        cx,
        window,
        OverlayRequest {
            content,
            on_cancel: Some(on_cancel),
            autofocus: Some(autofocus),
            width: px(520.),
            backdrop_closable: true,
            position: OverlayPosition::TopCenter,
        },
    );
}
