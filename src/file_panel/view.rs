use gpui_kit::component::input::Input;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::tree;
use gpui_kit::component::{ActiveTheme, Icon, IconNamed, StyledExt, h_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::actions::{
    CopyPath, CopyRelativePath, CreateFile, CreateFolder, DeleteItem, RenameItem,
    StressTestPlayground, TrashItem,
};
use super::model::FilePanel;
use crate::actions::CopyAsCode;
use crate::helpers::render_method_tag;
use crate::icons::IconName;

impl Render for FilePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snap = self.snapshot();
        let panel = cx.weak_entity();

        let workspace_name = snap.workspace_name.clone();
        let nodes = snap.nodes.clone();
        let pending_new = snap.pending_new.clone();
        let pending_rename = snap.pending_rename.clone();
        let pending_error = snap.pending_error.clone();
        let menu_nodes = nodes.clone();
        let menu_panel = panel.clone();
        let menu_focus = snap.focus.clone();
        let menu_roots = snap.root_id;
        let content = if self.is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No workspace")
                .into_any_element()
        } else {
            tree(
                &self.tree_state(),
                move |ix, entry, selected, _window, cx| {
                    let key = entry.item().id.clone();
                    if key.as_ref() == "pending:new" {
                        if let Some(input) = pending_new.clone() {
                            let error = pending_error.clone();
                            return ListItem::new(("pending-row", ix))
                                .mx(px(4.))
                                .rounded(px(6.))
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(Input::new(&input).appearance(false))
                                        .when_some(error, |this, message| {
                                            this.child(
                                                div()
                                                    .w_full()
                                                    .border_1()
                                                    .border_color(cx.theme().danger)
                                                    .rounded(px(6.))
                                                    .px_2()
                                                    .py_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().danger)
                                                            .child(message),
                                                    ),
                                            )
                                        }),
                                );
                        }
                    }
                    let node_id: usize = key.parse().unwrap_or(usize::MAX);
                    let Some(node) = nodes.get(&node_id).cloned() else {
                        return ListItem::new(("missing-row", ix));
                    };
                    if let Some((rename_id, input)) = pending_rename.clone() {
                        if rename_id == node_id {
                            let error = pending_error.clone();
                            return ListItem::new(("rename-row", ix))
                                .selected(selected)
                                .mx(px(4.))
                                .rounded(px(6.))
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(Input::new(&input).appearance(false))
                                        .when_some(error, |this, message| {
                                            this.child(
                                                div()
                                                    .w_full()
                                                    .border_1()
                                                    .border_color(cx.theme().danger)
                                                    .rounded(px(6.))
                                                    .px_2()
                                                    .py_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().danger)
                                                            .child(message),
                                                    ),
                                            )
                                        }),
                                );
                        }
                    }
                    let expanded = entry.is_expanded();
                    let folder_icon = if node.is_file {
                        None
                    } else if expanded {
                        Some(IconName::FolderOpen)
                    } else {
                        Some(IconName::Folder)
                    };
                    let chevron = if node.is_file {
                        None
                    } else if expanded {
                        Some(IconName::ChevronDown)
                    } else {
                        Some(IconName::ChevronRight)
                    };
                    let muted = cx.theme().muted_foreground;
                    // Zed-compact: 15px icons, small label, no extra padding.
                    let icon_el = |name: IconName| {
                        Icon::empty()
                            .path(name.path())
                            .size(px(15.))
                            .into_any_element()
                    };
                    let row_panel = panel.clone();
                    ListItem::new(("file-row", ix))
                        .selected(selected)
                        .mx(px(4.))
                        .rounded(px(6.))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_1()
                                .pl(px(6. + entry.depth() as f32 * 14.))
                                .children(folder_icon.map(|icon| div().flex_none().child(icon_el(icon))))
                                .child(div().text_sm().child(node.name.clone()))
                                .child(div().flex_1())
                                .when(node.is_file, |t| {
                                    t.child(render_method_tag(&node.method))
                                })
                                .children(chevron.map(|icon| {
                                    div().flex_none().text_color(muted).child(
                                        Icon::empty()
                                            .path(icon.path())
                                            .size(px(12.)),
                                    )
                                })),
                        )
                        .on_click(move |_, window, cx| {
                            row_panel
                                .update(cx, |p, cx| {
                                    p.click_node(node_id, window, cx);
                                })
                                .ok();
                        })
                },
            )
            .context_menu(move |_ix, entry, menu, _window, cx| {
                let node_id: usize = entry.item().id.parse().unwrap_or(usize::MAX);
                menu_panel
                    .update(cx, |p, _| {
                        p.set_context_target(node_id);
                    })
                    .ok();
                let is_file = menu_nodes
                    .get(&node_id)
                    .map(|n| n.is_file)
                    .unwrap_or(true);
                let is_root = menu_roots.is_some_and(|id| id == node_id);
                let menu = menu.min_w(px(200.)).action_context(menu_focus.clone());
                let menu = if !is_file {
                    menu.menu("Create File", Box::new(CreateFile))
                        .menu("Create Folder", Box::new(CreateFolder))
                        .separator()
                } else {
                    menu.menu("Stress Test", Box::new(StressTestPlayground))
                        .menu("Copy as Code", Box::new(CopyAsCode))
                        .separator()
                };
                let menu = menu
                    .menu("Copy Path", Box::new(CopyPath))
                    .menu("Copy Relative Path", Box::new(CopyRelativePath))
                    .separator();
                if !is_root {
                    menu.menu("Rename", Box::new(RenameItem))
                        .menu("Trash", Box::new(TrashItem))
                        .menu("Delete", Box::new(DeleteItem))
                } else {
                    menu
                }
            })
            .into_any_element()
        };

        div()
            .id("file-panel")
            .track_focus(&self.focus())
            .h_full()
            .w_full()
            .v_flex()
            .overflow_hidden()
            .bg(cx.theme().tokens.sidebar)
            .on_action(cx.listener(Self::handle_create_file))
            .on_action(cx.listener(Self::handle_create_folder))
            .on_action(cx.listener(Self::handle_rename_item))
            .on_action(cx.listener(Self::handle_delete_item))
            .on_action(cx.listener(Self::handle_trash_item))
            .on_action(cx.listener(Self::handle_copy_path))
            .on_action(cx.listener(Self::handle_copy_relative_path))
            .on_action(cx.listener(Self::activate_stress_test_playground))
            .on_action(cx.listener(Self::handle_copy_as_code))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_semibold()
                    .child(workspace_name),
            )
            .child(div().flex_1().min_h(px(0.)).px(px(2.)).child(content))
            .into_element()
    }
}
