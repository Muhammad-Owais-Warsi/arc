use gpui_kit::component::Side;
use gpui_kit::{App, Global};
use serde::{Deserialize, Serialize};

use crate::fs;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarDock {
    #[default]
    Left,
    Right,
}

impl SidebarDock {
    pub fn to_side(self) -> Side {
        match self {
            Self::Left => Side::Left,
            Self::Right => Side::Right,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSettings {
    pub name: String,
    pub mode: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            name: "One Dark".into(),
            mode: "dark".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: "Lilex".into(),
            size: 16.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FilePanelSettings {
    pub sidebar_dock: SidebarDock,
}

impl Default for FilePanelSettings {
    fn default() -> Self {
        Self {
            sidebar_dock: SidebarDock::Left,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvPanelSettings {
    pub sidebar_dock: SidebarDock,
}

impl Default for EnvPanelSettings {
    fn default() -> Self {
        Self {
            sidebar_dock: SidebarDock::Right,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestPlaygroundSettings {
    pub save_on_close: bool,
}

impl Default for RequestPlaygroundSettings {
    fn default() -> Self {
        Self {
            save_on_close: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    #[serde(alias = "project_panel")]
    pub file_panel: FilePanelSettings,
    pub env_panel: EnvPanelSettings,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            file_panel: FilePanelSettings::default(),
            env_panel: EnvPanelSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaygroundSettings {
    pub request_playground: RequestPlaygroundSettings,
}

impl Default for PlaygroundSettings {
    fn default() -> Self {
        Self {
            request_playground: RequestPlaygroundSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: ThemeSettings,
    pub font: FontSettings,
    pub panel: PanelSettings,
    pub playground: PlaygroundSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeSettings::default(),
            font: FontSettings::default(),
            panel: PanelSettings::default(),
            playground: PlaygroundSettings::default(),
        }
    }
}

impl Global for AppSettings {}

impl AppSettings {
    pub fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }

    pub fn get() -> Self {
        let content = fs::settings::read();
        serde_json::from_str(content.as_str()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::settings::write(&content);
        }
    }
}
