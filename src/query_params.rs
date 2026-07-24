use crate::tabs::{TabManager, Tabs};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{IconName, Sizable, h_flex, v_flex};

pub struct QueryParams {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

fn build_query_param_entity(
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
    key: &str,
    value: &str,
    active: bool,
) -> Entity<QueryParams> {
    let key_input_state = cx.new(|cx| InputState::new(window, cx).default_value(key));
    let key_input_state_sub = key_input_state.clone();

    let value_input_state = cx.new(|cx| InputState::new(window, cx).default_value(value));
    let value_input_state_sub = value_input_state.clone();

    let qp = cx.new(|_cx| QueryParams {
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

    qp
}

fn new_query_param(
    _api: &mut TabManager,
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
) {
    let qp = build_query_param_entity(window, cx, tab.clone(), "", "", true);

    tab.update(cx, |tab, _cx| {
        tab.query_params.push(qp);
    });
}

pub fn query_params_from_json(
    window: &mut Window,
    cx: &mut Context<TabManager>,
    tab: Entity<Tabs>,
    value: &serde_json::Value,
) -> Vec<Entity<QueryParams>> {
    let Some(items) = value.get("query_params").and_then(|v| v.as_array()) else {
        return vec![];
    };

    items
        .iter()
        .map(|item| {
            let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
            build_query_param_entity(window, cx, tab.clone(), key, value, active)
        })
        .collect()
}

pub fn render_query_params_section(
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
                    Button::new("add-qp")
                        .label("Add Param")
                        .icon(IconName::Plus)
                        .tooltip("Add Query Param")
                        .ghost()
                        .on_click({
                            let tab = tab.clone();
                            cx.listener(move |this: &mut TabManager, _event, window, cx| {
                                new_query_param(this, window, cx, tab.clone());
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
                    let query_params = tab.read(cx).query_params.clone();
                    TableBody::new().children(query_params.into_iter().enumerate().map(
                        |(i, entity)| {
                            let entity = entity.clone();
                            let (key, value, active) = {
                                let state = entity.read(cx);
                                (state.key.clone(), state.value.clone(), state.active)
                            };

                            TableRow::new()
                                .child(
                                    TableCell::new().w(rems(2.5)).child(
                                        Checkbox::new(format!("qp-{i}")).checked(active).on_click({
                                            let entity = entity.clone();
                                            cx.listener(move |_this: &mut TabManager, checked: &bool, _window, cx| {
                                                entity.update(cx, |qp, _cx| qp.active = *checked);
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
                                                            tab.query_params
                                                                .retain(|q| q.entity_id() != entity.entity_id());
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
