use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::AxisExt as _;
use gpui_kit::base::dock::{
    DockArea, DockAreaRenderer, DragPanel, DropIndicator, NodeId, PanelView, TabGroupContext,
    TabGroupRenderer, TileContext, TilesRenderer,
};
use gpui_kit::component::Selectable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::*;

use crate::icons::IconName;

use crate::dock::tabs::{KEEP_ALIVE_NAME, TabChromeRegistry};

// ---------------------------------------------------------------------------
// CleanSkin: base DockArea appearance with OUR tab bar (kit `TabBar`).
//
// This file is app-agnostic: tab chrome (label/prefix/suffix per panel
// type), the bar prefix/suffix slots and the [+ new tab] action are
// injected delegates; tabs themselves are the given kit `Tab`s, so the
// host app (arc-dock demo or the real gpui client) supplies chrome and
// changes nothing here.
//
// The skin only styles CENTER tab groups. Fixed side/bottom/aux panels
// live outside the dock area (see shell.rs) as plain resizable slots, so
// tab DnD physically cannot enter or leave them.
//
// Bar layout (Zed-style): `bar_prefix` (back/forward) + tabs + `bar_suffix`
// ([+] by default). Tabs are the given kit `Tab`s built from registry
// chrome (label/prefix/suffix + close); the skin applies selection,
// click-to-select, drag and policy-gated drops on top.
//
// Structural removals vs gpui-component's skin (not flag-hides):
// - no dock toggle buttons anywhere
// - no zoom affordance anywhere (panels report zoomable() = false,
//   ToggleZoom is refused, no button is drawn)
// - no ellipsis menu
// - no splits: area is locked so content drops never fire; tab-bar
//   reorder within the strip is the only DnD path
// - locked singles (one non-closable panel) render no strip at all
//
// Kept: reorder inside the strip (drag tabs, drop between tabs, drop past
// last tab), insertion highlight, click-to-select, per-tab close, suffix
// [+] (lands in the group whose button was clicked).
// ---------------------------------------------------------------------------

/// Custom tab chrome. The host maps its own panel types to a label plus
/// optional prefix (method pill, icon) and suffix (extra controls)
/// elements. Close (x) is drawn by the skin itself for closable panels.
pub struct TabChrome {
    pub prefix: Option<AnyElement>,
    pub label: SharedString,
    pub suffix: Option<AnyElement>,
}

impl TabChrome {
    pub fn label(label: impl Into<SharedString>) -> Self {
        Self {
            prefix: None,
            label: label.into(),
            suffix: None,
        }
    }
}

pub type TabChromeFn = Rc<dyn Fn(&Arc<dyn PanelView>, &mut Window, &mut App) -> TabChrome>;

/// What [+ new tab] does. Receives the live area plus the node of the
/// group whose [+] was clicked, so the host can `add_panel` then
/// `move_panel` the tab into that exact group.
/// `None` hides the [+] button entirely.
pub type AddTabAction =
    Rc<dyn Fn(&Entity<DockArea>, Option<NodeId>, &mut Window, &mut App)>;

/// Future-proof DnD switch. The area is locked exactly when splits are
/// off, so base installs no content drops; the strip always draws its own
/// reorder targets. Turning splits / cross-group moves on later is a
/// one-line policy change, no skin rewrite.
#[derive(Clone)]
pub struct DndPolicy {
    /// Master switch. `false` = no drag sources, no drop targets at all.
    pub enabled: bool,
    /// Accept drops whose source group differs from the target strip.
    /// `false` = same-strip reorder only.
    pub allow_cross_group: bool,
    /// Allow content-edge drops to split groups. `false` = locked area,
    /// no splits, no split dividers/indicators.
    pub allow_split: bool,
    /// Extra veto for cross-group drops. `None` = allow when
    /// `allow_cross_group` is set. Same-strip drops always pass.
    pub can_drop: Option<Rc<dyn Fn(NodeId, NodeId) -> bool>>,
}

impl DndPolicy {
    pub fn reorder_only() -> Self {
        Self {
            enabled: true,
            allow_cross_group: false,
            allow_split: false,
            can_drop: None,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            allow_cross_group: false,
            allow_split: false,
            can_drop: None,
        }
    }

    pub fn permissive() -> Self {
        Self {
            enabled: true,
            allow_cross_group: true,
            allow_split: true,
            can_drop: None,
        }
    }

    pub fn accepts(&self, source: NodeId, target: NodeId) -> bool {
        if !self.enabled {
            return false;
        }
        if source == target {
            return true;
        }
        if !self.allow_cross_group {
            return false;
        }
        self.can_drop
            .as_ref()
            .map(|f| f(source, target))
            .unwrap_or(true)
    }
}

impl Default for DndPolicy {
    fn default() -> Self {
        Self::reorder_only()
    }
}

/// Bar-level slot (Zed-style): back/forward buttons as `bar_prefix`,
/// [+ new tab] / menus as `bar_suffix`. Return `None` for no element.
pub type BarSlotFn = Rc<dyn Fn(&mut Window, &mut App) -> Option<AnyElement>>;

/// Knobs for hosts embedding the skin in their own app.
#[derive(Clone)]
pub struct SkinOptions {
    /// Show the [+] new-tab button in tab strips. Defaults to true.
    /// Ignored when `bar_suffix` supplies its own element.
    pub show_add_tab: bool,
    /// Show the per-tab close button. Defaults to true.
    pub show_close: bool,
    /// Tab strip height in px. Defaults to 32, matching the old bar.
    pub tab_height: f32,
    /// Content message when a group holds zero real tabs.
    pub empty_text: SharedString,
    /// Drag-and-drop behavior. Defaults to same-strip reorder only.
    pub dnd: DndPolicy,
    /// Leading bar element (e.g. back/forward like Zed). Defaults to none.
    pub bar_prefix: Option<BarSlotFn>,
    /// Trailing bar element (e.g. [+], menus). Defaults to the [+] button
    /// when `show_add_tab` is set.
    pub bar_suffix: Option<BarSlotFn>,
    /// Kit `TabBar` variant for the strip. Defaults to tab: static
    /// selected styling with no sliding-indicator springs, so the strip
    /// stays cheap on every frame. `Underline`/`Pill`/`Segmented` buy the
    /// animated indicator at the cost of per-frame spring work.
    pub bar_variant: TabVariant,
}

impl Default for SkinOptions {
    fn default() -> Self {
        Self {
            show_add_tab: true,
            show_close: true,
            tab_height: 32.0,
            empty_text: "No open tabs".into(),
            dnd: DndPolicy::reorder_only(),
            bar_prefix: None,
            bar_suffix: None,
            bar_variant: TabVariant::Tab,
        }
    }
}

/// Shared skin state. Groups cache a `CleanSkin` clone at creation, so
/// the action and options live behind `Rc<RefCell>` — late installs (e.g.
/// [+ new tab] wired after the area exists) reach already-mounted strips.
#[derive(Clone)]
pub struct CleanSkin {
    area: Rc<RefCell<WeakEntity<DockArea>>>,
    registry: Rc<TabChromeRegistry>,
    on_add_tab: Rc<RefCell<Option<AddTabAction>>>,
    options: Rc<RefCell<SkinOptions>>,
}

impl CleanSkin {
    pub fn new(registry: Rc<TabChromeRegistry>, on_add_tab: Option<AddTabAction>) -> Self {
        Self::with_options(registry, on_add_tab, SkinOptions::default())
    }

    pub fn with_options(
        registry: Rc<TabChromeRegistry>,
        on_add_tab: Option<AddTabAction>,
        options: SkinOptions,
    ) -> Self {
        Self {
            area: Rc::new(RefCell::new(WeakEntity::new_invalid())),
            registry,
            on_add_tab: Rc::new(RefCell::new(on_add_tab)),
            options: Rc::new(RefCell::new(options)),
        }
    }

    pub fn set_area(&self, area: &Entity<DockArea>) {
        *self.area.borrow_mut() = area.downgrade();
    }

    pub fn options(&self) -> SkinOptions {
        self.options.borrow().clone()
    }

    pub fn set_options(&mut self, options: SkinOptions) {
        *self.options.borrow_mut() = options;
    }

    pub fn set_dnd_policy(&mut self, policy: DndPolicy) {
        self.options.borrow_mut().dnd = policy;
    }

    pub fn set_add_tab(&mut self, action: Option<AddTabAction>) {
        *self.on_add_tab.borrow_mut() = action;
    }

    pub fn set_bar_prefix(&mut self, slot: Option<BarSlotFn>) {
        self.options.borrow_mut().bar_prefix = slot;
    }

    pub fn set_bar_suffix(&mut self, slot: Option<BarSlotFn>) {
        self.options.borrow_mut().bar_suffix = slot;
    }

    pub fn set_bar_variant(&mut self, variant: TabVariant) {
        self.options.borrow_mut().bar_variant = variant;
    }

    fn add_tab(&self, node: NodeId, window: &mut Window, cx: &mut App) {
        if let (Some(area), Some(on_add_tab)) = (
            self.area.borrow().upgrade(),
            self.on_add_tab.borrow().clone(),
        ) {
            on_add_tab(&area, Some(node), window, cx);
        }
    }
}

fn divider_paint(side_by_side: bool, active: bool, cx: &mut App) -> AnyElement {
    let color = if active {
        cx.theme().primary
    } else {
        cx.theme().border
    };
    if side_by_side {
        div().h_full().w(px(1.)).bg(color).into_any_element()
    } else {
        div().w_full().h(px(1.)).bg(color).into_any_element()
    }
}

impl DockAreaRenderer for CleanSkin {
    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(self.clone())
    }

    fn tiles_renderer(&self) -> Rc<dyn TilesRenderer> {
        Rc::new(self.clone())
    }

    fn render_split_handle(
        &self,
        handle: &gpui_kit::base::ResizeHandleContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        // Splits off = locked area = nothing to divide, paint nothing.
        // Flip `dnd.allow_split` on and dividers come back for free.
        if !self.options.borrow().dnd.allow_split {
            return None;
        }
        Some(divider_paint(
            handle.axis().is_horizontal(),
            handle.is_active(),
            cx,
        ))
    }
}

impl TilesRenderer for CleanSkin {
    fn render_drag_bar(
        &self,
        _tile: &TileContext,
        _window: &mut Window,
        _cx: &mut App,
    ) -> AnyElement {
        div().h(px(28.)).w_full().into_any_element()
    }
}

struct TabDragPreview {
    label: SharedString,
}

impl Render for TabDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .text_sm()
            .child(self.label.clone())
    }
}

impl TabGroupRenderer for CleanSkin {
    fn render_tab_bar(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        // Locked singles (fixed side/bottom panels): no strip, no tab DnD.
        // Base renders the panel content directly. Only real tab groups
        // (center, or anything user-merged) get the strip below. The
        // invisible keep-alive panel is exempt: a group holding ONLY it is
        // an emptied center group, which keeps the full strip + empty state.
        let locked_single = group.panels().len() == 1
            && group.panels().first().is_some_and(|p| {
                !p.closable(cx) && p.panel_name(cx) != KEEP_ALIVE_NAME
            });
        if locked_single {
            return Empty.into_any_element();
        }

        let active = group.active_ix();
        // Policy-driven: locked area = no content drops (no splits); the
        // strip wires its own targets so `move_panel` still lands while
        // locked. Flip `dnd` to allow cross-group / splits later.
        // Snapshot options up front: user slot fns run below without any
        // borrow held, so they may safely reconfigure the skin.
        let node = group.node();
        let options = self.options.borrow().clone();
        let dnd = options.dnd.clone();
        let tab_height = options.tab_height;
        let show_close_opt = options.show_close;
        let bar_variant = options.bar_variant;
        let bar_prefix = options.bar_prefix.clone();
        let bar_suffix = options.bar_suffix.clone();
        let show_add_tab = options.show_add_tab;
        let add_tab_set = self.on_add_tab.borrow().is_some();

        // Real tabs only, keeping base indices (the keep-alive panel never
        // shifts them). Filtered BY NAME, not by visible(): the keep-alive
        // is deliberately counted-visible so base's last-panel close/drag
        // rules stay legal. Empty => ghost content below, strip stays.
        let visible: Vec<(usize, &Arc<dyn PanelView>)> = group
            .panels()
            .iter()
            .enumerate()
            .filter(|(_, p)| p.panel_name(cx) != KEEP_ALIVE_NAME)
            .collect();
        let selected_pos = visible.iter().position(|(ix, _)| *ix == active);

        let mut bar = TabBar::new(SharedString::from(format!(
            "dock-tabbar:{}",
            node.as_u64()
        )))
        .with_variant(bar_variant)
        .h(px(tab_height))
        .w_full();
        if let Some(prefix) = bar_prefix.as_ref().and_then(|f| f(window, cx)) {
            bar = bar.prefix(prefix);
        }

        for (ix, panel) in &visible {
            let ix = *ix;
            let chrome = self.registry.chrome(panel, window, cx);
            let preview_label = chrome.label.clone();
            let selected = ix == active;
            // Base hides close on a dock's last group so it can't empty —
            // but an invisible keep-alive panel (total > visible) guarantees
            // survival, so the last REAL tab stays closable.
            let show_close = show_close_opt
                && panel.closable(cx)
                && (group.is_closable() || group.panels().len() > visible.len());

            let close: Option<AnyElement> = if show_close {
                let skin = self.clone();
                let panel = (*panel).clone();
                Some(
                    Button::new(("dock-close", ix))
                        .ghost()
                        .xsmall()
                        .tooltip("Close Tab")
                        .icon(IconName::Close)
                        .on_click(move |_, window, cx| {
                            // Typed removal through the registry: the area
                            // stays locked (no splits) and the group's own
                            // close path refuses locked groups, so the skin
                            // removes through the public DockArea API.
                            if skin.registry.can_close(&panel, cx) {
                                if let Some(area) = skin.area.borrow().upgrade() {
                                    skin.registry.close_panel(&panel, &area, window, cx);
                                }
                            }
                        })
                        .into_any_element(),
                )
            } else {
                None
            };

            let drag = if dnd.enabled {
                group.drag_panel(ix, cx)
            } else {
                None
            };
            let g_select = group.clone();
            let g_drop = group.clone();
            let accept_tab = dnd.clone();
            let finish_tab = dnd.clone();

            // Given kit `Tab` in the app's own style (outline, 32px,
            // pill prefix, icon close): chrome label/prefix/suffix +
            // close, then the skin applies selection, click, drag and
            // drop on top.
            let TabChrome {
                prefix,
                label,
                suffix,
            } = chrome;
            let mut tab = Tab::new().outline().min_h(px(32.)).px_1().label(label);
            if let Some(p) = prefix {
                tab = tab.prefix(div().mr_1().child(p));
            }
            match (suffix, close) {
                (Some(s), Some(c)) => {
                    tab = tab.suffix(h_flex().gap_1().items_center().child(s).child(c));
                }
                (Some(s), None) => {
                    tab = tab.suffix(s);
                }
                (None, Some(c)) => {
                    tab = tab.suffix(c);
                }
                (None, None) => {}
            }
            let tab = tab
                .selected(selected)
                .on_click(move |_, window, cx| {
                    g_select.select_tab(ix, window, cx);
                })
                // Policy gate: same-strip always, cross-group only
                // when `dnd` allows it. This stays the only drop path
                // while splits are off.
                .drag_over::<DragPanel>(move |t, drag, _, cx| {
                    if accept_tab.accepts(drag.source(), node) {
                        t.border_l_2().border_color(cx.theme().primary)
                    } else {
                        t
                    }
                })
                .on_drop(move |drag: &DragPanel, window, cx| {
                    if finish_tab.accepts(drag.source(), node) {
                        g_drop.drop_panel(drag.clone(), Some(ix), true, window, cx);
                    }
                });
            let tab = match drag {
                Some(drag) => tab.on_drag(drag, move |drag, offset, _, cx| {
                    drag.set_drag_offset(offset);
                    drag.set_preview_size(size(px(180.), px(30.)));
                    cx.new(|_| TabDragPreview {
                        label: preview_label.clone(),
                    })
                }),
                None => tab,
            };
            bar = bar.child(tab);
        }

        // NOTE: no ghost text here. With zero real tabs the keep-alive
        // panel becomes active and ITS content renders the empty message;
        // the strip stays clean (prefix + [+] only).

        // Empty space: drop past the last tab appends per policy.
        let g_end = group.clone();
        let accept_end = dnd.clone();
        let finish_end = dnd.clone();
        bar = bar.last_empty_space(
            div()
                .id("dock-tabs-space")
                .h_full()
                .flex_1()
                .min_w(px(24.))
                .drag_over::<DragPanel>(move |t, drag, _, cx| {
                    if accept_end.accepts(drag.source(), node) {
                        t.bg(cx.theme().border)
                    } else {
                        t
                    }
                })
                .on_drop(move |drag: &DragPanel, window, cx| {
                    if finish_end.accepts(drag.source(), node) {
                        g_end.drop_panel(drag.clone(), None, false, window, cx);
                    }
                }),
        );

        // Trailing slot: host element wins; otherwise the [+] button when
        // the host passes an add-tab action and keeps `show_add_tab`.
        let suffix = bar_suffix.as_ref().and_then(|f| f(window, cx));
        if let Some(suffix) = suffix {
            bar = bar.suffix(suffix);
        } else if show_add_tab && add_tab_set {
            let skin = self.clone();
            bar = bar.suffix(
                Button::new(("dock-add-tab", node.as_u64()))
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .tooltip("Add Tab")
                    .on_click(move |_, window, cx| {
                        skin.add_tab(node, window, cx);
                    }),
            );
        }
        if let Some(pos) = selected_pos {
            bar = bar.selected_index(pos);
        }

        bar.into_any_element()
    }

    fn render_empty(
        &self,
        _group: &TabGroupContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let hint = self.options.borrow().empty_text.clone();
        Some(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(hint)
                .into_any_element(),
        )
    }

    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let dnd = self.options.borrow().dnd.clone();
        if !dnd.enabled {
            return None;
        }
        // Split preview only when splits are allowed; locked areas never
        // produce one, so this is a no-op today and free later.
        if indicator.placement().is_some() && !dnd.allow_split {
            return None;
        }
        let to = indicator.to();
        Some(
            div()
                .absolute()
                .left(to.origin().x)
                .top(to.origin().y)
                .w(to.size().width)
                .h(to.size().height)
                .border_2()
                .border_color(cx.theme().primary)
                .into_any_element(),
        )
    }
}
