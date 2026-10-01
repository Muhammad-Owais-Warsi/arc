//! Demo theme previews shaped like an API client.
//!
//! Every color is read straight out of the candidate theme's Zed-format
//! `highlight` block and applied as an explicit inline style. GPUI has no
//! scoped theme override (the theme is one `Global` on `App`), so a preview
//! must never swap the active theme -- this renders beside the real UI
//! instead. Only the card's own border uses the active theme, which is
//! identical for every card and changes nothing.

use std::rc::Rc;

use gpui_kit::component::ThemeConfig;
use gpui_kit::component::highlighter::HighlightThemeStyle;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::theme::{get_theme_config, get_themes};

/// Colors resolved for one candidate theme.
struct PreviewPalette {
    background: Hsla,
    surface: Hsla,
    foreground: Hsla,
    dim: Hsla,
    keyword: Hsla,
    property: Hsla,
    string: Hsla,
    number: Hsla,
    boolean: Hsla,
    function: Hsla,
    punctuation: Hsla,
    success: Hsla,
}

impl PreviewPalette {
    fn from_config(config: &ThemeConfig, cx: &App) -> Self {
        let style: HighlightThemeStyle = config.highlight.clone().unwrap_or_default();
        let syntax = style.syntax.clone();

        // `SyntaxColors::style` already returns a public `HighlightStyle`.
        let token = |name: &str, fallback: Hsla| {
            syntax
                .style(name)
                .and_then(|style| style.color)
                .unwrap_or(fallback)
        };

        let background = style
            .editor_background
            .unwrap_or_else(|| hsla(0.14, 0., 0., 1.));
        let foreground = style
            .editor_foreground
            .unwrap_or_else(|| hsla(0.9, 0., 0.9, 1.));
        let surface = style
            .editor_gutter_background
            .or(style.editor_active_line)
            .unwrap_or(foreground.opacity(0.06));

        Self {
            background,
            surface,
            foreground,
            dim: style
                .editor_line_number
                .unwrap_or(foreground.opacity(0.55)),
            keyword: token("keyword", foreground),
            property: token("property", foreground),
            string: token("string", foreground),
            number: token("number", foreground),
            boolean: token("boolean", foreground),
            function: token("function", foreground),
            punctuation: token("punctuation", foreground.opacity(0.7)),
            success: style.status.success(cx),
        }
    }
}

/// One small rounded block standing in for a run of text.
fn bar(color: Hsla, width: f32, height: f32) -> impl IntoElement {
    div()
        .h(px(height))
        .w(px(width))
        .flex_none()
        .rounded(px(2.))
        .bg(color)
}

/// Deterministic pseudo-random source so each card has its own bar pattern
/// but stays stable across frames and re-orderings.
struct Seed(u32);

impl Seed {
    fn new(seed: &str) -> Self {
        // FNV-1a over the theme name.
        let mut hash: u32 = 2166136261;
        for byte in seed.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(16777619);
        }
        Self(hash | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / (u32::MAX >> 8) as f32
    }

    fn between(&mut self, min: f32, max: f32) -> f32 {
        min + self.next() * (max - min)
    }
}

/// The request bar: method chip plus a run of url bars, one of them standing
/// in for an environment variable pill.
fn request_row(p: &PreviewPalette, seed: &mut Seed) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap(px(4.))
        .h(px(26.))
        .px(px(7.))
        .bg(p.surface)
        .child(bar(p.keyword, 22., 9.))
        .child(bar(p.foreground.opacity(0.55), 10., 5.))
        .child(bar(p.keyword.opacity(0.75), seed.between(20., 30.), 8.))
        .child(bar(p.dim, seed.between(12., 20.), 5.))
}

/// The response meta strip: status, timing, size.
fn meta_row(p: &PreviewPalette, seed: &mut Seed) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap(px(4.))
        .h(px(16.))
        .px(px(7.))
        .child(bar(p.success, 18., 5.))
        .child(bar(p.dim, seed.between(14., 20.), 4.))
        .child(bar(p.dim, seed.between(12., 18.), 4.))
}

/// Indented rows of bars in syntax colors, standing in for a response body.
fn body(p: &PreviewPalette, seed: &mut Seed) -> impl IntoElement {
    // Candidate colors a body could use, sampled the way Zed does.
    let swatches = [
        p.property,
        p.string,
        p.number,
        p.boolean,
        p.keyword,
        p.function,
        p.punctuation,
    ];

    let mut lines = v_flex().flex_1().min_h(px(0.)).px(px(7.)).pt(px(6.));
    for _ in 0..6 {
        let indent = seed.between(0., 3.) * 8.;
        let mut row = h_flex().gap(px(3.)).pl(px(indent)).h(px(8.));
        let blocks = 2 + (seed.next() * 2.0) as usize;
        for _ in 0..blocks {
            let color = swatches[(seed.next() * swatches.len() as f32) as usize % swatches.len()];
            row = row.child(bar(color, seed.between(10., 30.), 4.));
        }
        lines = lines.child(row);
    }
    lines
}

/// One theme rendered as a miniature API client made of abstract bars.
/// `selected` only changes the card's chrome; the interior always shows the
/// candidate theme.
pub fn theme_preview_tile(
    name: SharedString,
    config: Rc<ThemeConfig>,
    selected: bool,
    cx: &App,
) -> impl IntoElement {
    let palette = PreviewPalette::from_config(&config, cx);
    let mut seed = Seed::new(&name);

    div()
        .w(px(152.))
        .h(px(104.))
        .flex_none()
        .overflow_hidden()
        .rounded(px(6.))
        .when(selected, |this| {
            this.border_2().border_color(cx.theme().primary)
        })
        .when(!selected, |this| {
            // Card chrome follows the active theme.
            this.border_1().border_color(cx.theme().border)
        })
        .bg(palette.background)
        .child(request_row(&palette, &mut seed))
        .child(meta_row(&palette, &mut seed))
        .child(body(&palette, &mut seed))
}

/// Theme name sits outside the card, like Zed's.
fn tile_with_label(name: SharedString, config: Rc<ThemeConfig>, cx: &App) -> impl IntoElement {
    v_flex()
        .gap(px(5.))
        .flex_none()
        .child(theme_preview_tile(name.clone(), config, false, cx))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(name.to_string()),
        )
}

/// Demo strip: every installed theme side by side.
pub fn theme_preview_demo(cx: &App) -> impl IntoElement {
    let tiles: Vec<AnyElement> = get_themes(cx)
        .into_iter()
        .filter_map(|(name, _)| {
            get_theme_config(cx, &name)
                .map(|config| tile_with_label(name.clone(), config, cx).into_any_element())
        })
        .collect();

    v_flex()
        .flex_none()
        .gap(px(6.))
        .px(px(24.))
        .pb(px(12.))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Theme preview (demo -- read-only, does not change the active theme)"),
        )
        .child(h_flex().gap(px(10.)).overflow_x_scrollbar().children(tiles))
}