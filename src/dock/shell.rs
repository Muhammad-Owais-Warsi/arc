use std::rc::Rc;

use gpui_kit::base::dock::{DockArea, DockLayout, DockPlacement, InsertTarget, Panel, PanelId};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::resizable::{
    ResizablePanel, ResizablePanelEvent, ResizableState, h_resizable, resizable_panel,
    v_resizable,
};
use gpui_kit::component::{Sizable, h_flex};
use gpui_kit::*;
use crate::dock::skin::{AddTabAction, BarSlotFn, CleanSkin, SkinOptions};

use crate::dock::tabs::TabChromeRegistry;

// ---------------------------------------------------------------------------
// DockShell: drop-in dock layout for the app.
//
// Shape: fixed panels live OUTSIDE the dock area as plain resizable side
// slots, so tab DnD physically cannot enter or leave them. Only the center
// hosts dock tab groups (reorder-only DnD, custom tab bar).
//
// ```text
// h_resizable("{prefix}-shell"): [left] [center: DockArea + bottom aux] [right]
// ```
//
// Direction: every fixed panel has a side (Left/Right) and each slot
// shows exactly one panel — the side's active view, no header, no
// switcher, no leftover icons. Single-open radio per side: showing one
// panel closes its side-mates; closing the shown one falls back to
// nothing and the slot collapses entirely. The pickers are the footer
// toggles ([`dock_toggles`], placeable anywhere): a toggle hides its
// panel when shown, otherwise shows it. Moving a panel across sides
// carries its toggle along — the toggles always group by current side.
// Plug/remove is one call: `plug_panel` / `set_panel` in,
// `unplug_panel` / `remove_panel` / `clear_side` / `clear_all_panels`
// out; aux the same via `set_aux_named` / `clear_aux`.
//
// The host (demo binary here, real gpui client later) does:
//   1. `let shell = cx.new(|cx| DockShell::new(window, cx, registry, on_add));`
//   2. `shell.update(cx, |s, cx| { s.set_center(..); s.set_panel(..); .. })`
//   3. render `shell` wherever the old sidebar flexbox was.
//
// Panels stay host-owned entities; the shell only holds `AnyView`s. To move
// this into the gpui project: copy `skin.rs` + `shell.rs` + `tabs.rs`,
// implement `CenterTab` on the center tab views, register them, done.
// ---------------------------------------------------------------------------

/// Which side slot a fixed panel lives in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Side::Left => "L",
            Side::Right => "R",
        }
    }
}

/// A fixed (non-tabbed) panel owned by a side slot. `id` is owned so
/// hosts can plug any number of dynamic sidebars in and out at runtime.
pub struct FixedPanel {
    id: SharedString,
    label: SharedString,
    view: AnyView,
    open: bool,
}

impl FixedPanel {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        view: AnyView,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            view,
            open: true,
        }
    }

    pub fn id(&self) -> &SharedString {
        &self.id
    }
}

/// Snapshot of one fixed panel for toggles widgets and host chrome.
#[derive(Clone)]
pub struct FixedPanelInfo {
    pub id: SharedString,
    pub label: SharedString,
    pub open: bool,
    pub active: bool,
}

/// A single bottom aux panel (response, terminal, problems...). Bottom-only
/// on purpose: one place, one drag state, no place-switching machinery.
struct AuxSlot {
    id: SharedString,
    label: SharedString,
    view: AnyView,
    visible: bool,
}

/// Sizes + resizable keys. Defaults match the demo; hosts embedding more
/// than one shell (or their own widths) pass their own.
#[derive(Clone)]
pub struct ShellOptions {
    pub left_width: f32,
    pub right_width: f32,
    pub aux_bottom_height: f32,
    /// Prefix for resizable keys + element ids (multi-window safe).
    pub group_prefix: SharedString,
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            left_width: 260.0,
            right_width: 240.0,
            aux_bottom_height: 280.0,
            group_prefix: "dock".into(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum DockShellEvent {
    Changed,
}

impl EventEmitter<DockShellEvent> for DockShell {}

pub struct DockShell {
    area: Entity<DockArea>,
    _skin: CleanSkin,
    shell_options: ShellOptions,
    /// Owned layout state (like the dock does): drag sizes live here, not
    /// in window-keyed element state, so they survive toggles and unmounts.
    shell_state: Entity<ResizableState>,
    center_col_state: Entity<ResizableState>,
    left: Vec<FixedPanel>,
    right: Vec<FixedPanel>,
    active_left: Option<SharedString>,
    active_right: Option<SharedString>,
    aux: Option<AuxSlot>,
}

impl DockShell {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        registry: Rc<TabChromeRegistry>,
        on_add_tab: Option<AddTabAction>,
    ) -> Self {
        Self::with_options(window, cx, registry, on_add_tab, SkinOptions::default())
    }

    pub fn with_options(
        window: &mut Window,
        cx: &mut Context<Self>,
        registry: Rc<TabChromeRegistry>,
        on_add_tab: Option<AddTabAction>,
        options: SkinOptions,
    ) -> Self {
        Self::with_shell_options(
            window,
            cx,
            registry,
            on_add_tab,
            options,
            ShellOptions::default(),
        )
    }

    pub fn with_shell_options(
        window: &mut Window,
        cx: &mut Context<Self>,
        registry: Rc<TabChromeRegistry>,
        on_add_tab: Option<AddTabAction>,
        options: SkinOptions,
        shell_options: ShellOptions,
    ) -> Self {
        let allow_split = options.dnd.allow_split;
        let skin = CleanSkin::with_options(registry, on_add_tab, options);
        let area = cx.new(|cx| {
            DockArea::new("arc-dock", Some(1), window, cx).with_renderer(Rc::new(skin.clone()))
        });
        // Splits off = locked area = no content drops; the strip still
        // wires its own reorder targets. Flip `dnd.allow_split` on and
        // splits come back with no other change.
        area.update(cx, |area, cx| {
            area.set_locked(!allow_split, window, cx);
        });
        skin.set_area(&area);
        let shell_state = cx.new(|_| ResizableState::default());
        let center_col_state = cx.new(|_| ResizableState::default());
        // Settle immediately when a drag lands instead of waiting for the
        // next input to repaint the window.
        for state in [&shell_state, &center_col_state] {
            cx.subscribe(state, |_, _, _: &ResizablePanelEvent, cx| {
                cx.notify();
            })
            .detach();
        }
        Self {
            area,
            _skin: skin,
            shell_options,
            shell_state,
            center_col_state,
            left: Vec::new(),
            right: Vec::new(),
            active_left: None,
            active_right: None,
            aux: None,
        }
    }

    pub fn shell_options(&self) -> &ShellOptions {
        &self.shell_options
    }

    pub fn set_shell_options(&mut self, options: ShellOptions, cx: &mut Context<Self>) {
        self.shell_options = options;
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Install (or clear) the bar prefix slot (e.g. back/forward) after
    /// construction.
    pub fn set_bar_prefix(&mut self, slot: Option<BarSlotFn>, cx: &mut Context<Self>) {
        self._skin.set_bar_prefix(slot);
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Install (or clear) the [+ new tab] action after construction.
    pub fn set_add_tab(
        &mut self,
        action: Option<crate::dock::skin::AddTabAction>,
        cx: &mut Context<Self>,
    ) {
        self._skin.set_add_tab(action);
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    pub fn set_dnd_policy(
        &mut self,
        policy: crate::dock::skin::DndPolicy,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let allow_split = policy.allow_split;
        self._skin.set_dnd_policy(policy);
        self.area.update(cx, |area, cx| {
            area.set_locked(!allow_split, window, cx);
        });
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    pub fn area(&self) -> Entity<DockArea> {
        self.area.clone()
    }

    pub fn set_center(
        &mut self,
        layout: DockLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.area.update(cx, |area, cx| {
            area.set_center(layout, window, cx);
        });
        cx.emit(DockShellEvent::Changed);
    }

    pub fn add_panel<P: Panel>(
        &mut self,
        panel: Entity<P>,
        placement: DockPlacement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.area.update(cx, |area, cx| {
            area.add_panel(panel, placement, None, window, cx);
        });
    }

    pub fn move_panel(
        &mut self,
        panel: PanelId,
        target: InsertTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Policy gate: splits refused while `dnd.allow_split` is off.
        if !self._skin.options().dnd.allow_split
            && matches!(target, InsertTarget::Split { .. })
        {
            return;
        }
        self.area.update(cx, |area, cx| {
            area.move_panel(panel, target, window, cx);
        });
    }

    fn slot(&self, side: Side) -> &Vec<FixedPanel> {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    fn slot_mut(&mut self, side: Side) -> &mut Vec<FixedPanel> {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    fn take_panel(&mut self, id: &str) -> Option<FixedPanel> {
        if let Some(ix) = self.left.iter().position(|p| p.id.as_ref() == id) {
            return Some(self.left.remove(ix));
        }
        if let Some(ix) = self.right.iter().position(|p| p.id.as_ref() == id) {
            return Some(self.right.remove(ix));
        }
        None
    }

    fn side_of(&self, id: &str) -> Option<Side> {
        if self.left.iter().any(|p| p.id.as_ref() == id) {
            Some(Side::Left)
        } else if self.right.iter().any(|p| p.id.as_ref() == id) {
            Some(Side::Right)
        } else {
            None
        }
    }

    /// Single-open radio per side: show `id`, closing every other panel on
    /// that side. No events — callers emit after.
    fn show_only(&mut self, side: Side, id: &SharedString) {
        let (slot, active) = match side {
            Side::Left => (&mut self.left, &mut self.active_left),
            Side::Right => (&mut self.right, &mut self.active_right),
        };
        for p in slot.iter_mut() {
            p.open = &p.id == id;
        }
        *active = Some(id.clone());
    }

    /// Insert or replace a fixed panel on a side. One call to plug a
    /// sidebar in; [`Self::remove_panel`]/[`Self::unplug_panel`] takes it
    /// back out. Ids are owned, so any number of dynamic panels works.
    /// An open panel takes over its side (side-mates close); a closed one
    /// just parks there until its toggle is used.
    pub fn set_panel(&mut self, side: Side, panel: FixedPanel, cx: &mut Context<Self>) {
        let open = panel.open;
        let pid = panel.id.clone();
        let slot = self.slot_mut(side);
        if let Some(ix) = slot.iter().position(|p| p.id == pid) {
            slot[ix] = panel;
        } else {
            slot.push(panel);
        }
        if open {
            self.show_only(side, &pid);
        }
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Alias that reads as plugin: `shell.plug(Side::Left, panel, cx)`.
    pub fn plug_panel(&mut self, side: Side, panel: FixedPanel, cx: &mut Context<Self>) {
        self.set_panel(side, panel, cx);
    }

    pub fn remove_panel(&mut self, id: &str, cx: &mut Context<Self>) {
        self.take_panel(id);
        if self.active_left.as_deref() == Some(id) {
            self.active_left = None;
        }
        if self.active_right.as_deref() == Some(id) {
            self.active_right = None;
        }
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Alias that reads as removal: `shell.unplug_panel("explorer", cx)`.
    pub fn unplug_panel(&mut self, id: &str, cx: &mut Context<Self>) {
        self.remove_panel(id, cx);
    }

    pub fn has_panel(&self, id: &str) -> bool {
        self.left.iter().any(|p| p.id.as_ref() == id)
            || self.right.iter().any(|p| p.id.as_ref() == id)
    }

    pub fn is_panel_open(&self, id: &str) -> bool {
        self.left
            .iter()
            .chain(self.right.iter())
            .any(|p| p.id.as_ref() == id && p.open)
    }

    pub fn set_panel_open(&mut self, id: &str, open: bool, cx: &mut Context<Self>) {
        if open {
            let side = self.side_of(id);
            let pid = self
                .left
                .iter()
                .chain(self.right.iter())
                .find(|p| p.id.as_ref() == id)
                .map(|p| p.id.clone());
            match (side, pid) {
                (Some(side), Some(pid)) => {
                    self.show_only(side, &pid);
                    cx.emit(DockShellEvent::Changed);
                    cx.notify();
                }
                _ => {}
            }
            return;
        }
        for slot in [&mut self.left, &mut self.right] {
            if let Some(p) = slot.iter_mut().find(|p| p.id.as_ref() == id) {
                if p.open {
                    p.open = false;
                    cx.emit(DockShellEvent::Changed);
                    cx.notify();
                }
                return;
            }
        }
    }

    pub fn clear_side(&mut self, side: Side, cx: &mut Context<Self>) {
        self.slot_mut(side).clear();
        match side {
            Side::Left => self.active_left = None,
            Side::Right => self.active_right = None,
        }
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    pub fn clear_all_panels(&mut self, cx: &mut Context<Self>) {
        self.left.clear();
        self.right.clear();
        self.active_left = None;
        self.active_right = None;
        self.aux = None;
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Move a fixed panel to the other side (or a given side). An open
    /// panel takes over its new side (mates there close); a closed one
    /// just parks there. Sizes stay with the placement.
    pub fn set_panel_side(&mut self, id: &str, side: Side, cx: &mut Context<Self>) {
        let Some(panel) = self.take_panel(id) else {
            return;
        };
        let open = panel.open;
        let pid = panel.id.clone();
        let slot = self.slot_mut(side);
        if slot.iter().all(|p| p.id != pid) {
            slot.push(panel);
        }
        if open {
            self.show_only(side, &pid);
        }
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Footer-toggle behavior: hide the panel when it is the side's shown
    /// one, otherwise show it (closing its side-mates). Closing the last
    /// open panel collapses the slot entirely — nothing lingers.
    pub fn toggle_panel(&mut self, id: &str, cx: &mut Context<Self>) {
        let side = self.side_of(id);
        let shown = side
            .and_then(|s| self.active_panel_id(s))
            .as_deref()
            == Some(id)
            && self.is_panel_open(id);
        match (side, shown) {
            (Some(_), true) => self.set_panel_open(id, false, cx),
            (Some(side), false) => {
                if let Some(pid) = self
                    .slot(side)
                    .iter()
                    .find(|p| p.id.as_ref() == id)
                    .map(|p| p.id.clone())
                {
                    self.show_only(side, &pid);
                    cx.emit(DockShellEvent::Changed);
                    cx.notify();
                }
            }
            (None, _) => {}
        }
    }

    /// Show a fixed panel: opens it, closes its side-mates, makes it the
    /// side's view. One open panel per side, always.
    pub fn activate_panel(&mut self, id: &str, cx: &mut Context<Self>) {
        let side = self.side_of(id);
        let pid = self
            .left
            .iter()
            .chain(self.right.iter())
            .find(|p| p.id.as_ref() == id)
            .map(|p| p.id.clone());
        match (side, pid) {
            (Some(side), Some(pid)) => {
                self.show_only(side, &pid);
                cx.emit(DockShellEvent::Changed);
                cx.notify();
            }
            _ => {}
        }
    }

    /// Id of the panel currently shown on a side, if any.
    pub fn active_panel_id(&self, side: Side) -> Option<SharedString> {
        match side {
            Side::Left => self.active_left.clone(),
            Side::Right => self.active_right.clone(),
        }
    }

    pub fn swap_sides(&mut self, cx: &mut Context<Self>) {
        std::mem::swap(&mut self.left, &mut self.right);
        std::mem::swap(&mut self.active_left, &mut self.active_right);
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    /// Which panel id is displayed: the stored one if still open, else
    /// the first open panel. ID-based (never index-based) so toggling or
    /// moving panels can never strand the view on the wrong entry.
    fn resolve_active_id(
        slot: &[FixedPanel],
        stored: Option<&SharedString>,
    ) -> Option<SharedString> {
        if let Some(id) = stored {
            if slot.iter().any(|p| &p.id == id && p.open) {
                return Some(id.clone());
            }
        }
        slot.iter().find(|p| p.open).map(|p| p.id.clone())
    }

    pub fn side_panels(&self, side: Side) -> Vec<FixedPanelInfo> {
        let slot = self.slot(side);
        let stored = match side {
            Side::Left => self.active_left.as_ref(),
            Side::Right => self.active_right.as_ref(),
        };
        let active = Self::resolve_active_id(slot, stored);
        slot
            .iter()
            .map(|p| FixedPanelInfo {
                id: p.id.clone(),
                label: p.label.clone(),
                open: p.open,
                active: Some(p.id.clone()) == active,
            })
            .collect()
    }

    /// Plug an aux panel under an explicit id + label. One call to add a
    /// bottom/side tool (response, terminal, problems...); `remove_aux`
    /// / `clear_aux` takes it out.
    pub fn set_aux_named(
        &mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        view: Option<AnyView>,
        cx: &mut Context<Self>,
    ) {
        let id = id.into();
        let label = label.into();
        self.aux = view.map(|view| AuxSlot {
            id,
            label,
            view,
            visible: true,
        });
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    pub fn set_aux(&mut self, view: Option<AnyView>, cx: &mut Context<Self>) {
        self.set_aux_named("aux", "Aux", view, cx);
    }

    pub fn remove_aux(&mut self, cx: &mut Context<Self>) {
        self.clear_aux(cx);
    }

    pub fn clear_aux(&mut self, cx: &mut Context<Self>) {
        self.aux = None;
        cx.emit(DockShellEvent::Changed);
        cx.notify();
    }

    pub fn aux_info(&self) -> Option<(SharedString, SharedString, bool)> {
        self.aux
            .as_ref()
            .map(|a| (a.id.clone(), a.label.clone(), a.visible))
    }

    pub fn toggle_aux(&mut self, cx: &mut Context<Self>) {
        if let Some(aux) = self.aux.as_mut() {
            aux.visible = !aux.visible;
            cx.emit(DockShellEvent::Changed);
            cx.notify();
        }
    }

    pub fn aux_visible(&self) -> bool {
        self.aux.as_ref().is_some_and(|a| a.visible)
    }

    /// Slot: exactly one panel mounted — the resolved active view, no
    /// header, no switcher, no leftover icons. With 2+ panels on one side
    /// the single-open radio decides: whichever panel the toggles last
    /// showed fills the slot, the rest stay closed.
    fn render_slot(&self, side: Side, width: Pixels, _cx: &mut App) -> ResizablePanel {
        let slot = self.slot(side);
        let stored = match side {
            Side::Left => self.active_left.as_ref(),
            Side::Right => self.active_right.as_ref(),
        };
        let active_id = Self::resolve_active_id(slot, stored);
        let mut panel = resizable_panel()
            .size(width)
            .flex_none()
            .visible(active_id.is_some());
        if let Some(active_view) = active_id
            .and_then(|id| slot.iter().find(|p| p.id == id))
            .or_else(|| slot.iter().find(|p| p.open))
            .map(|p| p.view.clone().into_any_element())
        {
            panel = panel.child(active_view);
        }
        panel
    }

    fn render_center(&self) -> AnyElement {
        let area = self.area.clone().into_any_element();
        match &self.aux {
            Some(aux) if aux.visible => {
                let view = aux.view.clone().into_any_element();
                let o = &self.shell_options;
                let col = SharedString::from(format!("{}-center-col", o.group_prefix));
                v_resizable(col)
                    .with_state(&self.center_col_state)
                    .child(resizable_panel().child(area))
                    .child(
                        resizable_panel()
                            .size(px(o.aux_bottom_height))
                            .flex_none()
                            .child(view),
                    )
                    .into_any_element()
            }
            _ => area,
        }
    }
}

impl Render for DockShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let o = self.shell_options.clone();
        let key = SharedString::from(format!("{}-shell", o.group_prefix));
        h_resizable(key)
            .with_state(&self.shell_state)
            .child(self.render_slot(Side::Left, px(o.left_width), cx))
            .child(resizable_panel().child(self.render_center()))
            .child(self.render_slot(Side::Right, px(o.right_width), cx))
    }
}

// ---------------------------------------------------------------------------
// Reusable toggle set. Place it anywhere (status bar, title bar, command
// palette — the buttons live wherever the host puts this element):
// per-panel show/hide toggles grouped by side + per-panel side switches +
// side swap + aux toggle, all wired to the shell. Hosts needing different
// controls call the same public methods directly.
// ---------------------------------------------------------------------------

pub fn dock_toggles(shell: &Entity<DockShell>, cx: &mut App) -> AnyElement {
    let mut row = h_flex().gap_1();
    for side in [Side::Left, Side::Right] {
        for info in shell.read(cx).side_panels(side) {
            let toggle_shell = shell.clone();
            let id = info.id.clone();
            row = row.child(
                Button::new(SharedString::from(format!("toggle-panel:{id}")))
                    .ghost()
                    .xsmall()
                    .label(info.label.clone())
                    .toggled(info.open)
                    .tooltip("Toggle panel")
                    .on_click(move |_, _, cx| {
                        let id = id.clone();
                        toggle_shell.update(cx, |s, cx| s.toggle_panel(&id, cx));
                    }),
            );
            let side_shell = shell.clone();
            let move_id = info.id.clone();
            row = row.child(
                Button::new(SharedString::from(format!("panel-side:{move_id}")))
                    .ghost()
                    .xsmall()
                    .label(side.short())
                    .tooltip("Move to other side")
                    .on_click(move |_, _, cx| {
                        let move_id = move_id.clone();
                        side_shell.update(cx, |s, cx| s.set_panel_side(&move_id, side.other(), cx));
                    }),
            );
        }
    }

    let swap_shell = shell.clone();
    let aux_shell = shell.clone();
    let aux = shell.read(cx).aux_info();
    row = row.child(
        Button::new("swap-sides")
            .ghost()
            .xsmall()
            .label("Swap")
            .tooltip("Swap left/right panels")
            .on_click(move |_, _, cx| {
                swap_shell.update(cx, |s, cx| s.swap_sides(cx));
            }),
    );
    // Generic aux toggle: follows whatever aux is plugged in (response,
    // terminal, ...). Hidden when no aux panel is mounted.
    if let Some((aux_id, aux_label, aux_visible)) = aux {
        row = row.child(
            Button::new(SharedString::from(format!("toggle-aux:{aux_id}")))
                .ghost()
                .xsmall()
                .label(aux_label)
                .toggled(aux_visible)
                .tooltip("Toggle aux panel")
                .on_click(move |_, _, cx| {
                    aux_shell.update(cx, |s, cx| s.toggle_aux(cx));
                }),
        );
    }
    row.into_any_element()
}
