use gpui::*;
use gpui_component::input::{Input, InputState, TabSize};
use gpui_component::scroll::ScrollableElement;
use gpui_component::tab::{self, Tab, TabBar};
use gpui_component::tag::Tag;
use gpui_component::{ActiveTheme, ColorName, IconName, Sizable, StyledExt, h_flex, v_flex};

use crate::http::Response;

fn format_size(bytes: usize) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
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
        let body_text = response.body.clone();
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
                .xsmall(),
        )
        // .child(div().text_sm().child(data.status_text.clone()))
    }

    fn render_duration(data: &Response) -> impl IntoElement {
        div().text_sm().child(format!("{:?}", data.duration))
    }

    fn render_headers_table(headers: &[(String, String)], cx: &App) -> impl IntoElement {
        use gpui_component::StyledExt;
        use gpui_component::scroll::ScrollableElement;

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
                        .map(|d| d.headers.as_slice())
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
