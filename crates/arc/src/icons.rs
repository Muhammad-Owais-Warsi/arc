// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use ui::{Icon, IconNamed};
use strum::AsRefStr;

#[derive(AsRefStr, IntoElement)]
#[strum(serialize_all = "kebab_case")]
pub enum IconName {
    ArrowDown,
    ArrowUp,
    Check,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    Close,
    Folder,
    FolderOpen,
    PanelBottom,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
    Send,
    Spinner,
    Trash,
    File,
    Rename,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
    Copy,
}

impl IconNamed for IconName {
    fn path(self) -> SharedString {
        format!("icons/{}.svg", self.as_ref()).into()
    }
}

impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Icon::empty().path(self.path())
    }
}
