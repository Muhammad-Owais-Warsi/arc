use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    input::Input,
    select::Select,
    v_flex,
};
use gpui_kit::*;

use super::model::{Auth, AuthType};

impl Auth {
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
                    .child(Input::new(&self.username_input())),
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
                    .child(Input::new(&self.password_input())),
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
                    .child(Input::new(&self.token_input())),
            )
            .into_any_element()
    }
}

impl Render for Auth {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(px(8.))
            .child(div().w(px(110.)).child(Select::new(&self.auth_type_select())))
            .child(match self.auth_type() {
                AuthType::Bearer => Self::bearer_auth(&self, cx).into_any_element(),
                AuthType::Basic => Self::basic_auth(&self, cx).into_any_element(),
                AuthType::None => div().into_any_element(),
            })
    }
}
