use crate::icons::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::model::QueryParams;

impl Render for QueryParams {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(rems(0.75))
            .child(
                h_flex()
                    .items_center()
                    .child(div().flex_1())
                    .child(
                        Button::new("add-qp")
                            .label("Add Param")
                            .icon(IconName::Plus)
                            .tooltip("Add Param")
                            .ghost()
                            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                                this.add_row(window, cx);
                            })),
                    ),
            )
            .child(
                Table::new()
                    .child(
                        TableHeader::new().w_full().child(
                            TableRow::new()
                                .child(TableHead::new().w(rems(2.5)).child(""))
                                .child(TableHead::new().flex_1().child("Key"))
                                .child(TableHead::new().flex_1().child("Value"))
                                .child(TableHead::new().w(rems(2.5)).child("")),
                        ),
                    )
                    .child(
                        TableBody::new().children(
                            self.row_inputs()
                                .into_iter()
                                .enumerate()
                                .map(|(i, (key, value, active))| {
                                TableRow::new()
                                    .child(
                                        TableCell::new().w(rems(2.5)).child(
                                            Checkbox::new(format!("qp-check-{i}"))
                                                .checked(active)
                                                .on_click({
                                                    cx.listener(
                                                        move |this: &mut Self, checked: &bool, _window, cx| {
                                                            this.set_row_active(i, *checked, cx);
                                                        },
                                                    )
                                                }),
                                        ),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&key)),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&value)),
                                    )
                                    .child(
                                        TableCell::new().w(rems(2.5)).flex().justify_end().child(
                                            Button::new(format!("del-qp-{i}"))
                                                .ghost()
                                                .small()
                                                .tooltip("Delete")
                                                .icon(IconName::Trash)
                                                .on_click(cx.listener(
                                                    move |this: &mut Self, _, _window, cx| {
                                                        this.remove_row(i, cx);
                                                    },
                                                )),
                                        ),
                                    )
                            }),
                        ),
                    ),
            )
    }
}
