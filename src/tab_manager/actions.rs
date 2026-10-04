use gpui_kit::actions;
use gpui_kit::Action;
use schemars::JsonSchema;
use serde::Deserialize;

actions!(
    tabs,
    [
        NewRequestTab,
        CloseTab,
        NextTab,
        PrevTab,
        ReopenClosedTab,
    ]
);

/// Activate the center tab at the given visual index (0-based).
/// Tuple shape so keymaps pass it positionally: `["tabs::ActivateTab", 0]`.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = tabs)]
pub struct ActivateTab(pub usize);
