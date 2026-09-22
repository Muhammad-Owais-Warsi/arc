use gpui_kit::component::input::{EditorState, TabSize};
use gpui_kit::*;

use crate::http::Response;

const ASYNC_PRETTY_THRESHOLD: usize = 200_000;
const MAX_PRETTY_BYTES: usize = 1_500_000;

#[derive(Clone)]
pub struct ResponsePanel {
    show: bool,
    selected_config: usize,
    body: Entity<EditorState>,
    data: Option<Response>,
    formatting: bool,
    format_id: u64,
}

impl ResponsePanel {
    fn pretty_print_body(raw: String) -> String {
        serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|json| serde_json::to_string_pretty(&json).ok())
            .unwrap_or(raw)
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let body = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("json")
                .folding(true)
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
            formatting: false,
            format_id: 0,
        }
    }

    pub fn is_shown(&self) -> bool {
        self.show
    }

    pub fn has_response(&self) -> bool {
        self.data.is_some()
    }

    pub fn selected_tab(&self) -> usize {
        self.selected_config
    }

    pub fn select_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected_config = ix;
        cx.notify();
    }

    pub fn is_formatting(&self) -> bool {
        self.formatting
    }

    pub fn body_editor(&self) -> Entity<EditorState> {
        self.body.clone()
    }

    pub fn response_data(&self) -> Option<&Response> {
        self.data.as_ref()
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.show = !self.show;
        cx.notify();
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        if self.show {
            return;
        }
        self.show = true;
        cx.notify();
    }

    pub fn format_size(bytes: usize) -> String {
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

    pub fn set_response(
        &mut self,
        response: Response,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let raw = response.body.body.clone();
        let looks_json = matches!(raw.trim_start().chars().next(), Some('{') | Some('['));
        if raw.len() < ASYNC_PRETTY_THRESHOLD || !looks_json || raw.len() >= MAX_PRETTY_BYTES {
            let body_text = if looks_json && raw.len() < MAX_PRETTY_BYTES {
                Self::pretty_print_body(raw)
            } else {
                raw
            };
            self.body
                .update(cx, |state, cx| state.set_value(body_text, window, cx));
            self.data = Some(response);
            self.show = true;
            self.formatting = false;
            cx.notify();
            return;
        }
        self.format_id += 1;
        let my_id = self.format_id;
        self.data = Some(response);
        self.formatting = true;
        self.show = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let pretty = cx
                .background_executor()
                .spawn(async move { Self::pretty_print_body(raw) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.format_id != my_id {
                    return;
                }
                this.formatting = false;
                this.body
                    .update(cx, |state, cx| state.set_value(pretty, window, cx));
                cx.notify();
            });
        })
        .detach();
    }
}
