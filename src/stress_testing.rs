use crate::fs::{FileContent, read_request_file};
use crate::helpers::render_method_tag;
use crate::http_request::HttpRequest;
use crate::icons::IconName;
use crate::playground::Playground;
use crate::request_playground::RequestPlayground;
use crate::response_panel::ResponsePanel;
use crate::stress_engine::{RequestMetric, StressEngine, StressTestConfig, StressTestStats};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::chart::LineChart;
use gpui_component::input::{Input, InputState, NumberInput};
use gpui_component::{ActiveTheme, StyledExt};
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub enum StressTestingStatus {
    Running,
    Cancelled,
}

#[derive(Clone)]
struct DataPoint {
    x: String,
    y: f64,
}

pub struct StressTesting {
    request_playground: Option<WeakEntity<RequestPlayground>>,
    path: String,
    request_per_second: Entity<InputState>,
    status: StressTestingStatus,
    duration: Entity<InputState>,
    url_display: Entity<InputState>,
    data: Vec<DataPoint>,
    // Stress test state
    cancel_token: Option<CancellationToken>,
    stats: StressTestStats,
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
                .placeholder("Duration (min)")
                .default_value("1")
                .step(1.)
                .min(1.)
                .max(10.)
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
        }
    }

    fn config(&self, cx: &mut Context<Self>) -> FileContent {
        if let Some(src) = self.request_playground.as_ref().and_then(|w| w.upgrade()) {
            src.read(cx).current_content(cx)
        } else {
            serde_json::from_value(read_request_file(Path::new(&self.path))).unwrap_or_default()
        }
    }

    fn duration_value(&self, cx: &App) -> std::time::Duration {
        let minutes: f64 = self.duration.read(cx).value().parse().unwrap_or(1.0);
        std::time::Duration::from_secs_f64(minutes * 60.0)
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
                // STOP the test
                if let Some(token) = &self.cancel_token {
                    token.cancel();
                }
                self.cancel_token = None;
                self.status = StressTestingStatus::Cancelled;
                cx.notify();
            }
            StressTestingStatus::Cancelled => {
                self.status = StressTestingStatus::Running;
                self.data.clear();
                self.stats = StressTestStats::new();

                // Get configuration
                let config = self.config(cx);
                let request = HttpRequest::from_file_content(&config);
                let rps = self.rps_value(cx);
                let duration = self.duration_value(cx);

                // Create test config
                let test_config = StressTestConfig {
                    request,
                    requests_per_second: rps,
                    duration_secs: duration.as_secs(),
                    background_executor: cx.background_executor().clone(),
                };

                // Start the stress test
                let handle = StressEngine::start(test_config);
                let (cancel_token, mut metrics_rx) = handle.split();
                self.cancel_token = Some(cancel_token);

                // Spawn task to process incoming metrics
                cx.spawn(async move |this, cx| {
                    while let Some(metric) = metrics_rx.recv().await {
                        let _ = this.update(cx, |this, cx| {
                            this.process_metric(metric, cx);
                        });
                    }

                    // Test completed
                    let _ = this.update(cx, |this, cx| {
                        this.cancel_token = None;
                        this.status = StressTestingStatus::Cancelled;
                        cx.notify();
                    });
                })
                .detach();

                cx.notify();
            }
        }
    }

    fn process_metric(&mut self, metric: RequestMetric, cx: &mut Context<Self>) {
        // Update stats
        self.stats.update(&metric);

        // Add to chart data (keep last 100 points for performance)
        self.data.push(DataPoint {
            x: format!("{:.1}s", metric.timestamp),
            y: metric.response_time_ms,
        });

        if self.data.len() > 100 {
            self.data.remove(0);
        }

        cx.notify();
    }

    fn config_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.config(cx);
        let is_running = self.is_running();

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
                // URL + Start/Stop on the same line
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
                // RPS + Duration, packed together
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
                            .child(NumberInput::new(&self.request_per_second).w(px(100.))),
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
                                    .child("Duration (min):"),
                            )
                            .child(NumberInput::new(&self.duration).w(px(100.))),
                    ),
            )
    }

    fn stats_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_flex()
            .gap_6()
            .px_4()
            .py_3()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Total"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .child(self.stats.total_requests.to_string()),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().success_foreground)
                            .child("Success"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(cx.theme().success_foreground)
                            .child(self.stats.successful_requests.to_string()),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().danger_foreground)
                            .child("Failed"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(cx.theme().danger_foreground)
                            .child(self.stats.failed_requests.to_string()),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Success Rate"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .child(format!("{:.1}%", self.stats.success_rate())),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Avg Latency"),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .child(format!("{:.1}ms", self.stats.avg_latency_ms)),
                    ),
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
                    .child("Latency (ms) per request, live"),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .pt_4()
                    .when(!self.data.is_empty(), |this| {
                        this.child(
                            LineChart::new(self.data.clone())
                                .x(|d: &DataPoint| d.x.clone())
                                .y(|d: &DataPoint| d.y)
                                .stroke(cx.theme().chart_1)
                                .linear()
                                .dot(),
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
            .when(self.stats.total_requests > 0, |this| {
                this.child(self.stats_panel(cx))
                    .child(div().h(px(1.)).w_full().my_3().bg(cx.theme().border))
            })
            .child(self.graph_panel(cx))
    }
}
