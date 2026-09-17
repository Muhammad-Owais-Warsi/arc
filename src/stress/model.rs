use crate::dock::tabs::Playground;
use crate::fs;
use crate::fs::request::RequestFileContent;
use crate::helpers::render_method_tag;
use crate::http_request::HttpRequest;
use crate::playground::RequestPlayground;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::input::InputState;
use gpui_kit::*;
use std::path::Path;
use tokio_util::sync::CancellationToken;

use super::engine::{RequestMetric, StressEngine, StressTestConfig, StressTestStats};

enum StressTestingStatus {
    Running,
    Cancelled,
}

#[derive(Clone)]
pub struct DataPoint {
    pub timestamp: f64,
    pub x: String,
    pub y: f64,
    pub success: bool,
}

pub struct StressTesting {
    request_playground: Option<WeakEntity<RequestPlayground>>,
    path: String,
    tab_name: String,
    request_per_second: Entity<InputState>,
    status: StressTestingStatus,
    duration: Entity<InputState>,
    url_display: Entity<InputState>,
    data: Vec<DataPoint>,
    cancel_token: Option<CancellationToken>,
    stats: StressTestStats,
    started_at: Option<std::time::Instant>,
    elapsed: std::time::Duration,
    focus: FocusHandle,
}

impl StressTesting {
    pub fn new(
        request_playground: Option<WeakEntity<RequestPlayground>>,
        path: String,
        name: String,
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
            tab_name: name,
            request_per_second: rps_counter,
            status: StressTestingStatus::Cancelled,
            duration: duration_counter,
            url_display,
            data: vec![],
            cancel_token: None,
            stats: StressTestStats::new(),
            started_at: None,
            elapsed: std::time::Duration::ZERO,
            focus: cx.focus_handle(),
        }
    }

    pub fn run_config(&self, cx: &mut Context<Self>) -> RequestFileContent {
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

    pub fn is_running(&self) -> bool {
        matches!(self.status, StressTestingStatus::Running)
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at
            .map_or(self.elapsed, |started| started.elapsed())
    }

    pub fn stats(&self) -> &StressTestStats {
        &self.stats
    }

    pub fn data_points(&self) -> Vec<DataPoint> {
        self.data.clone()
    }

    pub fn rps_input(&self) -> Entity<InputState> {
        self.request_per_second.clone()
    }

    pub fn duration_input(&self) -> Entity<InputState> {
        self.duration.clone()
    }

    pub fn url_display(&self) -> Entity<InputState> {
        self.url_display.clone()
    }

    pub fn sync_url_display(
        &mut self,
        url: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.url_display.update(cx, |state, cx| {
            if state.value() != url {
                state.set_value(url, window, cx);
            }
        });
    }

    pub fn toggle_run(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

                let config = self.run_config(cx);
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

}

impl Panel for StressTesting {
    fn panel_name(&self) -> &'static str {
        "stress"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for StressTesting {}

impl Focusable for StressTesting {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Playground for StressTesting {
    fn tab_label(&self, _cx: &App) -> SharedString {
        self.tab_name.clone().into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(render_method_tag("STRESS TEST").into_any_element())
    }
}

