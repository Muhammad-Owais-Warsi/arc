use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

pub enum HeadersEvent {
    Changed,
}

impl EventEmitter<HeadersEvent> for Headers {}

#[derive(Clone)]
struct Header {
    key: Entity<InputState>,
    value: Entity<InputState>,
    active: bool,
}

pub struct Headers {
    rows: Vec<Header>,
}

impl Headers {
    pub fn new() -> Self {
        Self { rows: vec![] }
    }

    pub fn rows(&self, cx: &App) -> Vec<(String, String, bool)> {
        self.rows
            .iter()
            .map(|h| {
                (
                    h.key.read(cx).value().to_string(),
                    h.value.read(cx).value().to_string(),
                    h.active,
                )
            })
            .collect()
    }

    pub fn row_inputs(&self) -> Vec<(Entity<InputState>, Entity<InputState>, bool)> {
        self.rows
            .iter()
            .map(|h| (h.key.clone(), h.value.clone(), h.active))
            .collect()
    }

    pub fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = cx.new(|cx| InputState::new(window, cx));
        let value = cx.new(|cx| InputState::new(window, cx));
        self.watch(key.clone(), value.clone(), window, cx);
        self.rows.push(Header {
            key,
            value,
            active: true,
        });
        cx.emit(HeadersEvent::Changed);
        cx.notify();
    }

    pub fn remove_row(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.rows.len() {
            self.rows.remove(ix);
            cx.emit(HeadersEvent::Changed);
            cx.notify();
        }
    }

    pub fn set_row_active(&mut self, ix: usize, active: bool, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get(ix) {
            self.rows[ix] = Header {
                key: row.key.clone(),
                value: row.value.clone(),
                active,
            };
            cx.emit(HeadersEvent::Changed);
            cx.notify();
        }
    }

    fn watch(
        &mut self,
        key: Entity<InputState>,
        value: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(&key, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(HeadersEvent::Changed);
            }
        })
        .detach();
        cx.subscribe_in(&value, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(HeadersEvent::Changed);
            }
        })
        .detach();
    }

    pub fn load_from_json(
        &mut self,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(arr) = data.get("headers").and_then(|v| v.as_array()) {
            let rows: Vec<Header> = arr
                .iter()
                .map(|item| {
                    let key_str = item.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let val_str = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    let active = item.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
                    let key = cx.new(|cx| InputState::new(window, cx).default_value(key_str));
                    let value = cx.new(|cx| InputState::new(window, cx).default_value(val_str));
                    Header { key, value, active }
                })
                .collect();

            self.rows = rows;
            let watched: Vec<(Entity<InputState>, Entity<InputState>)> = self
                .rows
                .iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect();
            for (key, value) in watched {
                self.watch(key, value, window, cx);
            }
        }
    }
}
