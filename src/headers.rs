use crate::tabs::{TabManager, Tabs};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{ActiveTheme, IconName, Sizable, StyledExt, h_flex, v_flex};

pub struct Headers {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

fn build_header_entity(
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
    key: &str,
    value: &str,
    active: bool,
) -> Entity<Headers> {
    let key_input_state = cx.new(|cx| InputState::new(window, cx).default_value(key));
    let key_input_state_sub = key_input_state.clone();

    let value_input_state = cx.new(|cx| InputState::new(window, cx).default_value(value));
    let value_input_state_sub = value_input_state.clone();

    let head = cx.new(|_cx| Headers {
        key: key_input_state,
        value: value_input_state,
        active,
    });

    let key_tab = tab.clone();
    cx.subscribe_in(
        &key_input_state_sub,
        window,
        move |_this: &mut TabManager, _, event, _window, cx| {
            if let InputEvent::Change = event {
                key_tab.update(cx, |tab, cx| {
                    tab.dirty = true;
                    cx.notify();
                })
            }
        },
    )
    .detach();

    let value_tab = tab.clone();

    cx.subscribe_in(
        &value_input_state_sub,
        window,
        move |_this: &mut TabManager, _, event, _window, cx| {
            if let InputEvent::Change = event {
                value_tab.update(cx, |tab, cx| {
                    tab.dirty = true;
                    cx.notify();
                })
            }
        },
    )
    .detach();

    head
}

fn new_header(
    _api: &mut TabManager,
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
) {
    let head = build_header_entity(window, cx, tab.clone(), "", "", true);

    tab.update(cx, |tab, _cx| {
        tab.headers.push(head);
    });
}

pub fn headers_from_json(
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
    value: &serde_json::Value,
) -> Vec<Entity<Headers>> {
    let Some(items) = value.get("headers").and_then(|v| v.as_array()) else {
        return vec![];
    };

    items
        .iter()
        .map(|item| {
            let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
            build_header_entity(window, cx, tab.clone(), key, value, active)
        })
        .collect()
}
pub fn render_response_headers(
    response_headers: Vec<(String, String)>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();

    div()
        .id("response-headers-vscroll") // outer: owns vertical scroll
        .w_full()
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scrollbar()
        .child(
            div()
                .id("response-headers-hscroll") // inner: owns horizontal scroll — different id from the outer
                .w_full()
                .min_w(px(0.))
                .overflow_x_scrollbar()
                .child(
                    div()
                        .flex_col()
                        .min_w(px(432.)) // Key (200) + Value (232) — forces horizontal scroll when panel is narrower
                        .child(
                            h_flex()
                                .flex_none()
                                .h(px(32.))
                                .items_center()
                                .bg(theme.table_head)
                                .text_color(theme.table_head_foreground)
                                .border_b_1()
                                .border_color(theme.table_row_border)
                                .child(
                                    div()
                                        .w(px(200.))
                                        .flex_none()
                                        .px(px(12.))
                                        .text_sm()
                                        .font_semibold()
                                        .child("Key"),
                                )
                                .child(
                                    div()
                                        .w(px(232.))
                                        .flex_none()
                                        .px(px(12.))
                                        .text_sm()
                                        .font_semibold()
                                        .child("Value"),
                                ),
                        )
                        .children(response_headers.into_iter().map(|(key, value)| {
                            h_flex()
                                .flex_none()
                                .h(px(32.))
                                .items_center()
                                .border_b_1()
                                .border_color(theme.table_row_border)
                                .child(
                                    div()
                                        .w(px(200.))
                                        .flex_none()
                                        .px(px(12.))
                                        .text_sm()
                                        .text_ellipsis()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(key),
                                )
                                .child(
                                    div()
                                        .w(px(232.))
                                        .flex_none()
                                        .px(px(12.))
                                        .text_sm()
                                        .text_ellipsis()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(value),
                                )
                        })),
                ),
        )
}
pub fn render_headers_section(
    api: &mut TabManager,
    cx: &mut Context<TabManager>,
) -> impl IntoElement {
    let Some(tab) = api.active_tab_id.and_then(|id| api.tabs.get(&id)).cloned() else {
        return div();
    };

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
                        .on_click({
                            let tab = tab.clone();
                            cx.listener(move |this: &mut TabManager, _event, window, cx| {
                                new_header(this, window, cx, tab.clone());
                                cx.notify();
                            })
                        }),
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
                .child({
                    let headers = tab.read(cx).headers.clone();
                    TableBody::new().children(headers.into_iter().enumerate().map(
                        |(i, entity)| {
                            let entity = entity.clone();
                            let (key, value, active) = {
                                let state = entity.read(cx);
                                (state.key.clone(), state.value.clone(), state.active)
                            };

                            TableRow::new()
                                .child(
                                    TableCell::new().w(rems(2.5)).child(
                                        Checkbox::new(format!("head-{i}")).checked(active).on_click({
                                            let entity = entity.clone();
                                            cx.listener(move |_this: &mut TabManager, checked: &bool, _window, cx| {
                                                entity.update(cx, |head, _cx| head.active = *checked);
                                                cx.notify();
                                            })
                                        }),
                                    ),
                                )
                                .child(TableCell::new().flex_1().child(Input::new(&key)))
                                .child(TableCell::new().flex_1().child(Input::new(&value)))
                                .child(
                                    TableCell::new().w(rems(2.5)).flex().justify_end().child(
                                        Button::new("del")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Delete)
                                            .on_click({
                                                let entity = entity.clone();
                                                let tab = tab.clone();

                                                    cx.listener(move |_this: &mut TabManager, _: &ClickEvent, _window, cx| {
                                                        tab.update(cx, |tab, _cx| {
                                                            tab.headers
                                                                .retain(|h| h.entity_id() != entity.entity_id());
                                                        });

                                                        cx.notify();
                                                    })
                                            }),
                                    ),
                                )
                        },
                    ))
                }),
        )
}
