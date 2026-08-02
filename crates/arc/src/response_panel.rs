// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use gpui::*;
use ui::button::{Button, ButtonVariants};
use ui::input::{Input, InputState, TabSize};
use ui::popover::Popover;
use ui::scroll::ScrollableElement;
use ui::tab::{self, Tab, TabBar};
use ui::tag::Tag;
use ui::{ActiveTheme, ColorName, Icon, Sizable, StyledExt, h_flex, v_flex};

use crate::icons::IconName;

use crate::http::Response;

fn format_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;

    let bytes = bytes as f64;

    if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}

#[derive(Clone)]
pub struct ResponsePanel {
    show: bool,
    selected_config: usize,
    body: Entity<InputState>,
    data: Option<Response>,
}

impl ResponsePanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let body = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("json")
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 10,
                    hard_tabs: false,
                })
                .default_value("")
        });

        Self {
            show: false,
            selected_config: 0,
            body,
            data: None,
        }
    }

    pub fn is_shown(&self) -> bool {
        self.show
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.show = !self.show;
        cx.notify();
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.show = true;
        cx.notify();
    }

    pub fn set_response(
        &mut self,
        response: Response,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let body_text = response.body.body.clone();
        let body_text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_text) {
            serde_json::to_string_pretty(&json).unwrap_or(body_text)
        } else {
            body_text
        };
        self.body
            .update(cx, |state, cx| state.set_value(body_text, window, cx));
        self.data = Some(response);
        self.show = true;
        cx.notify();
    }

    fn render_status_tag(data: &Response) -> impl IntoElement {
        let color = match data.status_code {
            200..=299 => ColorName::Green,
            300..=399 => ColorName::Yellow,
            _ => ColorName::Red,
        };

        h_flex().gap(px(8.)).items_center().child(
            Tag::color(color)
                .child(format!(
                    "{} {}",
                    data.status_code.to_string(),
                    data.status_text.to_string()
                ))
                .outline()
                .xsmall(),
        )
    }

    fn render_duration(data: &Response) -> impl IntoElement {
        let ms = data.duration.as_secs_f64() * 1000.0;

        Button::new("time-duration")
            .label(format!("{:.2} ms", ms))
            .ghost()
            .xsmall()
        // .tooltip("duration")
    }

    fn render_size(data: &Response) -> impl IntoElement {
        let req_hdr = format_size(data.request.header_size);
        let req_body = format_size(data.request.body_size);
        let req_total = format_size(data.request.size);

        let res_hdr = format_size(data.headers.response_size);
        let res_body = format_size(data.body.response_size);
        let res_total = format_size(data.response_size);

        Popover::new("response-size-popover")
            .anchor(Anchor::BottomLeft)
            .trigger(
                Button::new("response-size")
                    .label(res_total.clone())
                    .ghost()
                    .xsmall(),
            )
            .content(move |_, _window, cx| {
                let theme = cx.theme();

                let row = |label: SharedString, value: SharedString| {
                    h_flex()
                        .justify_between()
                        .items_center()
                        .w_full()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(label),
                        )
                        .child(div().text_xs().font_medium().child(value))
                };

                v_flex()
                    .p(px(10.))
                    .gap(px(10.))
                    .min_w(px(220.))
                    // Request
                    .child(
                        v_flex()
                            .gap(px(6.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        Icon::new(IconName::ArrowUp)
                                            .small()
                                            // .bg(cx.theme().background)
                                            .text_color(theme.danger),
                                    )
                                    .child(div().text_sm().font_semibold().child("Request")),
                            )
                            .child(row("Headers".into(), req_hdr.clone().into()))
                            .child(row("Body".into(), req_body.clone().into()))
                            .child(row("Total".into(), req_total.clone().into())),
                    )
                    .child(div().h(px(1.)).w_full().bg(theme.border))
                    // Response
                    .child(
                        v_flex()
                            .gap(px(6.))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        Icon::new(IconName::ArrowDown)
                                            .small()
                                            // .bg(cx.theme().background)
                                            .text_color(theme.success),
                                    )
                                    .child(div().text_sm().font_semibold().child("Response")),
                            )
                            .child(row("Headers".into(), res_hdr.clone().into()))
                            .child(row("Body".into(), res_body.clone().into()))
                            .child(row("Total".into(), res_total.clone().into())),
                    )
            })
    }
    fn render_headers_table(headers: &[(String, String)], cx: &App) -> impl IntoElement {
        use ui::StyledExt;
        use ui::scroll::ScrollableElement;

        let theme = cx.theme();

        div()
            .id("response-headers-vscroll")
            .w_full()
            .h_full()
            .min_h(px(0.))
            .min_w(px(0.))
            .overflow_y_scrollbar()
            .child(
                div()
                    .id("response-headers-hscroll")
                    .w_full()
                    .min_w(px(0.))
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex_col()
                            .min_w(px(432.))
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
                            .children(headers.iter().map(|(key, value)| {
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
                                            .child(key.clone()),
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
                                            .child(value.clone()),
                                    )
                            })),
                    ),
            )
    }
}

impl Render for ResponsePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("response-panel")
            .w_full()
            .min_w(px(0.))
            .h_full()
            .min_h(px(0.))
            .v_flex()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .w_full()
                    .flex_none()
                    .px(px(24.))
                    .py_2()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().text_sm().font_semibold().child("Response"))
                    .child(self.data.as_ref().map_or_else(
                        || div().into_any_element(),
                        |data| {
                            h_flex()
                                .gap(px(12.))
                                .items_center()
                                .child(Self::render_status_tag(data))
                                .child(Self::render_duration(data))
                                .child(Self::render_size(data))
                                .into_any_element()
                        },
                    )),
            )
            .child(
                TabBar::new("response-config")
                    .w_full()
                    .flex_none()
                    .px(px(24.))
                    .with_variant(tab::TabVariant::Underline)
                    .selected_index(self.selected_config)
                    .on_click(cx.listener(|this: &mut Self, idx: &usize, _window, cx| {
                        this.selected_config = *idx;
                        cx.notify();
                    }))
                    .child(Tab::new().label("Body"))
                    .child(Tab::new().label("Headers")),
            )
            .child(match self.selected_config {
                0 => div()
                    .id("response-body-hscroll")
                    .flex_1()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .overflow_hidden()
                    .px(px(24.))
                    .child(Input::new(&self.body).flex_1().h_full().appearance(false))
                    .into_any_element(),
                1 => {
                    let headers = self
                        .data
                        .as_ref()
                        .map(|d| d.headers.headers.as_slice())
                        .unwrap_or(&[]);
                    div()
                        .flex_1()
                        .min_h(px(0.))
                        .min_w(px(0.))
                        .overflow_y_scrollbar()
                        .px(px(24.))
                        .child(Self::render_headers_table(headers, cx))
                        .into_any_element()
                }
                _ => div().child("issue").into_any_element(),
            })
    }
}
