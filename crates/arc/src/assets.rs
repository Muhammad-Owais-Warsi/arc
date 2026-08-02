// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use gpui::*;
// use ui::{IconName, Root, v_flex};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// An asset source that loads assets from the repository root `./assets` folder.
#[derive(RustEmbed)]
#[folder = "../../assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
