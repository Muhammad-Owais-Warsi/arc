use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{ActiveTheme, Icon, h_flex, v_flex};

use crate::icons::IconName;

#[derive(Clone)]
pub struct Param {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

pub struct QueryParams {
    rows: Vec<Param>,
}

impl QueryParams {
    pub fn new() -> Self {
        Self { rows: vec![] }
    }

    pub fn active_params(&self, cx: &App) -> Vec<(String, String)> {
        self.rows
            .iter()
            .filter(|p| p.active)
            .map(|p| {
                (
                    p.key.read(cx).value().to_string(),
                    p.value.read(cx).value().to_string(),
                )
            })
            .collect()
    }

    pub fn load_from_json(
        &mut self,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(arr) = data.get("query_params").and_then(|v| v.as_array()) {
            self.rows = arr
                .iter()
                .map(|item| {
                    let key_str = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let val_str = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
                    let key = cx.new(|cx| InputState::new(window, cx).default_value(key_str));
                    let value = cx.new(|cx| InputState::new(window, cx).default_value(val_str));
                    Param { key, value, active }
                })
                .collect();
        }
    }
}

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
                                let key = cx.new(|cx| InputState::new(window, cx));
                                let value = cx.new(|cx| InputState::new(window, cx));
                                this.rows.push(Param {
                                    key,
                                    value,
                                    active: true,
                                });
                                cx.notify();
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
                        TableBody::new()
                            .children(self.rows.iter().enumerate().map(|(i, param)| {
                                TableRow::new()
                                    .child(
                                        TableCell::new().w(rems(2.5)).child(
                                            Checkbox::new(format!("qp-check-{i}"))
                                                .checked(param.active)
                                                .on_click({
                                                    let key = param.key.clone();
                                                    let value = param.value.clone();
                                                    cx.listener(
                                                        move |this: &mut Self, checked: &bool, _window, cx| {
                                                            this.rows[i] = Param {
                                                                key: key.clone(),
                                                                value: value.clone(),
                                                                active: *checked,
                                                            };
                                                            cx.notify();
                                                        },
                                                    )
                                                }),
                                        ),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&param.key)),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&param.value)),
                                    )
                                    .child(
                                        TableCell::new().w(rems(2.5)).flex().justify_end().child(
                                            Button::new(format!("del-qp-{i}"))
                                                .ghost()
                                                .small()
                                                .icon(IconName::Trash)
                                                .on_click(cx.listener(
                                                    move |this: &mut Self, _, _window, cx| {
                                                        this.rows.remove(i);
                                                        cx.notify();
                                                    },
                                                )),
                                        ),
                                    )
                            })),
                    ),
            )
    }
}
