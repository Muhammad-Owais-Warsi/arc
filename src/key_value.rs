use std::rc::Rc;

use crate::tabs::{TabManager, Tabs};
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{IconName, Sizable, h_flex, v_flex};

pub struct KeyValueItem {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

impl KeyValueItem {
    pub fn build(
        window: &mut Window,
        cx: &mut Context<TabManager>,
        tab: Entity<Tabs>,
        key: &str,
        value: &str,
        active: bool,
    ) -> Entity<Self> {
        let key_input = cx.new(|cx| InputState::new(window, cx).default_value(key));
        let key_sub = key_input.clone();

        let value_input = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let value_sub = value_input.clone();

        let item = cx.new(|_| Self {
            key: key_input,
            value: value_input,
            active,
        });

        let tab_clone = tab.clone();
        cx.subscribe_in(
            &key_sub,
            window,
            move |_: &mut TabManager, _, event, _window, cx| {
                if let InputEvent::Change = event {
                    tab_clone.update(cx, |tab, cx| {
                        tab.dirty = true;
                        cx.notify();
                    })
                }
            },
        )
        .detach();

        let tab_clone = tab.clone();
        cx.subscribe_in(
            &value_sub,
            window,
            move |_: &mut TabManager, _, event, _window, cx| {
                if let InputEvent::Change = event {
                    tab_clone.update(cx, |tab, cx| {
                        tab.dirty = true;
                        cx.notify();
                    })
                }
            },
        )
        .detach();

        item
    }

    pub fn from_json(
        window: &mut Window,
        cx: &mut Context<TabManager>,
        tab: Entity<Tabs>,
        value: &serde_json::Value,
        json_key: &str,
    ) -> Vec<Entity<Self>> {
        let Some(items) = value.get(json_key).and_then(|v| v.as_array()) else {
            return vec![];
        };

        items
            .iter()
            .map(|item| {
                let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
                Self::build(window, cx, tab.clone(), key, value, active)
            })
            .collect()
    }
}

pub struct KeyValueEditor {
    add_label: SharedString,
    checkbox_prefix: SharedString,
    add_id: SharedString,
    on_add: Rc<dyn Fn(&mut TabManager, &mut Window, &mut Context<TabManager>)>,
    on_delete: Rc<dyn Fn(EntityId, &mut TabManager, &mut Window, &mut Context<TabManager>)>,
}

impl KeyValueEditor {
    pub fn new(
        add_label: impl Into<SharedString>,
        checkbox_prefix: impl Into<SharedString>,
        add_id: impl Into<SharedString>,
        on_add: impl Fn(&mut TabManager, &mut Window, &mut Context<TabManager>) + 'static,
        on_delete: impl Fn(EntityId, &mut TabManager, &mut Window, &mut Context<TabManager>) + 'static,
    ) -> Self {
        Self {
            add_label: add_label.into(),
            checkbox_prefix: checkbox_prefix.into(),
            add_id: add_id.into(),
            on_add: Rc::new(on_add),
            on_delete: Rc::new(on_delete),
        }
    }

    pub fn render(
        &self,
        items: &[Entity<KeyValueItem>],
        api: &mut TabManager,
        cx: &mut Context<TabManager>,
    ) -> impl IntoElement {
        if api.active_tab_id.and_then(|id| api.tabs.get(&id)).is_none() {
            return div().into_any_element();
        }

        let on_add = self.on_add.clone();
        let on_delete = self.on_delete.clone();
        let checkbox_prefix = self.checkbox_prefix.clone();
        let add_label = self.add_label.clone();
        let add_id = self.add_id.clone();

        v_flex()
            .gap(rems(0.75))
            .child(
                h_flex()
                    .items_center()
                    .child(div().flex_1())
                    .child(
                        Button::new(add_id)
                            .label(add_label.clone())
                            .icon(IconName::Plus)
                            .tooltip(add_label)
                            .ghost()
                            .on_click({
                                cx.listener(move |this: &mut TabManager, _event, window, cx| {
                                    on_add(this, window, cx);
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
                        let items = items.to_vec();
                        let on_delete = on_delete.clone();
                        let checkbox_prefix = checkbox_prefix.clone();
                        TableBody::new().children(items.into_iter().enumerate().map(
                            move |(i, entity)| {
                                let entity = entity.clone();
                                let (key, value, active) = {
                                    let state = entity.read(cx);
                                    (state.key.clone(), state.value.clone(), state.active)
                                };

                                let on_delete = on_delete.clone();
                                let checkbox_prefix = checkbox_prefix.clone();

                                TableRow::new()
                                    .child(
                                        TableCell::new().w(rems(2.5)).child(
                                            Checkbox::new(format!("{checkbox_prefix}{i}"))
                                                .checked(active)
                                                .on_click({
                                                    let entity = entity.clone();
                                                    cx.listener(move |_: &mut TabManager, checked: &bool, _window, cx| {
                                                        entity.update(cx, |item, _| item.active = *checked);
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
                                                    let entity_id = entity.entity_id();
                                                    cx.listener(move |this: &mut TabManager, _: &ClickEvent, window, cx| {
                                                        on_delete(entity_id, this, window, cx);
                                                        cx.notify();
                                                    })
                                                }),
                                        ),
                                    )
                            },
                        ))
                    }),
            )
            .into_any_element()
    }
}
