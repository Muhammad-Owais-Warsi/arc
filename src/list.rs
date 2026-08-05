use gpui::{AnyElement, Component, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    list::{ListDelegate, ListItem, ListSeparatorItem, ListState},
};

use crate::fs;
use crate::project_panel::ProjectPanel;

pub enum WorkspaceListItemEvent {
    WorkspaceChanged(String),
}

impl EventEmitter<WorkspaceListItemEvent> for ListState<WorkspaceListItem> {}

pub struct WorkspaceListItem {
    project_panel: Entity<ProjectPanel>,
    active_workspace_index: Option<IndexPath>,
    query: String,
    new_workspace: Option<(usize, Entity<InputState>)>,
}

pub enum WorkspaceListRow {
    Workspace(ListItem),
    Separator(ListSeparatorItem),
    CreateWorkspace(ListItem),
}

impl Selectable for WorkspaceListRow {
    fn selected(self, selected: bool) -> Self {
        match self {
            Self::Workspace(item) => Self::Workspace(item.selected(selected)),
            Self::Separator(item) => Self::Separator(item.selected(selected)),
            Self::CreateWorkspace(item) => Self::CreateWorkspace(item.selected(selected)),
        }
    }

    fn is_selected(&self) -> bool {
        match self {
            Self::Workspace(item) => item.is_selected(),
            Self::Separator(item) => item.is_selected(),
            Self::CreateWorkspace(item) => item.is_selected(),
        }
    }

    fn secondary_selected(self, selected: bool) -> Self {
        match self {
            Self::Workspace(item) => Self::Workspace(item.secondary_selected(selected)),
            Self::Separator(item) => Self::Separator(item.secondary_selected(selected)),
            Self::CreateWorkspace(item) => Self::CreateWorkspace(item.secondary_selected(selected)),
        }
    }
}

impl IntoElement for WorkspaceListRow {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        match self {
            Self::Workspace(item) => item.into_element().into_any(),
            Self::Separator(item) => Component::new(item).into_element().into_any(),
            Self::CreateWorkspace(item) => item.into_element().into_any(),
        }
    }
}

impl WorkspaceListItem {
    pub fn new(project_panel: Entity<ProjectPanel>) -> Self {
        Self {
            project_panel,
            active_workspace_index: None,
            query: String::new(),
            new_workspace: None,
        }
    }

    fn filtered_names(&self, cx: &App) -> Vec<String> {
        let query = self.query.to_lowercase();
        self.project_panel
            .read(cx)
            .workspace_names()
            .into_iter()
            .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
            .collect()
    }

    fn start_create_workspace(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("workspace-name"));
        let input_for_subscription = input.clone();

        cx.subscribe_in(&input, window, {
            move |this: &mut ListState<WorkspaceListItem>, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => {
                    let name = input_for_subscription.read(cx).value().trim().to_string();
                    this.delegate_mut().finish_create_workspace(&name, cx);
                }
                InputEvent::Blur => {
                    this.delegate_mut().cancel_create_workspace(cx);
                }
                _ => {}
            }
        })
        .detach();

        input.update(cx, |i, cx| i.focus(window, cx));
        self.new_workspace = Some((self.filtered_names(cx).len() + 1, input));
        cx.notify();
    }

    fn cancel_create_workspace(&mut self, cx: &mut Context<ListState<Self>>) {
        self.new_workspace = None;
        cx.notify();
    }

    fn finish_create_workspace(&mut self, name: &str, cx: &mut Context<ListState<Self>>) {
        self.cancel_create_workspace(cx);

        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }

        match fs::create_workspace(&name) {
            Ok(path) => {
                self.project_panel.update(cx, |pp, cx| {
                    pp.add_workspace(name.clone(), path, cx);
                });

                cx.emit(WorkspaceListItemEvent::WorkspaceChanged(name));
            }
            Err(err) => eprintln!("Failed to create workspace: {err}"),
        }
    }
}

impl ListDelegate for WorkspaceListItem {
    type Item = WorkspaceListRow;

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.active_workspace_index = None;
        cx.notify();
        Task::ready(())
    }

    fn items_count(&self, _section: usize, cx: &App) -> usize {
        self.filtered_names(cx).len() + 2
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let names = self.filtered_names(cx);
        let separator_row = names.len();
        let create_row = names.len() + 1;

        if ix.row < names.len() {
            let active = self
                .project_panel
                .read(cx)
                .get_selected_workspace()
                .to_string();
            names.get(ix.row).map(|name| {
                WorkspaceListRow::Workspace(
                    ListItem::new(ix)
                        .child(Label::new(name.clone()))
                        // .selected(*name == active)
                        .check_icon(IconName::Check)
                        .confirmed(*name == active),
                )
            })
        } else if ix.row == separator_row {
            Some(WorkspaceListRow::Separator(
                ListSeparatorItem::new().child(div().h_px().w_full().bg(cx.theme().border)),
            ))
        } else if let Some((row, input)) = &self.new_workspace {
            if ix.row == *row {
                Some(WorkspaceListRow::CreateWorkspace(
                    ListItem::new(ix).child(Input::new(input).appearance(false)),
                ))
            } else if ix.row == create_row {
                Some(WorkspaceListRow::CreateWorkspace(
                    ListItem::new(ix).child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Plus))
                            .child(Label::new("Create workspace")),
                    ),
                ))
            } else {
                None
            }
        } else if ix.row == create_row {
            Some(WorkspaceListRow::CreateWorkspace(
                ListItem::new(ix).child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(Icon::new(IconName::Plus))
                        .child(Label::new("Create workspace")),
                ),
            ))
        } else {
            None
        }
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.active_workspace_index = ix;
        cx.notify();
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        if let Some(ix) = self.active_workspace_index {
            let names = self.filtered_names(cx);
            if ix.row < names.len() {
                if let Some(name) = names.get(ix.row).cloned() {
                    cx.emit(WorkspaceListItemEvent::WorkspaceChanged(name));
                }
            } else if ix.row == names.len() + 1 {
                self.start_create_workspace(window, cx);
            }
        }
    }
}
