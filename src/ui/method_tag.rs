use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ColorName, Sizable};
use gpui_kit::*;

pub fn method_tag(method: &str) -> impl IntoElement {
    match method {
        "GET" => Tag::color(ColorName::Green).outline().child("GET").xsmall(),

        "POST" => Tag::color(ColorName::Blue).outline().child("POST").xsmall(),

        "PUT" => Tag::color(ColorName::Yellow)
            .outline()
            .child("PUT")
            .xsmall(),

        "PATCH" => Tag::color(ColorName::Orange)
            .outline()
            .child("PATCH")
            .xsmall(),

        "DELETE" => Tag::color(ColorName::Red)
            .outline()
            .child("DELETE")
            .xsmall(),

        "HEAD" => Tag::color(ColorName::Purple)
            .outline()
            .child("HEAD")
            .xsmall(),

        "OPTIONS" => Tag::color(ColorName::Gray)
            .outline()
            .child("OPTIONS")
            .xsmall(),

        _ => Tag::color(ColorName::Neutral)
            .outline()
            .child(method.to_string())
            .xsmall(),
    }
}
