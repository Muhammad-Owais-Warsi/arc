use crate::ApiClient;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::table::{
    Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow,
};
use gpui_component::{IconName, Sizable, h_flex, v_flex};

pub struct QueryParams {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

fn new_query_param(
    api: &mut ApiClient,
    window: &mut Window,
    cx: &mut Context<ApiClient>,
    tab_id: usize,
) {
    let qp = cx.new(|cx| QueryParams {
        key: cx.new(|cx| InputState::new(window, cx)),
        value: cx.new(|cx| InputState::new(window, cx)),
        active: true,
    });
    if let Some(tab) = api.tabs.iter_mut().find(|t| t.id == tab_id) {
        tab.query_params.push(qp);
    }
}

pub fn render_query_params_section(
    api: &ApiClient,
    cx: &mut Context<ApiClient>,
) -> impl IntoElement {
    let Some(tab) = api
        .active_tab
        .and_then(|id| api.tabs.iter().find(|t| t.id == id))
    else {
        return div();
    };
    let tab_id = tab.id;

    v_flex()
        .gap(rems(1.))
        .child(
            h_flex()
                .items_center()
                .gap(rems(0.5))
                .child(div().flex_1().child(TableCaption::new().child("Query Parameters")))
                .child(
                    Button::new("add-qp")
                        .small()
                        .icon(IconName::Plus)
                        .ghost()
                        .on_click(
                            cx.listener(move |this: &mut ApiClient, _event, window, cx| {
                                new_query_param(this, window, cx, tab_id);
                                cx.notify();
                            }),
                        ),
                ),
        )
        .child(
            Table::new()
                .child(
                    TableHeader::new().child(
                        TableRow::new()
                            .child(TableHead::new().w(rems(2.5)).child(""))
                            .child(TableHead::new().flex_1().child("Key"))
                            .child(TableHead::new().flex_1().child("Value"))
                            .child(TableHead::new().w(rems(2.5)).child("")),
                    ),
                )
                .child(
                    TableBody::new().children(tab.query_params.iter().enumerate().map(
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
                                            cx.listener(move |_this: &mut ApiClient, checked: &bool, _window, cx| {
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
                                            .xsmall()
                                            .icon(IconName::Delete)
                                            .on_click({
                                                let entity = entity.clone();
                                                cx.listener(move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                                                    if let Some(target_tab) =
                                                        this.tabs.iter_mut().find(|t| t.id == tab_id)
                                                    {
                                                        target_tab
                                                            .query_params
                                                            .retain(|q| q.entity_id() != entity.entity_id());
                                                    }
                                                    cx.notify();
                                                })
                                            }),
                                    ),
                                )
                        },
                    )),
                ),
        )
}
