use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::editor_model::EnvPlayground;
use crate::icons::IconName;

impl Render for EnvPlayground {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .min_h(px(0.))
            .gap(rems(0.75))
            .px(px(24.))
            .pt(rems(1.0))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div().text_lg().w(rems(10.)).child(
                                            Input::new(&self.name_input())
                                                .w_full()
                                                .readonly(!self.editing()),
                                        ),
                                    )
                                    .when(!self.editing(), |this| {
                                        this.child(
                                            Button::new("edit")
                                                .icon(IconName::SquarePen)
                                                .ghost()
                                                .small()
                                                .tooltip("Edit")
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.enable_editing(cx);
                                                })),
                                        )
                                    })
                                    .when(self.editing(), |this| {
                                        this.child(
                                            Button::new("save-name")
                                                .icon(IconName::Check)
                                                .primary()
                                                .small()
                                                .tooltip("Save")
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.commit_name(cx);
                                                })),
                                        )
                                    })
                                    .when(self.editing(), |this| {
                                        this.child(
                                            Button::new("cancel")
                                                .icon(IconName::X)
                                                .secondary()
                                                .danger()
                                                .small()
                                                .tooltip("Cancel")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.disable_editing(window, cx);
                                                })),
                                        )
                                    }),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("save-env")
                            .secondary()
                            .label("Save")
                            .tooltip("Save changes")
                            .when(self.is_dirty(), |this| {
                                this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                            })
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.save(cx);
                            })),
                    ),
            )
            .child(
                h_flex().justify_end().child(
                    Button::new("add-var")
                        .label("Add Variable")
                        .tooltip("Add new variable")
                        .icon(IconName::Plus)
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.add_variable(window, cx);
                        })),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .mt_2()
                    .overflow_y_scrollbar()
                    .child(
                        Table::new().w_full().child(
                            TableHeader::new().w_full().child(
                                TableRow::new()
                                    .child(TableHead::new().w(rems(2.5)).child(""))
                                    .child(TableHead::new().flex_1().child("Key"))
                                    .child(TableHead::new().flex_1().child("Value"))
                                    .child(TableHead::new().w(rems(2.5)).child("")),
                            ),
                        )
                        .child(TableBody::new().children(
                            self.row_inputs().into_iter().enumerate().map(
                                |(i, (key, value, active))| {
                                    TableRow::new()
                                        .child(
                                            TableCell::new().w(rems(2.5)).child(
                                                Checkbox::new(format!("env-check-{i}"))
                                                    .checked(active)
                                                    .on_click(cx.listener(
                                                        move |this: &mut Self,
                                                              checked: &bool,
                                                              _window,
                                                              cx| {
                                                            this.set_variable_active(i, *checked, cx);
                                                        },
                                                    )),
                                            ),
                                        )
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&key).w_full()),
                                        )
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&value).w_full()),
                                        )
                                        .child(
                                            TableCell::new()
                                                .w(rems(2.5))
                                                .flex()
                                                .justify_end()
                                                .child(
                                                    Button::new(format!("del-var-{i}"))
                                                        .ghost()
                                                        .icon(IconName::Trash)
                                                        .on_click(cx.listener(
                                                            move |this, _, _window, cx| {
                                                                this.remove_variable(i, cx);
                                                            },
                                                        )),
                                                ),
                                        )
                                },
                            ),
                        )),
                    ),
            )
    }
}
