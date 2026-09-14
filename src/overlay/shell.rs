use std::rc::Rc;

use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{IntoElement, MouseButton, MouseDownEvent};
use gpui_kit::*;

#[derive(Clone, Copy, Default)]
pub enum OverlayPosition {
    #[default]
    Center,
    TopCenter,
}

#[derive(Clone)]
pub struct OverlayRequest {
    pub content: Rc<dyn Fn() -> AnyElement>,
    pub on_cancel: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    pub autofocus: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    pub width: Pixels,
    pub backdrop_closable: bool,
    pub position: OverlayPosition,
}

#[derive(Clone)]
struct OverlayState {
    request: OverlayRequest,
    focus: FocusHandle,
    prev_focus: Option<FocusHandle>,
}

pub struct OverlayRoot {
    current: Option<OverlayState>,
}

pub struct GlobalOverlayRoot(pub Entity<OverlayRoot>);

impl Global for GlobalOverlayRoot {}

impl OverlayRoot {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn show(cx: &mut App, window: &mut Window, request: OverlayRequest) {
        let Some(global) = cx.try_global::<GlobalOverlayRoot>() else {
            return;
        };
        let entity = global.0.clone();
        let focus = cx.focus_handle();
        let prev_focus = window.focused(cx);
        let autofocus = request.autofocus.clone();
        entity.update(cx, |this, cx| {
            this.current = Some(OverlayState {
                request,
                focus,
                prev_focus,
            });
            cx.notify();
        });
        if let Some(autofocus) = autofocus {
            window.defer(cx, move |window, cx| {
                autofocus(window, cx);
            });
        }
    }

    pub fn hide(cx: &mut App, window: &mut Window, cancelled: bool) {
        let entity = cx
            .try_global::<GlobalOverlayRoot>()
            .map(|global| global.0.clone());
        let Some(entity) = entity else {
            return;
        };
        let taken = entity.update(cx, |this, cx| {
            let taken = this.current.take();
            cx.notify();
            taken
        });
        let Some(state) = taken else {
            return;
        };
        if cancelled {
            if let Some(on_cancel) = state.request.on_cancel {
                on_cancel(window, cx);
            }
        }
        if let Some(prev) = state.prev_focus {
            window.focus(&prev, cx);
        }
    }

    pub fn overlay(window: &mut Window, cx: &mut App) -> AnyElement {
        let current = cx
            .try_global::<GlobalOverlayRoot>()
            .and_then(|global| global.0.read(cx).current.clone());
        let Some(state) = current else {
            return div().into_any_element();
        };
        let request = state.request.clone();
        let backdrop_request = state.request.clone();
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(cx.theme().overlay)
                    .on_any_mouse_down(move |event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        if event.button == MouseButton::Left && backdrop_request.backdrop_closable
                        {
                            Self::hide(cx, window, true);
                        }
                    }),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .justify_center()
                    .when(
                        matches!(request.position, OverlayPosition::Center),
                        |this| this.items_center(),
                    )
                    .when(
                        matches!(request.position, OverlayPosition::TopCenter),
                        |this| {
                            let offset = window.viewport_size().height / 10.;
                            this.items_start().pt(offset)
                        },
                    )
                    .child(
                        div()
                            .w(request.width)
                            .occlude()
                            .bg(cx.theme().popover)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_lg()
                            .shadow_lg()
                            .p_0()
                            .key_context("Dialog")
                            .on_action(|_: &Cancel, window, cx| {
                                Self::hide(cx, window, true);
                            })
                            .track_focus(&state.focus)
                            .focus_trap("overlay", &state.focus)
                            .child((request.content)()),
                    ),
            )
            .into_any_element()
    }
}
