//! Zed-style keymap file layer: JSONC sections of `{ context, unbind, bindings }`.
//!
//! The dispatch engine itself lives in gpui (`Keymap`, `KeyBinding`,
//! `KeyBindingContextPredicate`); this file only turns text into bindings.

use gpui_kit::{
    Action, App, DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate,
    KeyBindingMetaIndex, NoAction, SharedString, Unbind,
};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KeymapFile(pub Vec<KeymapSection>);

#[derive(Debug, Deserialize)]
pub struct KeymapSection {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub use_key_equivalents: bool,
    #[serde(default)]
    pub unbind: IndexMap<String, UnbindTarget>,
    #[serde(default)]
    pub bindings: IndexMap<String, Option<KeymapAction>>,
}

/// `"name"` | `["name", args]` — JSON `null` arrives as `None` via the `Option`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum KeymapAction {
    WithArgs(String, serde_json::Value),
    Name(String),
}

/// `["gpui::Unbind", "request::SaveRequest"]`
#[derive(Debug, Deserialize)]
pub struct UnbindTarget(String, String);

impl KeymapFile {
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(&strip_jsonc(text))?)
    }

    /// Registry lookups need `cx`, so loading borrows App. Returns
    /// (bindings, per-section error messages) — never fails wholesale.
    pub fn load(&self, cx: &App, meta: KeyBindingMetaIndex) -> (Vec<KeyBinding>, Vec<String>) {
        let mut out = Vec::new();
        let mut errors = Vec::new();
        for section in &self.0 {
            let predicate = if section.context.is_empty() {
                None
            } else {
                match KeyBindingContextPredicate::parse(&section.context) {
                    Ok(predicate) => Some(std::rc::Rc::new(predicate)),
                    Err(e) => {
                        errors.push(format!("bad context {:?}: {e:?}", section.context));
                        continue;
                    }
                }
            };
            // Unbind entries first, same as Zed.
            for (chord, target) in &section.unbind {
                let action =
                    Box::new(Unbind(SharedString::from(target.1.clone()))) as Box<dyn Action>;
                match KeyBinding::load(
                    chord,
                    action,
                    predicate.clone(),
                    section.use_key_equivalents,
                    None,
                    &DummyKeyboardMapper,
                ) {
                    Ok(binding) => out.push(binding.with_meta(meta)),
                    Err(e) => errors.push(format!("bad chord {chord:?}: {e:?}")),
                }
            }
            for (chord, slot) in &section.bindings {
                let action: Option<Box<dyn Action>> = match slot {
                    None => Some(Box::new(NoAction)),
                    Some(KeymapAction::Name(name)) => match cx.build_action(name, None) {
                        Ok(action) => Some(action),
                        Err(e) => {
                            errors.push(format!("unknown action {name:?}: {e:?}"));
                            None
                        }
                    },
                    Some(KeymapAction::WithArgs(name, args)) => {
                        match cx.build_action(name, Some(args.clone())) {
                            Ok(action) => Some(action),
                            Err(e) => {
                                errors.push(format!("bad args for {name:?}: {e:?}"));
                                None
                            }
                        }
                    }
                };
                let Some(action) = action else { continue };
                match KeyBinding::load(
                    chord,
                    action,
                    predicate.clone(),
                    section.use_key_equivalents,
                    None,
                    &DummyKeyboardMapper,
                ) {
                    Ok(binding) => out.push(binding.with_meta(meta)),
                    Err(e) => errors.push(format!("bad chord {chord:?}: {e:?}")),
                }
            }
        }
        (out, errors)
    }
}

/// Strip `//` comments, string-aware so `https://` inside quotes survives.
/// Then drop trailing commas (`{"a": 1,}`), which JSONC allows.
fn strip_jsonc(text: &str) -> String {
    strip_trailing_commas(&strip_comments(text))
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_str = false;
    let mut escaped = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn strip_trailing_commas(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut in_str = false;
    let mut escaped = false;
    for (i, c) in chars.iter().enumerate() {
        if in_str {
            out.push(*c);
            if escaped {
                escaped = false;
            } else if *c == '\\' {
                escaped = true;
            } else if *c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(*c);
            }
            ',' => {
                // Keep the comma unless only whitespace lies between it
                // and a closing `}` or `]`.
                let trailing = chars[i + 1..]
                    .iter()
                    .copied()
                    .skip_while(|c| c.is_whitespace())
                    .next()
                    .is_some_and(|c| c == '}' || c == ']');
                if !trailing {
                    out.push(*c);
                }
            }
            _ => out.push(*c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_args_null_and_unbind() {
        let file = KeymapFile::parse(
            r#"[
              // comment + trailing comma must survive
              { "context": "RequestPlayground",
                "unbind": { "cmd-s": ["gpui::Unbind", "request::SaveRequest"] },
                "bindings": {
                  "cmd-enter": "request::SendRequest",
                  "cmd-1": ["tabs::ActivateTab", 0],
                  "cmd-x": null,
                } },
            ]"#,
        )
        .unwrap();
        assert_eq!(file.0.len(), 1);
        assert_eq!(file.0[0].bindings.len(), 3);
        assert_eq!(file.0[0].unbind.len(), 1);
    }

    #[test]
    fn broken_section_reports_without_panicking() {
        assert!(KeymapFile::parse("[{oops]").is_err());
        let file = KeymapFile::parse("[{}]").unwrap();
        assert!(file.0[0].bindings.is_empty());
    }
}
