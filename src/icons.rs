use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, SharedString, Window};
use strum::AsRefStr;

#[derive(AsRefStr, IntoElement)]
#[strum(serialize_all = "kebab_case")]
pub enum IconName {
    ArrowDown,
    ArrowUp,
    ArrowLeft,
    ArrowRight,
    SquarePen,
    X,
    Check,
    Eye,
    EyeOff,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    BriefcaseBusiness,
    Stop,
    Close,
    Folder,
    FolderOpen,
    PanelBottom,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
    Send,
    Spinner,
    FolderTree,
    Trash,
    File,
    Rename,
    ExternalLink,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
    Copy,
    Inbox,
    Settings,
    Minus,
    Search,
    Undo2,
    Info,
    Variable,
    SquareMenu,
    Settings2,
    Menu,
    Monitor,
    SlidersHorizontal,
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
