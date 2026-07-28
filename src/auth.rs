use gpui::*;
use gpui_component::{
    IndexPath,
    input::{Input, InputState},
    select::{Select, SelectEvent, SelectState},
    v_flex,
};
use gpui_component::ActiveTheme;

pub struct Auth {
    auth_type: Entity<SelectState<Vec<String>>>,
    selected_auth_type: usize,
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
                auth_types,
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

        cx.subscribe_in(
            &auth_type_state,
            window,
            |this: &mut Self, _, event, _window, cx| {
                if let SelectEvent::Confirm(Some(auth_type)) = event {
                    this.selected_auth_type = match auth_type.as_str() {
                        "Bearer" => 1,
                        "Basic Auth" => 2,
                        _ => 0,
                    };
                    cx.notify();
                }
            },
        )
        .detach();

        Self {
            auth_type: auth_type_state,
            selected_auth_type: selected,
            username,
            password,
            token,
        }
    }
}

impl Render for Auth {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(px(8.))
            .child(div().w(px(110.)).child(Select::new(&self.auth_type)))
            .child(match self.selected_auth_type {
                1 => v_flex()
                    .gap(rems(0.5))
                    .child(
                        div().child(
                            div().text_xs().text_color(cx.theme().muted_foreground).mb_1().child("Token"),
                        ).child(Input::new(&self.token)),
                    )
                    .into_any_element(),
                2 => v_flex()
                    .gap(rems(0.5))
                    .child(
                        div().child(
                            div().text_xs().text_color(cx.theme().muted_foreground).mb_1().child("Username"),
                        ).child(Input::new(&self.username)),
                    )
                    .child(
                        div().child(
                            div().text_xs().text_color(cx.theme().muted_foreground).mb_1().child("Password"),
                        ).child(Input::new(&self.password)),
                    )
                    .into_any_element(),
                _ => div().into_any_element(),
            })
    }
}
