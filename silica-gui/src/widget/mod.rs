mod button;
mod label;
mod slider;

pub use self::{button::*, label::*, slider::*};
use crate::*;

#[derive(Default)]
pub struct NodeBuilder {
    style: Style,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn modify_style<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Style),
    {
        f(&mut self.style);
        self
    }
    pub fn parent(mut self, parent: impl Into<NodeId>) -> Self {
        self.parent = Some(parent.into());
        self
    }
    pub fn child(mut self, child: impl Into<NodeId>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn children(mut self, iter: impl IntoIterator<Item = NodeId>) -> Self {
        self.children.extend(iter);
        self
    }
    pub fn build(self, gui: &mut Gui) -> NodeId {
        let node = gui.create_node(self.style);
        gui.set_node_children(node, self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, node);
        }
        node
    }
    pub fn build_widget<W: Widget>(self, gui: &mut Gui, widget: W) -> WidgetId<W> {
        let widget = gui.create_widget(self.style, widget);
        gui.set_node_children(widget, self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, widget);
        }
        widget
    }
}
