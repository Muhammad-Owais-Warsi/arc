use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::input::InputContentType;
use gpui_component::{
    IndexPath,
    input::{Input, InputEvent, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};
use serde::{Deserialize, Serialize};

pub enum AuthEvent {
    Changed,
}

impl EventEmitter<AuthEvent> for Auth {}

#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum AuthType {
    #[default]
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
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Password")
                .masked(true)
        });
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Token")
                .masked(true)
        });

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
                    cx.emit(AuthEvent::Changed);
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(&username, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(AuthEvent::Changed);
            }
        })
        .detach();
        cx.subscribe_in(&password, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(AuthEvent::Changed);
            }
        })
        .detach();
        cx.subscribe_in(&token, window, |_, _, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(AuthEvent::Changed);
            }
        })
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
                    .child(
                        Input::new(&self.password)
                            .content_type(InputContentType::Password)
                            .mask_toggle(),
                    ),
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
                    .child(
                        Input::new(&self.token)
                            .content_type(InputContentType::Password)
                            .mask_toggle(),
                    ),
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

    pub fn load_from_json(
        &mut self,
        data: &serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(auth) = data.get("auth") else {
            return;
        };
        let auth_type = match auth.get("auth_type").and_then(|v| v.as_str()) {
            Some("Bearer") => AuthType::Bearer,
            Some("Basic") => AuthType::Basic,
            _ => AuthType::None,
        };
        let username = auth.get("username").and_then(|v| v.as_str()).unwrap_or("");
        let password = auth.get("password").and_then(|v| v.as_str()).unwrap_or("");
        let token = auth.get("token").and_then(|v| v.as_str()).unwrap_or("");

        self.selected_auth_type = auth_type.clone();
        let row = match auth_type {
            AuthType::None => 0,
            AuthType::Bearer => 1,
            AuthType::Basic => 2,
        };
        self.auth_type.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::default().row(row)), window, cx);
        });
        self.username
            .update(cx, |s, cx| s.set_value(username, window, cx));
        self.password
            .update(cx, |s, cx| s.set_value(password, window, cx));
        self.token
            .update(cx, |s, cx| s.set_value(token, window, cx));
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
