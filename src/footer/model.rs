use gpui_kit::*;

use crate::settings::SidebarDock;

pub struct Footer {
    show_toggle: bool,
    file_panel_collapsed: bool,
    env_panel_collapsed: bool,
    response_collapsed: bool,
    file_panel_dock: SidebarDock,
    env_panel_dock: SidebarDock,
}

#[derive(Clone, Debug)]
pub enum FooterEvent {
    ToggleResponse,
    ToggleFilePanel,
    ToggleEnvPanel,
}

impl EventEmitter<FooterEvent> for Footer {}

impl Footer {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            show_toggle: false,
            file_panel_collapsed: true,
            env_panel_collapsed: true,
            response_collapsed: true,
            file_panel_dock: SidebarDock::Left,
            env_panel_dock: SidebarDock::Right,
        }
    }

    pub fn set_show_toggle(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.show_toggle == show {
            return;
        }
        self.show_toggle = show;
        cx.notify();
    }

    pub fn set_file_panel_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.file_panel_collapsed == collapsed {
            return;
        }
        self.file_panel_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_env_panel_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.env_panel_collapsed == collapsed {
            return;
        }
        self.env_panel_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_response_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.response_collapsed == collapsed {
            return;
        }
        self.response_collapsed = collapsed;
        cx.notify();
    }

    pub fn set_file_panel_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        if self.file_panel_dock == dock {
            return;
        }
        self.file_panel_dock = dock;
        cx.notify();
    }

    pub fn set_env_panel_dock(&mut self, dock: SidebarDock, cx: &mut Context<Self>) {
        if self.env_panel_dock == dock {
            return;
        }
        self.env_panel_dock = dock;
        cx.notify();
    }

    pub fn show_response_toggle(&self) -> bool {
        self.show_toggle
    }

    pub fn file_panel_open(&self) -> bool {
        !self.file_panel_collapsed
    }

    pub fn env_panel_open(&self) -> bool {
        !self.env_panel_collapsed
    }

    pub fn response_open(&self) -> bool {
        !self.response_collapsed
    }

    pub fn file_panel_dock(&self) -> SidebarDock {
        self.file_panel_dock
    }

    pub fn env_panel_dock(&self) -> SidebarDock {
        self.env_panel_dock
    }
}
