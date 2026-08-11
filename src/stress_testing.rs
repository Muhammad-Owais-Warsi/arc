use gpui::*;

use crate::fs::{FileContent, read_request_file, read_request_method};
use crate::playground::Playground;
use crate::request_playground::RequestPlayground;
use crate::response_panel::ResponsePanel;
use std::path::Path;

pub enum StressTestingStatus {
    Running,
    Cancelled,
}

pub struct StressTesting {
    request_playground: Option<WeakEntity<RequestPlayground>>,
    path: String,
    request_per_second: usize,
    status: StressTestingStatus,
    duration: std::time::Duration,
}

impl StressTesting {
    pub fn new(request_playground: Option<WeakEntity<RequestPlayground>>, path: String) -> Self {
        Self {
            request_playground,
            path,
            request_per_second: 5,
            status: StressTestingStatus::Cancelled,
            duration: std::time::Duration::ZERO,
        }
    }

    fn config(&self, cx: &App) -> FileContent {
        if let Some(src) = self.request_playground.as_ref().and_then(|w| w.upgrade()) {
            src.read(cx).current_content(cx)
        } else {
            serde_json::from_value(read_request_file(Path::new(&self.path))).unwrap_or_default()
        }
    }
}

impl Playground for StressTesting {
    fn method(&self, cx: &App) -> String {
        "STRESS TEST".to_string()
    }

    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        None
    }
}

impl Render for StressTesting {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(div().child(format!("Stress Test: {}", self.path)))
    }
}
