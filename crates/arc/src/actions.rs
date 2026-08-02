// Copyright (c) 2026 Muhammad Owais Warsi
// SPDX-License-Identifier: Apache-2.0

use gpui::Action;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent_id: usize,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct RenameFile {
    pub node_id: usize,
    pub node_name: String,
    pub new_name: String,
}
