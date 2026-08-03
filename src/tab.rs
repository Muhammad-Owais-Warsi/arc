use crate::helpers::render_method_tag;
use crate::playground::Playground;
use gpui::*;
use gpui_component::Sizable;
use gpui_component::button::Button;
use gpui_component::button::ButtonVariants;
use gpui_component::tab::Tab;

use crate::icons::IconName;

pub enum TabEvent {
    Close(usize), // node_id
}

impl EventEmitter<TabEvent> for Tabs {}

pub struct Tabs {
    id: usize,
    node_id: usize,
    name: String,
    playground: Entity<Playground>,
}

impl Tabs {
    pub fn new(id: usize, node_id: usize, name: String, playground: Entity<Playground>) -> Self {
        Self {
            id,
            node_id,
            name,
            playground,
        }
    }

    pub fn playground(&self) -> Entity<Playground> {
        self.playground.clone()
    }

    // pub fn name(&self) -> String {
    //     self.name.clone()
    // }

    // pub fn node_id(&self) -> usize {
    //     self.node_id
    // }

    pub fn update_name(&mut self, new_name: String) {
        self.name = new_name
    }

    pub fn to_tab_element(&self, cx: &mut Context<Self>) -> Tab {
        let method = self.playground.read(cx).method(cx);
        let node_id = self.node_id;

        Tab::default()
            .min_h(px(32.))
            .px_1()
            .prefix(div().mr_1().child(render_method_tag(&method)))
            .label(self.name.clone())
            .suffix(
                Button::new(("close-tab", self.id))
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .on_click(cx.listener(move |_, _, _window, cx| {
                        cx.emit(TabEvent::Close(node_id));
                    })),
            )
    }
}

impl Render for Tabs {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.to_tab_element(cx)
    }
}
