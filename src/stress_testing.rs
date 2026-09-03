use crate::fs;
use crate::fs::request::RequestFileContent;
use crate::helpers::render_method_tag;
use crate::http_request::HttpRequest;
use crate::icons::IconName;
use crate::playground::Playground;
use crate::request_playground::RequestPlayground;
use crate::response_panel::ResponsePanel;
use crate::stress_engine::{RequestMetric, StressEngine, StressTestConfig, StressTestStats};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::chart::AreaChart;
use gpui_kit::component::input::{Input, InputState, NumberInput};
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub enum StressTestingStatus {
    Running,
    Cancelled,
}

#[derive(Clone)]
struct DataPoint {
    timestamp: f64,
    x: String,
    y: f64,
    success: bool,
}

fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs_f64();
    if total_secs >= 60.0 {
        format!("{}m {:02.0}s", total_secs as u64 / 60, total_secs % 60.0)
    } else {
        format!("{:.1}s", total_secs)
    }
}

pub struct StressTesting {
    request_playground: Option<WeakEntity<RequestPlayground>>,
    path: String,
    request_per_second: Entity<InputState>,
    status: StressTestingStatus,
    duration: Entity<InputState>,
    url_display: Entity<InputState>,
    data: Vec<DataPoint>,
    cancel_token: Option<CancellationToken>,
    stats: StressTestStats,
    started_at: Option<std::time::Instant>,
    elapsed: std::time::Duration,
}

impl StressTesting {
    pub fn new(
        request_playground: Option<WeakEntity<RequestPlayground>>,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let rps_counter = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Requests/sec")
                .default_value("5")
                .step(1.)
                .min(1.)
                .max(500.)
        });

        let duration_counter = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Duration (sec)")
                .default_value("10")
                .step(1.)
                .min(1.)
                .max(30.)
        });

        let url_display = cx.new(|cx| InputState::new(window, cx));

        Self {
            request_playground,
            path,
            request_per_second: rps_counter,
            status: StressTestingStatus::Cancelled,
            duration: duration_counter,
            url_display,
            data: vec![],
            cancel_token: None,
            stats: StressTestStats::new(),
            started_at: None,
            elapsed: std::time::Duration::ZERO,
        }
    }

    fn config(&self, cx: &mut Context<Self>) -> RequestFileContent {
        if let Some(src) = self.request_playground.as_ref().and_then(|w| w.upgrade()) {
            src.read(cx).current_content(cx)
        } else {
            serde_json::from_value(fs::request::read(Path::new(&self.path))).unwrap_or_default()
        }
    }

    fn duration_value(&self, cx: &App) -> std::time::Duration {
        let seconds: f64 = self.duration.read(cx).value().parse().unwrap_or(10.0);
        std::time::Duration::from_secs_f64(seconds)
    }

    fn rps_value(&self, cx: &App) -> usize {
        self.request_per_second
            .read(cx)
            .value()
            .parse()
            .unwrap_or(1)
    }

    fn is_running(&self) -> bool {
        matches!(self.status, StressTestingStatus::Running)
    }

    fn toggle_run(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        match self.status {
            StressTestingStatus::Running => {
                if let Some(token) = &self.cancel_token {
                    token.cancel();
                }
                self.cancel_token = None;
                // self.is_running = false;
                if self.started_at.is_some() {
                    self.elapsed = self
                        .started_at
                        .take()
                        .map_or(std::time::Duration::ZERO, |started| started.elapsed());
                }
                self.status = StressTestingStatus::Cancelled;
                cx.notify();
            }
            StressTestingStatus::Cancelled => {
                self.status = StressTestingStatus::Running;
                self.data.clear();
                self.stats = StressTestStats::new();
                self.started_at = Some(std::time::Instant::now());
                self.elapsed = std::time::Duration::ZERO;

                let config = self.config(cx);
                let request = HttpRequest::from_file_content(&config);
                let rps = self.rps_value(cx);
                let duration = self.duration_value(cx);

                let test_config = StressTestConfig {
                    request,
                    requests_per_second: rps,
                    duration_secs: duration.as_secs(),
                };

                let handle = StressEngine::start(test_config);
                let (cancel_token, mut metrics_rx) = handle.split();
                self.cancel_token = Some(cancel_token);

                cx.spawn(async move |this, cx| {
                    while let Some(metric) = metrics_rx.recv().await {
                        let _ = this.update(cx, |this, cx| {
                            this.process_metric(metric, cx);
                        });
                    }

                    let _ = this.update(cx, |this, cx| {
                        this.cancel_token = None;
                        if let Some(started) = this.started_at.take() {
                            this.elapsed = started.elapsed();
                        }
                        this.status = StressTestingStatus::Cancelled;
                        cx.notify();
                    });
                })
                .detach();

                cx.spawn(async move |this, cx| {
                    while this
                        .update(cx, |this, cx| {
                            if !this.is_running() {
                                return false;
                            }
                            cx.notify();
                            true
                        })
                        .unwrap_or(false)
                    {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                    }
                })
                .detach();

                cx.notify();
            }
        }
    }

    fn process_metric(&mut self, metric: RequestMetric, cx: &mut Context<Self>) {
        self.stats.update(&metric);

        let data_point = DataPoint {
            timestamp: metric.timestamp,
            x: format!("{:.1}s", metric.timestamp),
            y: metric.response_time_ms,
            success: metric.success,
        };

        let idx = self
            .data
            .partition_point(|p| p.timestamp <= data_point.timestamp);

        self.data.insert(idx, data_point);

        if self.data.len() > 100 {
            self.data.remove(0);
        }

        cx.notify();
    }

    fn config_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.config(cx);
        let is_running = self.is_running();
        let elapsed = self
            .started_at
            .map_or(self.elapsed, |started| started.elapsed());

        self.url_display.update(cx, |state, cx| {
            if state.value() != config.url {
                state.set_value(config.url.clone(), window, cx);
            }
        });

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
                            Input::new(&self.url_display)
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
                            .child(NumberInput::new(&self.request_per_second).w(px(140.))),
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
                            .child(NumberInput::new(&self.duration).w(px(120.))),
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
                                    .child(format_duration(elapsed)),
                            ),
                    ),
            )
    }

    fn stats_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.stat_block("Total", self.stats.total_requests.to_string(), None, cx))
            .child(self.stat_block(
                "Success",
                self.stats.successful_requests.to_string(),
                None,
                cx,
            ))
            .child(self.stat_block("Failed", self.stats.failed_requests.to_string(), None, cx))
            .child(self.stat_block(
                "Success Rate",
                format!("{:.1}%", self.stats.success_rate()),
                None,
                cx,
            ))
            .child(self.stat_block(
                "Avg Latency",
                format!("{:.1}ms", self.stats.avg_latency_ms),
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
                    .when(!self.data.is_empty(), |this| {
                        this.child(
                            AreaChart::new(self.data.clone())
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
                                .tick_margin((self.data.len() / 10).max(1))
                                .id("stress-chart"),
                        )
                    }),
            )
    }
}

impl Playground for StressTesting {
    fn method(&self, _cx: &App) -> String {
        "STRESS TEST".to_string()
    }
    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        None
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
