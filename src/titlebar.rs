use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::actions;
use crate::env_panel::EnvPanel;
use crate::env_playground::Environment;
use crate::fs;
use crate::icons::IconName;
use crate::project_panel::ProjectPanel;

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
        self.workspaces = ProjectPanel::list_workspace_dirs()
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

    fn workspace_name(&self) -> String {
        self.selected_workspace
            .and_then(|ix| self.workspaces.get(ix))
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "open workspace".to_string())
    }
}

impl Render for TitleBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace_name = self.workspace_name();
        let env_panel = self.env_panel.clone();
        let selected_workspace = self.selected_workspace;
        let workspaces = self.workspaces.clone();

        // Palette handles live here — read directly from self.
        let workspace_palette = self.workspace_palette.clone();
        let env_palette = self.env_palette.clone();

        let active_env = fs::env::read_active();
        let this = cx.entity(); // Entity<TitleBarView> — for popover-open mutations

        TitleBar::new()
            .h(px(32.))
            .bg(cx.theme().title_bar)
            .on_close_window({
                let this = this.clone();
                move |_, _, cx| {
                    this.update(cx, |_, cx| {
                        cx.emit(TitleBarEvent::MainWindowClosing);
                    });
                }
            })
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .gap_0p5()
                    .child(
                        Button::new("menu")
                            .icon(IconName::Menu)
                            .ghost()
                            .small()
                            .tooltip("Open Application Menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.min_w(px(270.))
                                    .link(
                                        "About Arc",
                                        "https://github.com/Muhammad-Owais-Warsi/arc",
                                    )
                                    .separator()
                                    .menu("Open Settings", Box::new(actions::OpenSettings))
                                    .menu("Copy Settings", Box::new(actions::CopySettings))
                                    .separator()
                                    .menu("Select Theme...", Box::new(actions::ThemeChange))
                                    .separator()
                                    .menu("Quit Arc", Box::new(actions::QuitArc))
                            }),
                    )
                    .child({
                        let this = this.clone();
                        let palette = workspace_palette.clone();
                        Popover::new("workspace-picker")
                            .open(self.workspace_palette_open)
                            .on_open_change({
                                let this = this.clone();
                                let palette = palette.clone();
                                move |is_open, window, cx| {
                                    if *is_open {
                                        palette.update(cx, |p, cx| p.set_query("", window, cx));
                                    }
                                    this.update(cx, |t, cx| {
                                        t.workspace_palette_open = *is_open;
                                        cx.notify();
                                    });
                                }
                            })
                            .trigger(
                                Button::new("workspace")
                                    .ghost()
                                    .small()
                                    .label(workspace_name)
                                    .tooltip("Switch Workspace"),
                            )
                            .content({
                                let this = this.clone();
                                let palette = palette.clone();
                                move |_, _, _cx| {
                                    let items: Vec<CommandItem> = workspaces
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (name, _))| {
                                            CommandItem::new()
                                                .label(name.clone())
                                                .icon(IconName::BriefcaseBusiness)
                                                .checked(Some(i) == selected_workspace)
                                        })
                                        .collect();
                                    Command::new(&palette)
                                        .bordered(false)
                                        .placeholder("Search or switch workspace")
                                        .w(px(260.))
                                        .items(items)
                                        .separator()
                                        .footer({
                                            let this = this.clone();
                                            let palette = palette.clone();
                                            move |_, _window, _cx| {
                                                Button::new("add-workspace")
                                                    .ghost()
                                                    .label("Add Workspace")
                                                    .icon(IconName::Plus)
                                                    .w_full()
                                                    .justify_start()
                                                    .on_click({
                                                        let this = this.clone();
                                                        let palette = palette.clone();
                                                        move |_, window, cx| {
                                                            let name = palette
                                                                .read(cx)
                                                                .query(cx)
                                                                .to_string();
                                                            let name = name.trim().to_string();
                                                            if name.is_empty() {
                                                                return;
                                                            }
                                                            let path = match fs::workspace::create(
                                                                &name,
                                                            ) {
                                                                Ok(p) => p,
                                                                Err(_) => return,
                                                            };
                                                            this.update(cx, |t, cx| {
                                                                t.add_workspace(
                                                                    name, path, window, cx,
                                                                );
                                                                t.workspace_palette_open = false;
                                                                cx.notify();
                                                            });
                                                        }
                                                    })
                                                    .into_any_element()
                                            }
                                        })
                                        .on_confirm({
                                            let this = this.clone();
                                            move |index, window, cx| {
                                                this.update(cx, |t, cx| {
                                                    t.switch_workspace_to(
                                                        index.row, window, cx,
                                                    );
                                                    t.workspace_palette_open = false;
                                                    cx.notify();
                                                });
                                            }
                                        })
                                        .into_any_element()
                                }
                            })
                    })
                    .when(selected_workspace.is_some(), |builder| {
                        let this = this.clone();
                        let palette = env_palette.clone();
                        let ep = env_panel.clone();
                        builder.child(
                            Popover::new("environment-picker")
                                .open(self.env_palette_open)
                                .on_open_change({
                                    let this = this.clone();
                                    let palette = palette.clone();
                                    move |is_open, window, cx| {
                                        if *is_open {
                                            palette.update(cx, |p, cx| p.set_query("", window, cx));
                                        }
                                        this.update(cx, |t, cx| {
                                            t.env_palette_open = *is_open;
                                            cx.notify();
                                        });
                                    }
                                })
                                .trigger(
                                    Button::new("env-trigger")
                                        .ghost()
                                        .small()
                                        .label(if active_env.is_empty() {
                                            "no env".into()
                                        } else {
                                            active_env.clone()
                                        })
                                        .tooltip("Switch Environment"),
                                )
                                .content(move |_, _, cx| {
                                    let envs = ep.read(cx).env_names(cx);
                                    let active = active_env.clone();
                                    let items: Vec<CommandItem> = envs
                                        .iter()
                                        .map(|name| {
                                            CommandItem::new()
                                                .label(name.clone())
                                                .icon(IconName::Variable)
                                                .checked(name == &active)
                                        })
                                        .collect();
                                    Command::new(&palette)
                                        .bordered(false)
                                        .placeholder("Search or switch environment")
                                        .w(px(260.))
                                        .items(items)
                                        .separator()
                                        .footer({
                                            let this = this.clone();
                                            let palette = palette.clone();
                                            let ep = ep.clone();
                                            move |_, _, _| {
                                                Button::new("add-env")
                                                    .ghost()
                                                    .label("Add Environment")
                                                    .icon(IconName::Plus)
                                                    .w_full()
                                                    .justify_start()
                                                    .on_click({
                                                        let this = this.clone();
                                                        let palette = palette.clone();
                                                        let ep = ep.clone();
                                                        move |_, _window, cx| {
                                                            let name = palette
                                                                .read(cx)
                                                                .query(cx)
                                                                .to_string();
                                                            let name = name.trim().to_string();
                                                            if name.is_empty() {
                                                                return;
                                                            }
                                                            let mut envs: Vec<Environment> =
                                                                serde_json::from_str(
                                                                    &fs::env::read_environments(),
                                                                )
                                                                .unwrap_or_default();
                                                            if envs.iter().any(|e| e.name == name) {
                                                                return;
                                                            }
                                                            envs.push(Environment {
                                                                name: name.clone(),
                                                                variables: Vec::new(),
                                                            });
                                                            let content =
                                                                serde_json::to_string_pretty(&envs)
                                                                    .unwrap_or_default();
                                                            let _ = fs::env::write_environments(
                                                                &content,
                                                            );
                                                            ep.update(cx, |panel, cx| {
                                                                panel.refresh(cx)
                                                            });
                                                            this.update(cx, |_, cx| cx.notify());
                                                        }
                                                    })
                                                    .into_any_element()
                                            }
                                        })
                                        .on_confirm({
                                            let this = this.clone();
                                            let ep = ep.clone();
                                            move |index, _window, cx| {
                                                let envs = ep.read(cx).env_names(cx);
                                                if let Some(name) = envs.get(index.row) {
                                                    let name = name.clone();
                                                    fs::env::save_active(&name);
                                                    ep.update(cx, |panel, cx| panel.refresh(cx));
                                                    this.update(cx, |t, cx| {
                                                        t.env_palette_open = false;
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        })
                                        .into_any_element()
                                }),
                        )
                    }),
            )
            .into_any_element()
    }
}
