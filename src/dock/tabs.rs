use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::dock::{DockArea, Panel, PanelEvent, PanelId, PanelView};
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

use crate::dock::skin::TabChrome;

// ---------------------------------------------------------------------------
// CenterTab: the contract every center-tab view follows.
//
// Mirrors the original app's `Playground`/`PlaygroundHandle` pair: behavior
// lives on the view, a clone-box handle erases the type for managers, and
// any component implementing this opens as a tab with full chrome and
// close semantics and zero skin changes.
// ---------------------------------------------------------------------------

pub trait CenterTab: Panel {
    fn tab_label(&self, cx: &App) -> SharedString;

    fn tab_prefix(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    fn tab_suffix(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    /// Dirty guard hook: return false to refuse closing (host may prompt).
    fn can_close(&self, _cx: &App) -> bool {
        true
    }

    fn tab_chrome(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TabChrome {
        TabChrome {
            prefix: self.tab_prefix(window, cx),
            label: self.tab_label(cx),
            suffix: self.tab_suffix(window, cx),
        }
    }
}

pub trait CenterTabHandle {
    fn tab_label(&self, cx: &App) -> SharedString;
    fn tab_chrome(&self, window: &mut Window, cx: &mut App) -> TabChrome;
    fn panel_id(&self, cx: &App) -> PanelId;
    fn view(&self) -> AnyView;
    fn can_close(&self, cx: &App) -> bool;
    fn clone_box(&self) -> Box<dyn CenterTabHandle>;
}

impl<T: CenterTab> CenterTabHandle for Entity<T> {
    fn tab_label(&self, cx: &App) -> SharedString {
        self.read(cx).tab_label(cx)
    }

    fn tab_chrome(&self, window: &mut Window, cx: &mut App) -> TabChrome {
        self.update(cx, |this, cx| this.tab_chrome(window, cx))
    }

    fn panel_id(&self, _cx: &App) -> PanelId {
        PanelId::from(self.entity_id())
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn can_close(&self, cx: &App) -> bool {
        self.read(cx).can_close(cx)
    }

    fn clone_box(&self) -> Box<dyn CenterTabHandle> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// TabChromeRegistry: panel_name -> chrome fn. The skin asks the registry,
// never host types, so new tab kinds plug in with one `register` call and
// unknown panels fall back to a plain name label.
// ---------------------------------------------------------------------------

/// panel_name of the invisible keep-alive panel. The skin filters it out
/// of strips by name (not by `visible()`) so base keeps counting it:
/// with it present, closing the last real tab stays legal and the group
/// survives with zero real tabs.
pub const KEEP_ALIVE_NAME: &str = "empty-tab";

type ChromeFn = Rc<dyn Fn(&Arc<dyn PanelView>, &mut Window, &mut App) -> TabChrome>;
type CanCloseFn = Rc<dyn Fn(&Arc<dyn PanelView>, &App) -> bool>;
type RemoveFn =
    Rc<dyn Fn(&Arc<dyn PanelView>, &Entity<DockArea>, &mut Window, &mut App) -> bool>;

#[derive(Default)]
pub struct TabChromeRegistry {
    items: HashMap<&'static str, ChromeFn>,
    closers: HashMap<&'static str, CanCloseFn>,
    removers: HashMap<&'static str, RemoveFn>,
}

impl TabChromeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop a tab kind (its chrome + close guard) by name.
    pub fn unregister(&mut self, name: &'static str) {
        self.items.remove(name);
        self.closers.remove(name);
        self.removers.remove(name);
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.closers.clear();
        self.removers.clear();
    }

    pub fn register<T: CenterTab>(&mut self, name: &'static str) {
        let f: ChromeFn = Rc::new(|panel, window, cx| {
            panel
                .as_any()
                .downcast_ref::<Entity<T>>()
                .map(|e| e.update(cx, |this, cx| this.tab_chrome(window, cx)))
                .unwrap_or_else(|| TabChrome::label(panel.panel_name(cx)))
        });
        self.items.insert(name, f);
        let c: CanCloseFn = Rc::new(|panel, cx| {
            panel
                .as_any()
                .downcast_ref::<Entity<T>>()
                .map(|e| e.read(cx).can_close(cx))
                .unwrap_or(true)
        });
        self.closers.insert(name, c);
        // Typed removal through the public DockArea API, which (unlike the
        // group's close path) is not gated on the area lock — the lock
        // exists to forbid splits, not to trap tabs.
        let r: RemoveFn = Rc::new(|panel, area, window, cx| {
            panel
                .as_any()
                .downcast_ref::<Entity<T>>()
                .map(|e| {
                    area.update(cx, |area, cx| {
                        area.remove_panel(e.clone(), window, cx);
                    });
                    true
                })
                .unwrap_or(false)
        });
        self.removers.insert(name, r);
    }

    pub fn chrome(
        &self,
        panel: &Arc<dyn PanelView>,
        window: &mut Window,
        cx: &mut App,
    ) -> TabChrome {
        self.items
            .get(panel.panel_name(cx))
            .map(|f| f(panel, window, cx))
            .unwrap_or_else(|| TabChrome::label(panel.panel_name(cx)))
    }

    /// Dirty guard for the strip close button. Unregistered types pass.
    pub fn can_close(&self, panel: &Arc<dyn PanelView>, cx: &App) -> bool {
        self.closers
            .get(panel.panel_name(cx))
            .map(|f| f(panel, cx))
            .unwrap_or(true)
    }

    /// Remove a tab through its registered remover. Returns false when the
    /// panel kind is unknown (nothing removed).
    pub fn close_panel(
        &self,
        panel: &Arc<dyn PanelView>,
        area: &Entity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.removers
            .get(panel.panel_name(cx))
            .map(|f| f(panel, area, window, cx))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// EmptyTab: invisible keep-alive panel. Appended LAST in center tab groups
// so a group with zero real tabs survives normalization: the strip keeps
// rendering (with its empty state + [+]) instead of the pane vanishing.
// The skin filters invisible panels out of the strip; indices are base
// indices and stay stable.
// ---------------------------------------------------------------------------

pub struct EmptyTab {
    focus: FocusHandle,
    hint: SharedString,
}

impl EmptyTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            hint: "No open tabs".into(),
        }
    }

    pub fn with_hint(hint: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            hint: hint.into(),
        }
    }
}

impl Panel for EmptyTab {
    fn panel_name(&self) -> &'static str {
        KEEP_ALIVE_NAME
    }

    // Deliberately VISIBLE: base counts visible panels for its
    // last-panel/draggable/close rules, so only a counted keep-alive keeps
    // closing the last real tab legal. The skin filters it out of strips
    // by name and it renders Empty, so users never see it.
    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> bool {
        false
    }
}

impl EventEmitter<PanelEvent> for EmptyTab {}

impl Focusable for EmptyTab {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for EmptyTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The empty state lives HERE in the playground content, never in
        // the tab strip: with zero real tabs active falls onto this panel
        // and the content shows the message while the strip stays clean.
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.hint.clone()),
            )
    }
}
