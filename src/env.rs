use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{Disableable, IndexPath, Sizable, h_flex, v_flex};

use crate::fs;
use crate::icons::IconName;
use crate::playground::Playground;
use crate::response_panel::ResponsePanel;

pub enum EnvStoreEvent {
    Changed,
    EnvironmentSwitched,
}

impl EventEmitter<EnvStoreEvent> for EnvironmentStore {}

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
    pub variables: Vec<fs::KeyValue>,
}

pub struct EnvironmentStore {
    pub environments: Vec<Environment>,
    pub active_name: Option<String>,
    rows: Vec<EnvRow>,
    select: Entity<SelectState<Vec<String>>>,
}

impl EnvironmentStore {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let names: Vec<String> = vec!["Local".into(), "Production".into()];
        let select = cx.new(|cx| {
            SelectState::new(
                names,
                Some(IndexPath::default().row(0)),
                window,
                cx,
            )
        });

        let environments = vec![
            Environment {
                name: "Local".into(),
                variables: vec![
                    fs::KeyValue { key: "base_url".into(), value: "http://localhost:3000".into(), active: true },
                    fs::KeyValue { key: "api_key".into(), value: "dev-key-123".into(), active: true },
                ],
            },
            Environment {
                name: "Production".into(),
                variables: vec![
                    fs::KeyValue { key: "base_url".into(), value: "https://api.example.com".into(), active: true },
                    fs::KeyValue { key: "api_key".into(), value: "prod-key-789".into(), active: true },
                ],
            },
        ];

        let mut this = Self {
            active_name: Some("Local".into()),
            rows: vec![],
            select,
            environments,
        };

        this.load_rows_from_active(window, cx);

        cx.subscribe_in(&this.select, window, |this: &mut Self, _, event, window, cx| {
            if let SelectEvent::Confirm(selected_name) = event {
                if let Some(name) = selected_name {
                    if this.environments.iter().any(|e| &e.name == name) {
                        this.active_name = Some(name.clone());
                        this.load_rows_from_active(window, cx);
                        cx.emit(EnvStoreEvent::EnvironmentSwitched);
                        cx.notify();
                    }
                }
            }
        })
        .detach();

        this
    }

    fn load_rows_from_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let vars = self
            .active()
            .map(|e| e.variables.clone())
            .unwrap_or_default();

        self.rows = vars
            .into_iter()
            .map(|kv| {
                let key = cx.new(|cx| InputState::new(window, cx).default_value(&kv.key));
                let value = cx.new(|cx| InputState::new(window, cx).default_value(&kv.value));
                self.watch(key.clone(), value.clone(), window, cx);
                EnvRow {
                    key,
                    value,
                    active: kv.active,
                }
            })
            .collect();
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
                cx.emit(EnvStoreEvent::Changed);
            }
        })
        .detach();
        cx.subscribe_in(&value, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(EnvStoreEvent::Changed);
            }
        })
        .detach();
    }

    fn update_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let names: Vec<String> = self.environments.iter().map(|e| e.name.clone()).collect();
        let selected_row = self
            .active_name
            .as_ref()
            .and_then(|name| self.environments.iter().position(|e| &e.name == name))
            .unwrap_or(0);
        self.select.update(cx, |state, cx| {
            state.set_items(names, window, cx);
            state.set_selected_index(
                Some(IndexPath::default().row(selected_row)),
                window,
                cx,
            );
        });
    }

    pub fn active(&self) -> Option<&Environment> {
        self.active_name
            .as_ref()
            .and_then(|name| self.environments.iter().find(|e| &e.name == name))
    }

    pub fn active_mut(&mut self) -> Option<&mut Environment> {
        let name = self.active_name.clone()?;
        self.environments.iter_mut().find(|e| e.name == name)
    }

    pub fn active_vars(&self, cx: &App) -> Vec<(String, String)> {
        self.rows
            .iter()
            .filter(|r| r.active)
            .map(|r| {
                (
                    r.key.read(cx).value().to_string(),
                    r.value.read(cx).value().to_string(),
                )
            })
            .collect()
    }

    pub fn resolve(&self, input: &str) -> String {
        let Some(env) = self.active() else {
            return input.to_string();
        };
        let mut out = input.to_string();
        for kv in &env.variables {
            if kv.active {
                out = out.replace(&format!("{{{{{}}}}}", kv.key), &kv.value);
            }
        }
        out
    }

    fn save(&self) {
        let data: Vec<Environment> = self
            .environments
            .iter()
            .map(|e| Environment {
                name: e.name.clone(),
                variables: e
                    .variables
                    .iter()
                    .map(|kv| fs::KeyValue {
                        key: kv.key.clone(),
                        value: kv.value.clone(),
                        active: kv.active,
                    })
                    .collect(),
            })
            .collect();

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(fs::environments_path(), json);
        }
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = fs::environments_path();
        if !std::path::Path::new(&path).exists() {
            return;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(envs) = serde_json::from_str::<Vec<Environment>>(&content) {
                if !envs.is_empty() {
                    self.environments = envs;
                    self.active_name = self.environments.first().map(|e| e.name.clone());
                    self.update_select(window, cx);
                    self.load_rows_from_active(window, cx);
                }
            }
        }
    }
}

impl Playground for EnvironmentStore {
    fn method(&self, _cx: &App) -> String {
        "ENV".to_string()
    }
    fn response_panel(&self, _cx: &App) -> Option<Entity<ResponsePanel>> {
        None
    }
}

impl Render for EnvironmentStore {
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
                        div().w(px(180.)).child(
                            Select::new(&self.select).placeholder("Environment"),
                        ),
                    )
                    .child(
                        Button::new("add-env")
                            .label("Add Env")
                            .icon(IconName::Plus)
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let name = format!("Environment {}", this.environments.len() + 1);
                                this.environments.push(Environment {
                                    name: name.clone(),
                                    variables: vec![],
                                });
                                this.active_name = Some(name);
                                this.update_select(window, cx);
                                this.load_rows_from_active(window, cx);
                                this.save();
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("delete-env")
                            .label("Delete Env")
                            .icon(IconName::Trash)
                            .ghost()
                            .disabled(self.environments.len() <= 1)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.environments.len() <= 1 {
                                    return;
                                }
                                if let Some(name) = this.active_name.clone() {
                                    this.environments.retain(|e| e.name != name);
                                    this.active_name = this.environments.first().map(|e| e.name.clone());
                                    this.update_select(window, cx);
                                    this.load_rows_from_active(window, cx);
                                    this.save();
                                    cx.notify();
                                }
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("add-var")
                            .label("Add Variable")
                            .icon(IconName::Plus)
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let key = cx.new(|cx| InputState::new(window, cx));
                                let value = cx.new(|cx| InputState::new(window, cx));
                                this.watch(key.clone(), value.clone(), window, cx);
                                this.rows.push(EnvRow {
                                    key,
                                    value,
                                    active: true,
                                });
                                cx.emit(EnvStoreEvent::Changed);
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
                                                            cx.emit(EnvStoreEvent::Changed);
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
                                                        .small()
                                                        .icon(IconName::Trash)
                                                        .on_click(cx.listener(
                                                            move |this, _, _window, cx| {
                                                                this.rows.remove(i);
                                                                cx.emit(EnvStoreEvent::Changed);
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
