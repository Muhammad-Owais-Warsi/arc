use gpui::*;

use crate::response_panel::ResponsePanel;

pub trait Playground: Render + 'static {
    fn method(&self, cx: &App) -> String;
    fn response_panel(&self, cx: &App) -> Option<Entity<ResponsePanel>>;
}

pub trait PlaygroundHandle {
    fn method(&self, cx: &App) -> String;
    fn response_panel(&self, cx: &App) -> Option<Entity<ResponsePanel>>;
    fn render_into(&self) -> AnyElement;
    fn entity(&self) -> AnyEntity;
    fn clone_box(&self) -> Box<dyn PlaygroundHandle>;
}

impl<V: Playground> PlaygroundHandle for Entity<V> {
    fn method(&self, cx: &App) -> String {
        self.read(cx).method(cx)
    }
    fn response_panel(&self, cx: &App) -> Option<Entity<ResponsePanel>> {
        self.read(cx).response_panel(cx)
    }
    fn render_into(&self) -> AnyElement {
        self.clone().into_any_element()
    }
    fn entity(&self) -> AnyEntity {
        self.clone().into()
    }
    fn clone_box(&self) -> Box<dyn PlaygroundHandle> {
        Box::new(self.clone())
    }
}
