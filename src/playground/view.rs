use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::clipboard::Clipboard;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    ActiveTheme, StyledExt, h_flex,
    select::Select,
    tab::{self, Tab, TabBar},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::actions::CopyURL;
use crate::actions::CopyAsCode;
use crate::icons::IconName;

use super::model::RequestPlayground;

impl RequestPlayground {
    fn render_editor_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&self.method_input())))
            .child(
                div().flex_1().child(
                    Input::new(&self.url_input()).suffix(
                        h_flex().gap_1().items_center().child(
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    Clipboard::new("url-clip")
                                        .tooltip("Copy")
                                        .value(self.url_input().read(cx).value()),
                                )
                                .context_menu(move |menu, _, _| {
                                    menu.menu("Copy URL", Box::new(CopyURL))
                                        .menu("Copy as Code", Box::new(CopyAsCode))
                                }),
                        ),
                    ),
                ),
            )
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(self.is_dirty(), |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    })
                    .on_click(cx.listener(|this: &mut Self, _, _window, cx| {
                        this.save(cx);
                    })),
            )
            .child(
                Button::new("send")
                    .when(self.is_sending(), |this| this.danger())
                    .when(!self.is_sending(), |this| this.primary())
                    .icon(if self.is_sending() {
                        IconName::Stop
                    } else {
                        IconName::Send
                    })
                    .label(if self.is_sending() {
                        "Stop"
                    } else {
                        "Send"
                    })
                    .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                        if this.is_sending() {
                            this.cancel_pending();
                        } else {
                            this.send_request(window, cx);
                        }
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn render_config_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div().px(px(24.)).child(
                    TabBar::new("request-tabs")
                        .w_full()
                        .with_variant(tab::TabVariant::Underline)
                        .selected_index(self.selected_tab())
                        .child(Tab::new().label("Params"))
                        .child(Tab::new().label("Authorization"))
                        .child(Tab::new().label("Headers"))
                        .child(Tab::new().label("Body"))
                        .on_click(cx.listener(|this: &mut Self, idx: &usize, _window, cx| {
                            this.select_tab(*idx, cx);
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_config_content(&self, _cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .h_full()
            .min_h(px(0.))
            .child(
                div()
                    .w_full()
                    .h_full()
                    .min_h(px(0.))
                    .when(self.selected_tab() != 0, |this| {
                        this.absolute().top_0().left_0().right_0().hidden()
                    })
                    .child(self.query_view()),
            )
            .child(
                div()
                    .w_full()
                    .h_full()
                    .min_h(px(0.))
                    .when(self.selected_tab() != 1, |this| {
                        this.absolute().top_0().left_0().right_0().hidden()
                    })
                    .child(self.auth_view()),
            )
            .child(
                div()
                    .w_full()
                    .h_full()
                    .min_h(px(0.))
                    .when(self.selected_tab() != 2, |this| {
                        this.absolute().top_0().left_0().right_0().hidden()
                    })
                    .child(self.headers_view()),
            )
            .child(
                div()
                    .w_full()
                    .h_full()
                    .min_h(px(0.))
                    .when(self.selected_tab() != 3, |this| {
                        this.absolute().top_0().left_0().right_0().hidden()
                    })
                    .child(self.body_view()),
            )
            .into_any_element()
    }
}

impl Render for RequestPlayground {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The response lives in the shell's bottom aux slot now; center tabs
        // render the editor only.
        div()
            .on_action((cx.listener(Self::handle_copy_url)))
            .on_action((cx.listener(Self::handle_copy_as_code)))
            .size_full()
            .min_h(px(0.))
            .v_flex()
            .gap(px(16.))
            .child(
                div()
                    .flex_none()
                    .v_flex()
                    .px(px(24.))
                    .pt(rems(1.0))
                    .child(self.render_editor_bar(cx)),
            )
            .child(self.render_config_tabs(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .px(px(24.))
                    .child(self.render_config_content(cx)),
            )
    }
}
