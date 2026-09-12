use crate::actions::{CopyEnv, DeleteEnv};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::component::{ActiveTheme, Icon, IconNamed, StyledExt, h_flex};
use gpui_kit::*;

use crate::env_playground::Environment;
use crate::fs;
use crate::fs::request::KeyValue;
use crate::icons::IconName;

pub enum EnvPanelEvent {
    EnvActivated { name: String },
    EnvDeleted { name: String },
}

impl EventEmitter<EnvPanelEvent> for EnvPanel {}

pub struct EnvPanel {
    pub envs: Vec<String>,
    context_target: Option<String>,
    focus_handle: FocusHandle,
    tree: Entity<TreeState>,
}

impl EnvPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let envs = Self::read_env_names();
        let items: Vec<TreeItem> = envs
            .iter()
            .map(|name| TreeItem::new(name.clone(), name.clone()))
            .collect();
        let tree = cx.new(|cx| TreeState::new(cx).items(items));

        Self {
            envs,
            context_target: None,
            focus_handle: cx.focus_handle(),
            tree,
        }
    }

    pub fn read_env_names() -> Vec<String> {
        let content = fs::env::read_environments();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter().map(|e| e.name).collect()
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.envs = Self::read_env_names();
        let items: Vec<TreeItem> = self
            .envs
            .iter()
            .map(|name| TreeItem::new(name.clone(), name.clone()))
            .collect();
        self.tree.update(cx, |tree, cx| {
            tree.set_items(items, cx);
        });
        cx.notify();
    }

    fn read_env_from_disk(&self, name: &str) -> Vec<KeyValue> {
        let content = fs::env::read_environments();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter()
            .find(|e| e.name == name)
            .map(|e| e.variables)
            .unwrap_or_default()
    }

    fn handle_copy_env(&mut self, _: &CopyEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            let vars = self.read_env_from_disk(&name);
            let json = serde_json::to_string_pretty(&vars).unwrap_or_default();
            cx.write_to_clipboard(ClipboardItem::new_string(json));
        }
    }

    fn handle_delete_env(&mut self, _: &DeleteEnv, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = self.context_target.take() {
            fs::env::delete(&name);
            cx.emit(EnvPanelEvent::EnvDeleted { name: name.clone() });
            self.refresh(cx);
        }
    }
}

impl Render for EnvPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = cx.weak_entity();
        let menu_panel = panel.clone();

        let content =
            if self.envs.is_empty() {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child("No environments")
                    .into_any_element()
            } else {
                tree(&self.tree, move |ix, entry, selected, _window, _cx| {
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
                            p.context_target = Some(name.clone());
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
            .track_focus(&self.focus_handle)
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
