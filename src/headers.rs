use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{h_flex, v_flex};

use crate::icons::IconName;

pub enum HeadersEvent {
    Changed,
}

impl EventEmitter<HeadersEvent> for Headers {}

#[derive(Clone)]
pub struct Header {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

pub struct Headers {
    rows: Vec<Header>,
}

impl Headers {
    pub fn new() -> Self {
        Self { rows: vec![] }
    }

    pub fn active_headers(&self, cx: &App) -> Vec<(String, String)> {
        self.rows
            .iter()
            .filter(|h| h.active)
            .map(|h| {
                (
                    h.key.read(cx).value().to_string(),
                    h.value.read(cx).value().to_string(),
                )
            })
            .collect()
    }

    pub fn rows(&self, cx: &App) -> Vec<(String, String, bool)> {
        self.rows
            .iter()
            .map(|h| {
                (
                    h.key.read(cx).value().to_string(),
                    h.value.read(cx).value().to_string(),
                    h.active,
                )
            })
            .collect()
    }

    fn watch(
        &mut self,
        key: Entity<InputState>,
        value: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(&key, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(HeadersEvent::Changed);
            }
        })
        .detach();
        cx.subscribe_in(&value, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(HeadersEvent::Changed);
            }
        })
        .detach();
    }

    pub fn load_from_json(
        &mut self,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(arr) = data.get("headers").and_then(|v| v.as_array()) {
            let rows: Vec<Header> = arr
                .iter()
                .map(|item| {
                    let key_str = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let val_str = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
                    let key = cx.new(|cx| InputState::new(window, cx).default_value(key_str));
                    let value = cx.new(|cx| InputState::new(window, cx).default_value(val_str));
                    Header { key, value, active }
                })
                .collect();

            self.rows = rows;
            let watched: Vec<(Entity<InputState>, Entity<InputState>)> = self
                .rows
                .iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect();
            for (key, value) in watched {
                self.watch(key, value, window, cx);
            }
        }
    }
}

impl Render for Headers {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(rems(0.75))
            .child(
                h_flex()
                    .items_center()
                    .child(div().flex_1())
                    .child(
                        Button::new("add-head")
                            .label("Add Header")
                            .icon(IconName::Plus)
                            .tooltip("Add Header")
                            .ghost()
                            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                                let key = cx.new(|cx| InputState::new(window, cx));
                                let value = cx.new(|cx| InputState::new(window, cx));
                                this.watch(key.clone(), value.clone(), window, cx);
                                this.rows.push(Header {
                                    key,
                                    value,
                                    active: true,
                                });
                                cx.emit(HeadersEvent::Changed);
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
                            .children(self.rows.iter().enumerate().map(|(i, header)| {
                                TableRow::new()
                                    .child(
                                        TableCell::new().w(rems(2.5)).child(
                                            Checkbox::new(format!("head-check-{i}"))
                                                .checked(header.active)
                                                .on_click({
                                                    let key = header.key.clone();
                                                    let value = header.value.clone();
                                                    cx.listener(
                                                        move |this: &mut Self, checked: &bool, _window, cx| {
                                                            this.rows[i] = Header {
                                                                key: key.clone(),
                                                                value: value.clone(),
                                                                active: *checked,
                                                            };
                                                            cx.emit(HeadersEvent::Changed);
                                                            cx.notify();
                                                        },
                                                    )
                                                }),
                                        ),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&header.key)),
                                    )
                                    .child(
                                        TableCell::new()
                                            .flex_1()
                                            .child(Input::new(&header.value)),
                                    )
                                    .child(
                                        TableCell::new().w(rems(2.5)).flex().justify_end().child(
                                            Button::new(format!("del-head-{i}"))
                                                .ghost()
                                                .small()
                                                .tooltip("Delete")
                                                .icon(IconName::Trash)
                                                .on_click(cx.listener(
                                                    move |this: &mut Self, _, _window, cx| {
                                                        this.rows.remove(i);
                                                        cx.emit(HeadersEvent::Changed);
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
