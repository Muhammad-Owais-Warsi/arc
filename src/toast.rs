use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable};
use gpui_kit::*;
use std::rc::Rc;
pub enum ToastVariant {
    Success,
    Error,
}
use gpui_kit::IntoElement;
#[derive(IntoElement)]
pub struct Toast {
    variant: ToastVariant,
    message: SharedString,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}
impl Toast {
    pub fn success(msg: impl Into<SharedString>) -> Self {
        Self {
            variant: ToastVariant::Success,
            message: msg.into(),
            on_close: None,
        }
    }
    pub fn error(msg: impl Into<SharedString>) -> Self {
        Self {
            variant: ToastVariant::Error,
            message: msg.into(),
            on_close: None,
        }
    }
    pub fn on_close(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }
}
impl RenderOnce for Toast {
    fn render(self, _w: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (icon, icon_color) = match self.variant {
            ToastVariant::Success => (IconName::Check, theme.success),
            ToastVariant::Error => (IconName::CircleX, theme.danger),
        };
        let on_close = self.on_close.clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .child(Icon::new(icon).size_4().text_color(icon_color))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(self.message.clone()),
            )
            .child(
                Button::new("toast-close")
                    .ghost()
                    .small()
                    .icon(IconName::Close)
                    .text_color(theme.muted_foreground)
                    .on_click(move |_, window, cx| {
                        if let Some(f) = on_close.clone() {
                            f(window, cx);
                        }
                    }),
            )
    }
}
