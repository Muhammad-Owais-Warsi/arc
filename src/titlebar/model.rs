use gpui_kit::component::command::CommandState;
use gpui_kit::component::*;
use gpui_kit::*;

use crate::env::EnvPanel;
use crate::file_panel::FilePanel;
use crate::fs;

pub enum TitleBarEvent {
    WorkspaceSwitched { name: String, path: String },
    MainWindowClosing,
}

impl EventEmitter<TitleBarEvent> for TitleBarView {}

pub struct TitleBarView {
    workspaces: Vec<(String, String)>,
    selected_workspace: Option<usize>,
    env_panel: Entity<EnvPanel>,
    workspace_palette: Entity<CommandState>,
    env_palette: Entity<CommandState>,
    workspace_palette_open: bool,
    env_palette_open: bool,
}

impl TitleBarView {
    pub fn new(
        workspace_palette: Entity<CommandState>,
        env_palette: Entity<CommandState>,
        env_panel: Entity<EnvPanel>,
    ) -> Self {
        Self {
            workspaces: Vec::new(),
            selected_workspace: None,
            env_panel,
            workspace_palette,
            env_palette,
            workspace_palette_open: false,
            env_palette_open: false,
        }
    }

    pub fn load_workspaces(&mut self, cx: &mut Context<Self>) {
        self.workspaces = FilePanel::list_workspace_dirs()
            .into_iter()
            .map(|(name, path)| (name, path.to_string_lossy().to_string()))
            .collect();
        self.selected_workspace = None;
        cx.notify();
    }

    pub fn switch_workspace_to(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((name, path)) = self.workspaces.get(ix).cloned() else {
            return;
        };
        self.selected_workspace = Some(ix);
        fs::workspace::save(&name, &path);
        self.workspace_palette.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(ix)), window, cx);
        });
        cx.emit(TitleBarEvent::WorkspaceSwitched { name, path });
        cx.notify();
    }

    pub fn add_workspace(
        &mut self,
        name: String,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspaces.iter().any(|(n, _)| n == &name) {
            return;
        }
        let ix = self.workspaces.len();
        self.workspaces.push((name, path));
        self.switch_workspace_to(ix, window, cx);
    }

    pub fn restore_saved_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some((name, path)) = fs::workspace::read() {
            if let Some(ix) = self
                .workspaces
                .iter()
                .position(|(n, p)| *n == name && *p == path)
            {
                self.switch_workspace_to(ix, window, cx);
                return true;
            }
        }
        false
    }

    pub fn workspace_name(&self) -> String {
        self.selected_workspace
            .and_then(|ix| self.workspaces.get(ix))
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "open workspace".to_string())
    }

    pub fn selected_ix(&self) -> Option<usize> {
        self.selected_workspace
    }

    pub fn workspace_list(&self) -> Vec<(String, String)> {
        self.workspaces.clone()
    }

    pub fn env_panel(&self) -> Entity<EnvPanel> {
        self.env_panel.clone()
    }

    pub fn workspace_palette(&self) -> Entity<CommandState> {
        self.workspace_palette.clone()
    }

    pub fn env_palette(&self) -> Entity<CommandState> {
        self.env_palette.clone()
    }

    pub fn workspace_picker_open(&self) -> bool {
        self.workspace_palette_open
    }

    pub fn env_picker_open(&self) -> bool {
        self.env_palette_open
    }

    pub fn set_workspace_picker_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.workspace_palette_open = open;
        cx.notify();
    }

    pub fn set_env_picker_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.env_palette_open = open;
        cx.notify();
    }

    pub fn close_workspace_picker(&mut self, cx: &mut Context<Self>) {
        self.workspace_palette_open = false;
        cx.notify();
    }

    pub fn close_env_picker(&mut self, cx: &mut Context<Self>) {
        self.env_palette_open = false;
        cx.notify();
    }
}
