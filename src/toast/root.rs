use gpui_kit::IntoElement;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::time::Duration;

use super::view::{Toast, ToastVariant};

#[derive(Clone)]
pub struct ToastData {
    pub variant: ToastVariant,
    pub message: SharedString,
    pub placement: Anchor,
}

pub struct ToastRoot {
    pub(crate) current: Option<ToastData>,
}

pub struct GlobalToastRoot(pub Entity<ToastRoot>);

impl Global for GlobalToastRoot {}

impl ToastRoot {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn show(cx: &mut App, variant: ToastVariant, message: SharedString) {
        Self::show_placed(cx, variant, message, Anchor::BottomCenter);
    }

    pub fn show_placed(
        cx: &mut App,
        variant: ToastVariant,
        message: SharedString,
        placement: Anchor,
    ) {
        let Some(global) = cx.try_global::<GlobalToastRoot>() else {
            return;
        };
        let entity = global.0.clone();
        entity.update(cx, |this, cx| {
            this.current = Some(ToastData {
                variant,
                message,
                placement,
            });
            cx.notify();
        });
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_secs(4))
                .await;
            let _ = entity.update(cx, |this, cx| {
                this.current = None;
                cx.notify();
            });
        })
        .detach();
    }

    pub fn hide(cx: &mut App) {
        let entity = cx
            .try_global::<GlobalToastRoot>()
            .map(|global| global.0.clone());
        if let Some(entity) = entity {
            entity.update(cx, |this, cx| {
                this.current = None;
                cx.notify();
            });
        }
    }

    pub fn overlay(cx: &mut App) -> AnyElement {
        let current = cx
            .try_global::<GlobalToastRoot>()
            .and_then(|global| global.0.read(cx).current.clone());
        let Some(data) = current else {
            return Empty.into_any_element();
        };
        let placement = data.placement;
        div()
            .absolute()
            .size_full()
            .child(
                div()
                    .absolute()
                    .when(placement == Anchor::TopLeft, |this| {
                        this.top_6().left_6().flex().justify_start()
                    })
                    .when(placement == Anchor::TopCenter, |this| {
                        this.top_6().left_0().right_0().flex().justify_center()
                    })
                    .when(placement == Anchor::TopRight, |this| {
                        this.top_6().right_6().flex().justify_end()
                    })
                    .when(placement == Anchor::BottomLeft, |this| {
                        this.bottom_6().left_6().flex().justify_start()
                    })
                    .when(placement == Anchor::BottomCenter, |this| {
                        this.bottom_6().left_0().right_0().flex().justify_center()
                    })
                    .when(placement == Anchor::BottomRight, |this| {
                        this.bottom_6().right_6().flex().justify_end()
                    })
                    .when(placement == Anchor::LeftCenter, |this| {
                        this.left_6().top_0().bottom_0().flex().items_center()
                    })
                    .when(placement == Anchor::RightCenter, |this| {
                        this.right_6().top_0().bottom_0().flex().items_center()
                    })
                    .when_some(Some(data), |el, data| {
                        let toast = match data.variant {
                            ToastVariant::Success => Toast::success(data.message.clone()),
                            ToastVariant::Error => Toast::error(data.message.clone()),
                        };
                        el.child(toast.on_close(|_, cx| {
                            Self::hide(cx);
                        }))
                    }),
            )
            .into_any_element()
    }
}
