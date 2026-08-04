use std::collections::HashMap;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_component::{ActiveTheme, IndexPath, Sizable, h_flex, v_flex};

use crate::icons::IconName;

#[derive(Clone, Debug)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

impl Environment {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }
}

#[derive(Clone)]
pub struct EnvVar {
    pub key: Entity<InputState>,
    pub value: Entity<InputState>,
}

pub struct EnvironmentStore {
    pub environments: Vec<Environment>,
    pub active_name: Option<String>,
    rows: Vec<EnvVar>,
    select: Entity<SelectState<Vec<String>>>,
}

impl EnvironmentStore {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select = cx.new(|cx| SelectState::new(Vec::<String>::new(), None, window, cx));

        Self {
            environments: vec![],
            active_name: None,
            rows: vec![],
            select,
        }
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

    pub fn resolve(&self, input: &str) -> String {
        let Some(env) = self.active() else {
            return input.to_string();
        };
        let mut out = input.to_string();
        for (key, value) in &env.variables {
            out = out.replace(&format!("{{{{{key}}}}}"), value);
        }
        out
    }
}

impl Render for EnvironmentStore {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(rems(0.75))
            .w_full()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .w(px(130.))
                            .child(Select::new(&self.select).placeholder("Environment")),
                    )
                    .child(
                        Button::new("add-env-var")
                            .label("Add Variable")
                            .icon(IconName::Plus)
                            .tooltip("Add Variable")
                            .ghost()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let key = cx.new(|cx| InputState::new(window, cx));
                                let value = cx.new(|cx| InputState::new(window, cx));
                                this.rows.push(EnvVar { key, value });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .child(
                        Table::new()
                            .child(
                                TableHeader::new().w_full().child(
                                    TableRow::new()
                                        .child(TableHead::new().flex_1().child("Key"))
                                        .child(TableHead::new().flex_1().child("Value"))
                                        .child(TableHead::new().w(rems(2.5)).child("")),
                                ),
                            )
                            .child(TableBody::new().children(self.rows.iter().enumerate().map(
                                |(i, var)| {
                                    TableRow::new()
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&var.key)),
                                        )
                                        .child(
                                            TableCell::new()
                                                .flex_1()
                                                .child(Input::new(&var.value)),
                                        )
                                        .child(
                                            TableCell::new()
                                                .w(rems(2.5))
                                                .flex()
                                                .justify_end()
                                                .child(
                                                    Button::new(format!("del-env-var-{i}"))
                                                        .ghost()
                                                        .icon(IconName::Trash)
                                                        .on_click(cx.listener(
                                                            move |this, _, _window, cx| {
                                                                this.rows.remove(i);
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
