use crate::dock::tabs::Playground;
use crate::fs;
use crate::fs::request::KeyValue;
use crate::ui::method_tag;
use gpui_kit::base::dock::{Panel, PanelEvent};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

pub enum EnvPlaygroundEvent {
    Renamed { old_name: String, new_name: String },
}

impl EventEmitter<EnvPlaygroundEvent> for EnvPlayground {}

#[derive(Clone)]
struct EnvRow {
    key: Entity<InputState>,
    value: Entity<InputState>,
    active: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<KeyValue>,
}

pub struct EnvPlayground {
    name: Entity<InputState>,
    initial_name: String,
    is_editing: bool,
    rows: Vec<EnvRow>,
    dirty: bool,
    initial: Vec<KeyValue>,
    focus: FocusHandle,
}

impl EnvPlayground {
    pub fn new(name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let variables = Self::read_env_from_disk(&name);

        let initial_name = name.clone();
        let name = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let rows = variables
            .iter()
            .map(|kv| {
                let key = cx.new(|cx| InputState::new(window, cx).default_value(&kv.key));
                let value = cx.new(|cx| InputState::new(window, cx).default_value(&kv.value));
                EnvRow {
                    key,
                    value,
                    active: kv.active,
                }
            })
            .collect();

        let initial = variables.clone();

        let mut this = Self {
            name,
            is_editing: false,
            rows,
            dirty: false,
            initial_name,
            initial,
            focus: cx.focus_handle(),
        };

        this.watch_all_inputs(window, cx);
        this
    }

    pub fn name(&self, cx: &App) -> String {
        self.name.read(cx).value().to_string()
    }

    pub fn name_input(&self) -> Entity<InputState> {
        self.name.clone()
    }

    pub fn editing(&self) -> bool {
        self.is_editing
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn row_inputs(&self) -> Vec<(Entity<InputState>, Entity<InputState>, bool)> {
        self.rows
            .iter()
            .map(|row| (row.key.clone(), row.value.clone(), row.active))
            .collect()
    }

    pub fn add_variable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = cx.new(|cx| InputState::new(window, cx));
        let value = cx.new(|cx| InputState::new(window, cx));
        self.watch_row(key.clone(), value.clone(), window, cx);
        self.rows.push(EnvRow {
            key,
            value,
            active: true,
        });
        self.evaluate_dirty(cx);
        cx.notify();
    }

    pub fn remove_variable(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.rows.len() {
            self.rows.remove(ix);
            self.evaluate_dirty(cx);
            cx.notify();
        }
    }

    pub fn set_variable_active(&mut self, ix: usize, active: bool, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get(ix) {
            self.rows[ix] = EnvRow {
                key: row.key.clone(),
                value: row.value.clone(),
                active,
            };
            self.evaluate_dirty(cx);
            cx.notify();
        }
    }

    pub fn commit_name(&mut self, cx: &mut Context<Self>) {
        let new_name = self.name.read(cx).value().to_string();
        if new_name != self.initial_name {
            let old_name = std::mem::replace(&mut self.initial_name, new_name.clone());
            fs::env::rename(&old_name, &new_name);
            cx.emit(EnvPlaygroundEvent::Renamed {
                old_name,
                new_name,
            });
        }
        self.is_editing = false;
        cx.notify();
    }

    fn read_env_from_disk(name: &str) -> Vec<KeyValue> {
        let content = fs::env::read_environments();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter()
            .find(|e| e.name == name)
            .map(|e| e.variables)
            .unwrap_or_default()
    }

    fn write_all_envs_to_disk(envs: &[Environment]) {
        if let Ok(json) = serde_json::to_string_pretty(envs) {
            fs::env::write_environments(&json);
        }
    }

    fn watch_all_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let watched: Vec<(Entity<InputState>, Entity<InputState>)> = self
            .rows
            .iter()
            .map(|row| (row.key.clone(), row.value.clone()))
            .collect();
        for (key, value) in watched {
            self.watch_row(key, value, window, cx);
        }
    }

    fn watch_row(
        &mut self,
        key: Entity<InputState>,
        value: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(&key, window, |this, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                this.evaluate_dirty(cx);
            }
        })
        .detach();
        cx.subscribe_in(&value, window, |this, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                this.evaluate_dirty(cx);
            }
        })
        .detach();
    }

    fn evaluate_dirty(&mut self, cx: &mut Context<Self>) {
        let current = self.current_content(cx);
        self.dirty = current != self.initial;
        cx.notify();
    }

    fn current_content(&self, cx: &mut Context<Self>) -> Vec<KeyValue> {
        self.rows
            .iter()
            .map(|row| KeyValue {
                key: row.key.read(cx).value().to_string(),
                value: row.value.read(cx).value().to_string(),
                active: row.active,
            })
            .collect()
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let variables = self.current_content(cx);

        let mut envs: Vec<Environment> =
            serde_json::from_str(&fs::env::read_environments()).unwrap_or_default();

        if let Some(env) = envs.iter_mut().find(|e| e.name == self.initial_name) {
            env.variables = variables;
        }

        Self::write_all_envs_to_disk(&envs);

        self.initial = self
            .rows
            .iter()
            .map(|row| KeyValue {
                key: row.key.read(cx).value().to_string(),
                value: row.value.read(cx).value().to_string(),
                active: row.active,
            })
            .collect();
        self.dirty = false;
        cx.notify();
    }

    pub fn enable_editing(&mut self, cx: &mut Context<Self>) {
        self.is_editing = true;
        cx.notify();
    }

    pub fn disable_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name.update(cx, |input, cx| {
            input.set_value(self.initial_name.clone(), window, cx);
        });
        self.is_editing = false;
        cx.notify();
    }
}

impl Panel for EnvPlayground {
    fn panel_name(&self) -> &'static str {
        "env"
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for EnvPlayground {}

impl Focusable for EnvPlayground {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Playground for EnvPlayground {
    fn tab_label(&self, cx: &App) -> SharedString {
        self.name(cx).into()
    }

    fn tab_prefix(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(method_tag("ENV").into_any_element())
    }
}

