use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::ApiClient;
use crate::actions;
use crate::env_playground::Environment;
use crate::fs;
use crate::icons::IconName;

pub struct TitleBarView {
    client: WeakEntity<ApiClient>,
    // Palettes owned here: only the titlebar renders and resets them.
    workspace_palette: Entity<CommandState>,
    env_palette: Entity<CommandState>,
    workspace_palette_open: bool,
    env_palette_open: bool,
}

impl TitleBarView {
    pub fn new(workspace_palette: Entity<CommandState>, env_palette: Entity<CommandState>) -> Self {
        Self {
            client: WeakEntity::new_invalid(),
            workspace_palette,
            env_palette,
            workspace_palette_open: false,
            env_palette_open: false,
        }
    }

    /// Wire in the ApiClient handle after the entity has been created.
    pub fn set_client(&mut self, client: WeakEntity<ApiClient>) {
        self.client = client;
    }

    /// Called by ApiClient when a workspace is switched so the palette
    /// reflects the newly selected item.
    pub fn sync_workspace_selection(
        &mut self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace_palette.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(ix)), window, cx);
        });
    }
}

impl Render for TitleBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(client_entity) = self.client.upgrade() else {
            return div().into_any_element();
        };

        // Read ApiClient state that only it owns.
        let (settings_window, workspace_name, env_panel, selected_workspace, workspaces) = {
            let c = client_entity.read(cx);
            let workspace_name = c
                .selected_workspace
                .and_then(|ix| c.workspaces.get(ix))
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "open workspace".to_string());
            (
                c.settings_window.clone(),
                workspace_name,
                c.env_panel.clone(),
                c.selected_workspace,
                c.workspaces.clone(),
            )
        };

        // Palette handles live here — read directly from self.
        let workspace_palette = self.workspace_palette.clone();
        let env_palette = self.env_palette.clone();

        let active_env = fs::env::read_active();
        let this = cx.entity(); // Entity<TitleBarView> — for popover-open mutations
        let client = self.client.clone(); // WeakEntity<ApiClient> — for workspace mutations

        TitleBar::new()
            .h(px(32.))
            .bg(cx.theme().title_bar)
            .on_close_window(move |_, _, cx| {
                if let Some((_, settings_handle)) = settings_window.clone() {
                    cx.update_window(settings_handle, |_root, window, _cx| {
                        window.remove_window();
                    })
                    .ok();
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
                                let client = client.clone();
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
                                            let client = client.clone();
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
                                                        let client = client.clone();
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
                                                            client
                                                                .update(cx, |c, cx| {
                                                                    c.add_workspace(
                                                                        name, path, window, cx,
                                                                    );
                                                                })
                                                                .ok();
                                                            this.update(cx, |t, cx| {
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
                                            let client = client.clone();
                                            move |index, window, cx| {
                                                client
                                                    .update(cx, |c, cx| {
                                                        c.switch_workspace_to(
                                                            index.row, window, cx,
                                                        );
                                                    })
                                                    .ok();
                                                this.update(cx, |t, cx| {
                                                    t.workspace_palette_open = false;
                                                    cx.notify();
                                                });
                                            }
                                        })
                                        .into_any_element()
                                }
                            })
                    })
                    .when(!active_env.is_empty(), |builder| {
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
                                        .label(active_env.clone())
                                        .tooltip("Switch Environment"),
                                )
                                .content(move |_, _, cx| {
                                    let envs = ep.read(cx).envs.clone();
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
                                                let envs = ep.read(cx).envs.clone();
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
