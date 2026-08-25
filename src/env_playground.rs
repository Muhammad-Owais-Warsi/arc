use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{ActiveTheme, Icon, h_flex, v_flex};

use crate::env_fs::EnvFileSystem;
use crate::icons::IconName;
use crate::playground::Playground;
use crate::request_fs::KeyValue;
use crate::response_panel::ResponsePanel;

pub enum EnvPlaygroundEvent {
    Saved { name: String },
    Deleted { name: String },
}

impl EventEmitter<EnvPlaygroundEvent> for EnvPlayground {}

#[derive(Clone)]
pub struct EnvRow {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
    pub active: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(default)]
    pub variables: Vec<KeyValue>,
}

pub struct EnvPlayground {
    name: String,
    rows: Vec<EnvRow>,
    dirty: bool,
    initial: Vec<KeyValue>,
}

impl EnvPlayground {
    pub fn new(name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let variables = Self::read_env_from_disk(&name);

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
            rows,
            dirty: false,
            initial,
        };

        this.watch_all_inputs(window, cx);
        this
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn read_env_from_disk(name: &str) -> Vec<KeyValue> {
        let content = EnvFileSystem::read_environment_variables();
        let envs: Vec<Environment> = serde_json::from_str(&content).unwrap_or_default();
        envs.into_iter()
            .find(|e| e.name == name)
            .map(|e| e.variables)
            .unwrap_or_default()
    }

    fn write_all_envs_to_disk(envs: &[Environment]) {
        if let Ok(json) = serde_json::to_string_pretty(envs) {
            EnvFileSystem::save_environment_variables(&json).ok();
        }
    }

    fn load_rows(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let variables = Self::read_env_from_disk(&self.name);

        self.initial = variables.clone();
        self.rows = variables
            .into_iter()
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

        self.watch_all_inputs(window, cx);
        self.dirty = false;
        cx.notify();
    }

    fn watch_all_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for row in &self.rows {
            let key = row.key.clone();
            let value = row.value.clone();
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

    fn save(&mut self, cx: &mut Context<Self>) {
        let variables = self.current_content(cx);

        let mut envs: Vec<Environment> =
            serde_json::from_str(&EnvFileSystem::read_environment_variables()).unwrap_or_default();

        if let Some(env) = envs.iter_mut().find(|e| e.name == self.name) {
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

    pub fn delete_env(&mut self, cx: &mut Context<Self>) {
        let mut envs: Vec<Environment> =
            serde_json::from_str(&EnvFileSystem::read_environment_variables()).unwrap_or_default();

        envs.retain(|e| e.name != self.name);
        Self::write_all_envs_to_disk(&envs);

        cx.emit(EnvPlaygroundEvent::Deleted {
            name: self.name.clone(),
        });
        cx.notify();
    }
}

impl Playground for EnvPlayground {
    fn method(&self, _cx: &App) -> String {
        "ENV".to_string()
    }
    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        None
    }
}

impl Render for EnvPlayground {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h(px(0.))
            .gap(rems(0.75))
            .px(px(24.))
            .pt(rems(1.0))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Variable).size(px(16.)))
                            .child(
                                div()
                                    .text_lg()
                                    .child(self.name.clone()),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("save-env")
                            .label("Save")
                            .tooltip("Save changes")
                            .when(self.dirty, |this| {
                                this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                            })
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.save(cx);
                            })),
                    )
                    .child(
                        Button::new("add-var")
                            .label("Add Variable")
                            .tooltip("Add new variable")
                            .icon(IconName::Plus)
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let key = cx.new(|cx| InputState::new(window, cx));
                                let value = cx.new(|cx| InputState::new(window, cx));
                                let k = key.clone();
                                let v = value.clone();
                                cx.subscribe_in(&k, window, |this, _, event, _window, cx| {
                                    if matches!(event, InputEvent::Change) {
                                        this.evaluate_dirty(cx);
                                    }
                                })
                                .detach();
                                cx.subscribe_in(&v, window, |this, _, event, _window, cx| {
                                    if matches!(event, InputEvent::Change) {
                                        this.evaluate_dirty(cx);
                                    }
                                })
                                .detach();
                                this.rows.push(EnvRow {
                                    key,
                                    value,
                                    active: true,
                                });
                                this.evaluate_dirty(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scrollbar()
                    .child(
                        Table::new()
                            .w_full()
                            .child(
                                TableHeader::new().w_full().child(
                                    TableRow::new()
                                        .child(TableHead::new().w(rems(2.5)).child(""))
                                        .child(TableHead::new().flex_1().child("Key"))
                                        .child(TableHead::new().flex_1().child("Value"))
                                        .child(TableHead::new().w(rems(2.5)).child("")),
                                ),
                            )
                            .child(TableBody::new().children(self.rows.iter().enumerate().map(
                                |(i, row)| {
                                    TableRow::new()
                                        .child(
                                            TableCell::new().w(rems(2.5)).child(
                                                gpui_component::checkbox::Checkbox::new(
                                                    format!("env-check-{i}"),
                                                )
                                                .checked(row.active)
                                                .on_click({
                                                    let key = row.key.clone();
                                                    let value = row.value.clone();
                                                    cx.listener(
                                                        move |this: &mut Self, checked: &bool, _window, cx| {
                                                            this.rows[i] = EnvRow {
                                                                key: key.clone(),
                                                                value: value.clone(),
                                                                active: *checked,
                                                            };
                                                            this.evaluate_dirty(cx);
                                                            cx.notify();
                                                        },
                                                    )
                                                }),
                                            ),
                                        )
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&row.key)),
                                        )
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&row.value)),
                                        )
                                        .child(
                                            TableCell::new()
                                                .w(rems(2.5))
                                                .flex()
                                                .justify_end()
                                                .child(
                                                    Button::new(format!("del-var-{i}"))
                                                        .ghost()
                                                        .icon(IconName::Trash)
                                                        .on_click(cx.listener(
                                                            move |this, _, _window, cx| {
                                                                this.rows.remove(i);
                                                                this.evaluate_dirty(cx);
                                                                cx.notify();
                                                            },
                                                        )),
                                                ),
                                        )
                                },
                            ))),
                    ),
            )
    }
}
