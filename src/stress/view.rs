use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::chart::AreaChart;
use gpui_kit::component::input::{Input, NumberInput};
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::model::{DataPoint, StressTesting};
use crate::helpers::render_method_tag;
use crate::icons::IconName;

fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs_f64();
    if total_secs >= 60.0 {
        format!("{}m {:02.0}s", total_secs as u64 / 60, total_secs % 60.0)
    } else {
        format!("{:.1}s", total_secs)
    }
}

impl StressTesting {
    fn config_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.run_config(cx);
        let is_running = self.is_running();

        self.sync_url_display(&config.url, window, cx);

        div()
            .v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .w_full()
                    .gap_2()
                    .child(
                        div().flex_1().child(
                            Input::new(&self.url_display())
                                .disabled(true)
                                .w_full()
                                .prefix(render_method_tag(&config.method)),
                        ),
                    )
                    .child(
                        Button::new("send")
                            .when(is_running, |this| this.danger())
                            .when(!is_running, |this| this.primary())
                            .icon(if is_running {
                                IconName::Stop
                            } else {
                                IconName::Send
                            })
                            .label(if is_running { "Stop" } else { "Start" })
                            .on_click(
                                cx.listener(|this, _, window, cx| this.toggle_run(window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("RPS:"),
                            )
                            .child(NumberInput::new(&self.rps_input()).w(px(140.))),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Duration (sec):"),
                            )
                            .child(NumberInput::new(&self.duration_input()).w(px(120.))),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Elapsed:"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child(format_duration(self.elapsed())),
                            ),
                    ),
            )
    }

    fn stats_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.stats();
        div()
            .h_flex()
            .w_full()
            .justify_between()
            .px_4()
            .py_3()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(self.stat_block("Total", stats.total_requests.to_string(), None, cx))
            .child(self.stat_block(
                "Success",
                stats.successful_requests.to_string(),
                None,
                cx,
            ))
            .child(self.stat_block("Failed", stats.failed_requests.to_string(), None, cx))
            .child(self.stat_block(
                "Success Rate",
                format!("{:.1}%", stats.success_rate()),
                None,
                cx,
            ))
            .child(self.stat_block(
                "Avg Latency",
                format!("{:.1}ms", stats.avg_latency_ms),
                None,
                cx,
            ))
    }

    fn stat_block(
        &self,
        label: &str,
        value: String,
        color: Option<Hsla>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(color.unwrap_or(cx.theme().muted_foreground))
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .when_some(color, |this, c| this.text_color(c))
                    .child(value),
            )
    }

    fn graph_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let data = self.data_points();
        div()
            .v_flex()
            .flex_1()
            .w_full()
            .min_h_0()
            .px_4()
            .py_3()
            .gap_1()
            .child(div().text_sm().font_semibold().child("Response Time"))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Latency (ms) per request"),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .pt_4()
                    .when(!data.is_empty(), |this| {
                        this.child(
                            AreaChart::new(data.clone())
                                .x(|d: &DataPoint| d.x.clone())
                                .y(|d: &DataPoint| if d.success { d.y } else { 0.0 })
                                .stroke(cx.theme().chart_1)
                                .fill(transparent_black())
                                .name("Latency")
                                .linear()
                                .y(|d: &DataPoint| if d.success { 0.0 } else { d.y })
                                .stroke(cx.theme().danger)
                                .fill(transparent_black())
                                .name("Failed")
                                .tick_margin((data.len() / 10).max(1))
                                .id("stress-chart"),
                        )
                    }),
            )
    }
}

impl Render for StressTesting {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .p_4()
            .bg(cx.theme().background)
            .child(self.config_bar(window, cx))
            .child(div().h(px(1.)).w_full().my_3().bg(cx.theme().border))
            .child(self.stats_panel(cx))
            .child(div().h(px(1.)).w_full().my_3().bg(cx.theme().border))
            .child(self.graph_panel(cx))
    }
}
