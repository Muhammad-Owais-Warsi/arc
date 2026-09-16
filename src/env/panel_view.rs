use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::tree;
use gpui_kit::component::{ActiveTheme, Icon, IconNamed, StyledExt, h_flex};
use gpui_kit::*;

use super::actions::{CopyEnv, DeleteEnv};
use super::panel_model::{EnvPanel, EnvPanelEvent};
use crate::icons::IconName;

impl Render for EnvPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = cx.weak_entity();
        let menu_panel = panel.clone();

        let content = if self.is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No environments")
                .into_any_element()
        } else {
            tree(&self.tree_state(), move |ix, entry, selected, _window, _cx| {
                let name = entry.item().label.to_string();
                let row_panel = panel.clone();
                let emit_name = name.clone();
                ListItem::new(("env-row", ix))
                    .selected(selected)
                    .mx(px(4.))
                    .rounded(px(6.))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap_1()
                            .child(div().flex_none().child(
                                Icon::empty().path(IconName::Variable.path()).size(px(15.)),
                            ))
                            .child(div().text_sm().child(name)),
                    )
                    .on_click(move |_, _, cx| {
                        row_panel
                            .update(cx, |_, cx| {
                                cx.emit(EnvPanelEvent::EnvActivated {
                                    name: emit_name.clone(),
                                });
                            })
                            .ok();
                    })
            })
            .context_menu(move |_ix, entry, menu, _window, cx| {
                let name = entry.item().label.to_string();
                menu_panel
                    .update(cx, |p, _| {
                        p.set_context_target(name.clone());
                    })
                    .ok();
                menu.menu("Copy Variables", Box::new(CopyEnv))
                    .separator()
                    .menu("Delete", Box::new(DeleteEnv))
            })
            .into_any_element()
        };

        div()
            .id("env-panel")
            .track_focus(&self.focus())
            .h_full()
            .w_full()
            .v_flex()
            .overflow_hidden()
            .bg(cx.theme().tokens.sidebar)
            .on_action(cx.listener(Self::handle_copy_env))
            .on_action(cx.listener(Self::handle_delete_env))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_semibold()
                    .child("Environments"),
            )
            .child(div().flex_1().min_h(px(0.)).px(px(2.)).child(content))
    }
}
