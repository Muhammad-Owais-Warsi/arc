// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use gpui::*;
use ui::ActiveTheme;
use ui::{
    IndexPath,
    input::{Input, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};

#[derive(Clone, PartialEq)]
pub enum AuthType {
    None,
    Bearer,
    Basic,
}

pub struct Auth {
    auth_type: Entity<SelectState<Vec<String>>>,
    selected_auth_type: AuthType,
    username: Entity<InputState>,
    password: Entity<InputState>,
    token: Entity<InputState>,
}

impl Auth {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let auth_types: Vec<String> = vec!["No Auth", "Bearer", "Basic Auth"]
            .into_iter()
            .map(String::from)
            .collect();
        let selected = auth_types.iter().position(|m| *m == "No Auth").unwrap_or(0);

        let auth_type_state = cx.new(|cx| {
            SelectState::new(
                auth_types.clone(),
                Some(IndexPath {
                    section: 0,
                    row: selected,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        let username = cx.new(|cx| InputState::new(window, cx).placeholder("Username"));
        let password = cx.new(|cx| InputState::new(window, cx).placeholder("Password"));
        let token = cx.new(|cx| InputState::new(window, cx).placeholder("Token"));

        let auth_types_for_sub = auth_types.clone();

        cx.subscribe_in(
            &auth_type_state,
            window,
            move |this: &mut Self, _, event, _window, cx| {
                if let SelectEvent::Confirm(Some(idx)) = event {
                    this.selected_auth_type = match auth_types_for_sub.iter().position(|o| o == idx)
                    {
                        Some(1) => AuthType::Bearer,
                        Some(2) => AuthType::Basic,
                        _ => AuthType::None,
                    };
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            auth_type: auth_type_state,
            selected_auth_type: AuthType::None,
            username,
            password,
            token,
        }
    }

    fn basic_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(rems(0.5))
            .child(
                div()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .mb_1()
                            .child("Username"),
                    )
                    .child(Input::new(&self.username)),
            )
            .child(
                div()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .mb_1()
                            .child("Password"),
                    )
                    .child(Input::new(&self.password)),
            )
            .into_any_element()
    }

    fn bearer_auth(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(rems(0.5))
            .child(
                div()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .mb_1()
                            .child("Token"),
                    )
                    .child(Input::new(&self.token)),
            )
            .into_any_element()
    }

    pub fn auth_type(&self) -> AuthType {
        self.selected_auth_type.clone()
    }

    pub fn basic_auth_values(&self, cx: &App) -> (String, String) {
        let username = self.username.read(cx).value();
        let password = self.password.read(cx).value();

        (username.to_string(), password.to_string())
    }

    pub fn bearer_auth_value(&self, cx: &App) -> String {
        self.token.read(cx).value().to_string()
    }
}

impl Render for Auth {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(px(8.))
            .child(div().w(px(110.)).child(Select::new(&self.auth_type)))
            .child(match self.selected_auth_type {
                AuthType::Bearer => Self::bearer_auth(&self, cx).into_any_element(),
                AuthType::Basic => Self::basic_auth(&self, cx).into_any_element(),
                AuthType::None => div().into_any_element(),
            })
    }
}
