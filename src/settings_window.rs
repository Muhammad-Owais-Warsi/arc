use gpui_kit::component::{ActiveTheme as _, StyledExt as _, TitleBar};
use gpui_kit::*;

use crate::settings_panel::SettingsPanel;

pub struct SettingsWindow {
    settings_panel: Entity<SettingsPanel>,
}

impl SettingsWindow {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings_panel = cx.new(|cx| SettingsPanel::new(cx));
        Self { settings_panel }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .w_full()
            .size_full()
            .bg(theme.background)
            .v_flex()
            .child(
                TitleBar::new()
                    .bg(cx.theme().title_bar)
                    .child(div().px_2().child("Settings")),
            )
            .child(div().flex_1().child(self.settings_panel.clone()))
    }
}
