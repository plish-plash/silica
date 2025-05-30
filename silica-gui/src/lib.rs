mod render;
pub mod theme;
mod widget;

use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    rc::Rc,
};

use glyphon::FontSystem;
use silica_wgpu::SurfaceSize;
use taffy::{AvailableSpace, Layout, PrintTree, Style, TaffyTree, TraversePartialTree};

pub use glyphon;
pub use render::*;
pub use silica_color::Rgba;
pub use silica_gui_macros::*;
pub use taffy::{self, NodeId};
pub use widget::*;

pub type Point = euclid::Point2D<f32, silica_wgpu::Surface>;
pub type Vector = euclid::Vector2D<f32, silica_wgpu::Surface>;
pub type Size = euclid::Size2D<f32, silica_wgpu::Surface>;
pub type Rect = euclid::Box2D<f32, silica_wgpu::Surface>;
pub type SideOffsets = euclid::SideOffsets2D<f32, silica_wgpu::Surface>;

trait LayoutExt {
    fn border_box(&self) -> Rect;
    fn content_box(&self) -> Rect;
    fn padding(&self) -> SideOffsets;
}

impl LayoutExt for Layout {
    fn border_box(&self) -> Rect {
        Rect::from_origin_and_size(
            Point::new(self.location.x, self.location.y),
            euclid::Size2D::new(self.size.width, self.size.height),
        )
    }
    fn content_box(&self) -> Rect {
        Rect::from_origin_and_size(
            Point::new(self.content_box_x(), self.content_box_y()),
            euclid::Size2D::new(self.content_box_width(), self.content_box_height()),
        )
    }
    fn padding(&self) -> SideOffsets {
        SideOffsets::new(
            self.padding.top,
            self.padding.right,
            self.padding.bottom,
            self.padding.left,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    key: char,
    shift: bool,
    ctrl: bool,
    alt: bool,
}

pub trait KeyboardEvent {
    fn to_hotkey(&self) -> Option<Hotkey>;
}

pub trait MouseButtonEvent {
    fn is_primary_button(&self) -> bool;
    fn is_pressed(&self) -> bool;
}

pub enum InputEvent<Keyboard, MouseButton> {
    Keyboard(Keyboard),
    MouseMotion(Point),
    MouseButton(MouseButton),
    MouseWheel(f32),
}

#[derive(Default)]
pub struct GuiInput {
    pub blocked: bool,
    pub grabbed: bool,
    pub pointer: Point,
    pub button_pressed: bool,
    pub clicked: bool,
    pub double_clicked: bool,
    pub hotkey: Option<Hotkey>,
}

impl GuiInput {
    fn process<K: KeyboardEvent, M: MouseButtonEvent>(&mut self, event: &InputEvent<K, M>) {
        match event {
            InputEvent::Keyboard(keyboard_event) => self.hotkey = keyboard_event.to_hotkey(),
            InputEvent::MouseMotion(point) => self.pointer = *point,
            InputEvent::MouseButton(mouse_button_event) => {
                if mouse_button_event.is_primary_button() {
                    if !self.button_pressed && mouse_button_event.is_pressed() {
                        self.clicked = true;
                    }
                    self.button_pressed = mouse_button_event.is_pressed();
                }
            }
            InputEvent::MouseWheel(_) => {}
        }
    }
    fn reset(&mut self) {
        self.blocked = false;
        self.grabbed = false;
        self.clicked = false;
        self.double_clicked = false;
        self.hotkey = None;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Pass,
    Block,
    Grab,
}

pub trait EventContext {
    fn get_by_type(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct EventFn(Rc<dyn Fn(&mut dyn EventContext, Option<Box<dyn Any>>)>);

impl EventFn {
    fn get_context<C: 'static>(context: &mut dyn EventContext) -> &mut C {
        context
            .get_by_type(TypeId::of::<C>())
            .expect("EventContext doesn't provide requested type")
            .downcast_mut()
            .expect("EventContext::get_by_type return type mismatch")
    }
    pub fn new<C, F>(f: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        EventFn(Rc::new(move |context, _param| {
            f(Self::get_context(context));
        }))
    }
    pub fn new_param<C, P, F>(f: F) -> Self
    where
        C: 'static,
        P: 'static,
        F: Fn(&mut C, P) + 'static,
    {
        EventFn(Rc::new(move |context, param| {
            let param = param
                .expect("no parameter for event")
                .downcast()
                .expect("event parameter wrong type");
            f(Self::get_context(context), *param);
        }))
    }
}

#[must_use]
#[derive(Default)]
pub struct EventExecutor {
    funcs: Vec<(EventFn, Option<Box<dyn Any>>)>,
    draw_dirty: bool,
}

impl EventExecutor {
    pub fn new() -> Self {
        EventExecutor::default()
    }
    pub fn mark_draw_dirty(&mut self) {
        self.draw_dirty = true;
    }
    pub fn queue(&mut self, event: EventFn, param: Option<Box<dyn Any>>) {
        self.funcs.push((event, param));
    }
    pub fn execute(self, context: &mut impl EventContext) {
        for func in self.funcs {
            func.0 .0(context, func.1);
        }
    }
}

pub trait Upcast {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[allow(unused)]
pub trait Widget: Upcast + 'static {
    fn measure(
        &mut self,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<AvailableSpace>,
        font_system: &mut FontSystem,
    ) -> taffy::Size<f32> {
        taffy::Size::ZERO
    }
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, rect: Rect) -> InputAction {
        InputAction::Pass
    }
    fn visible(&self) -> bool {
        true
    }
    fn draw<'a>(
        &'a self,
        batcher: &mut GuiBatcher<'a>,
        theme: &dyn theme::Theme,
        rect: Rect,
        padding: SideOffsets,
    );
}

impl<T: Widget> Upcast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait WidgetBuilder {
    type Properties<'a>;
}

#[derive(Default)]
pub struct Container {
    pub layout: Style,
}

impl Container {
    pub fn create(gui: &mut Gui, properties: Container) -> NodeId {
        gui.create_container(properties.layout)
    }
}
impl WidgetBuilder for Container {
    type Properties<'a> = Self;
}

#[derive(PartialEq, Eq, Hash)]
pub struct WidgetId<T>(NodeId, PhantomData<T>);

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for WidgetId<T> {}
impl<T> From<WidgetId<T>> for NodeId {
    fn from(value: WidgetId<T>) -> Self {
        value.0
    }
}

pub struct Gui {
    font_system: FontSystem,
    tree: TaffyTree<Box<dyn Widget>>,
    root: NodeId,
    available_space: taffy::Size<AvailableSpace>,
    input: GuiInput,
    grabbed_node: Option<NodeId>,
    draw_dirty: bool,
}

impl Gui {
    pub fn new(font_system: FontSystem) -> Self {
        let mut tree = TaffyTree::new();
        let root = tree
            .new_leaf(Style {
                size: taffy::Size::percent(1.0),
                ..Default::default()
            })
            .unwrap();
        Gui {
            font_system,
            tree,
            root,
            available_space: taffy::Size::min_content(),
            input: GuiInput::default(),
            grabbed_node: None,
            draw_dirty: true,
        }
    }
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn set_root(&mut self, root: NodeId) {
        let mut layout = self.tree.style(root).unwrap().clone();
        layout.size = taffy::Size::percent(1.0);
        self.tree.set_style(root, layout).unwrap();
        self.root = root;
    }
    pub fn clear(&mut self) {
        self.tree.clear();
        self.root = self
            .tree
            .new_leaf(Style {
                size: taffy::Size::percent(1.0),
                ..Default::default()
            })
            .unwrap();
    }
    pub fn get_widget<W: Widget>(&self, id: WidgetId<W>) -> Option<&W> {
        self.tree.get_node_context(id.0).map(|context| {
            context
                .as_any()
                .downcast_ref()
                .expect("WidgetId has incorrect type")
        })
    }
    pub fn get_widget_mut<W: Widget>(&mut self, id: WidgetId<W>) -> Option<&mut W> {
        self.tree.get_node_context_mut(id.0).map(|context| {
            context
                .as_any_mut()
                .downcast_mut()
                .expect("WidgetId has incorrect type")
        })
    }
    pub fn get_widget_and_font_system<W: Widget>(
        &mut self,
        id: WidgetId<W>,
    ) -> Option<(&mut W, &mut FontSystem)> {
        self.tree.get_node_context_mut(id.0).map(|context| {
            (
                context
                    .as_any_mut()
                    .downcast_mut()
                    .expect("WidgetId has incorrect type"),
                &mut self.font_system,
            )
        })
    }
    #[must_use]
    pub fn create_widget<W: Widget>(&mut self, layout: Style, widget: W) -> WidgetId<W> {
        WidgetId(
            self.tree
                .new_leaf_with_context(layout, Box::new(widget))
                .unwrap(),
            PhantomData,
        )
    }
    #[must_use]
    pub fn create_container(&mut self, layout: Style) -> NodeId {
        self.tree.new_leaf(layout).unwrap()
    }
    pub fn set_node_widget<W: Widget>(&mut self, node: NodeId, widget: W) -> WidgetId<W> {
        self.tree
            .set_node_context(node, Some(Box::new(widget)))
            .unwrap();
        WidgetId(node, PhantomData)
    }
    pub fn delete(&mut self, node: impl Into<NodeId>) {
        self.tree.remove(node.into()).unwrap();
    }
    pub fn delete_children(&mut self, parent: impl Into<NodeId>) {
        self.tree.remove_children_range(parent.into(), ..).unwrap();
    }
    pub fn add_child(&mut self, parent: impl Into<NodeId>, child: impl Into<NodeId>) {
        self.tree.add_child(parent.into(), child.into()).unwrap();
    }
    pub fn remove_child(&mut self, parent: impl Into<NodeId>, child: impl Into<NodeId>) {
        self.tree.remove_child(parent.into(), child.into()).unwrap();
    }
    pub fn set_layout(&mut self, node: impl Into<NodeId>, mut layout: Style) {
        let node = node.into();
        if node == self.root {
            layout.size = taffy::Size::percent(1.0);
        }
        self.tree.set_style(node, layout).unwrap();
    }
    pub fn dirty(&self) -> bool {
        let layout_dirty = self.tree.dirty(self.root).unwrap();
        layout_dirty || self.draw_dirty
    }
    pub fn mark_layout_dirty(&mut self, node: impl Into<NodeId>) {
        self.tree.mark_dirty(node.into()).unwrap();
    }
    pub fn mark_draw_dirty(&mut self) {
        self.draw_dirty = true;
    }

    pub fn set_available_space(&mut self, available_space: taffy::Size<AvailableSpace>) {
        self.available_space = available_space;
        self.mark_layout_dirty(self.root);
    }
    pub fn set_surface_size(&mut self, size: SurfaceSize) {
        self.set_available_space(taffy::Size {
            width: AvailableSpace::Definite(size.width as f32),
            height: AvailableSpace::Definite(size.height as f32),
        });
    }
    pub fn layout(&mut self) {
        if !self.tree.dirty(self.root).unwrap() {
            return;
        }
        self.tree
            .compute_layout_with_measure(
                self.root,
                self.available_space,
                |known_dimensions, available_space, _node, context, _style| {
                    if let taffy::Size {
                        width: Some(width),
                        height: Some(height),
                    } = known_dimensions
                    {
                        taffy::Size { width, height }
                    } else if let Some(context) = context {
                        context.measure(known_dimensions, available_space, &mut self.font_system)
                    } else {
                        taffy::Size::ZERO
                    }
                },
            )
            .unwrap();
        self.draw_dirty = true;
    }
    pub fn get_rect(&self, node: impl Into<NodeId>) -> Rect {
        let mut node = node.into();
        let layout = self.tree.get_final_layout(node);
        let mut rect = layout.content_box();
        while let Some(parent) = self.tree.parent(node) {
            let parent_layout = self.tree.get_final_layout(parent);
            rect = rect.translate(Vector::new(
                parent_layout.location.x,
                parent_layout.location.y,
            ));
            node = parent;
        }
        rect
    }

    fn dispatch_input_event(
        &mut self,
        node: NodeId,
        mut offset: Vector,
        executor: &mut EventExecutor,
    ) {
        let layout = self.tree.get_final_layout(node);
        let rect = layout.content_box().translate(offset);
        offset.x += layout.location.x;
        offset.y += layout.location.y;

        let child_count = self.tree.child_count(node);
        for child_index in (0..child_count).rev() {
            let child = self.tree.child_at_index(node, child_index).unwrap();
            self.dispatch_input_event(child, offset, executor);
        }
        if let Some(context) = self.tree.get_node_context_mut(node) {
            match context.input(&self.input, executor, rect) {
                InputAction::Pass => {}
                InputAction::Block => {
                    self.input.blocked = true;
                }
                InputAction::Grab => {
                    self.input.blocked = true;
                    self.grabbed_node = Some(node);
                }
            }
        }
    }
    pub fn input_event<K: KeyboardEvent, M: MouseButtonEvent>(
        &mut self,
        event: InputEvent<K, M>,
    ) -> (EventExecutor, Option<InputEvent<K, M>>) {
        self.input.process(&event);
        let mut executor = EventExecutor::new();
        if let Some(node) = self.grabbed_node.take() {
            self.input.grabbed = true;
            self.dispatch_input_event(node, Vector::zero(), &mut executor);
        } else {
            self.dispatch_input_event(self.root, Vector::zero(), &mut executor);
        }
        self.draw_dirty |= executor.draw_dirty;
        let event = if self.input.blocked {
            None
        } else {
            Some(event)
        };
        self.input.reset();
        (executor, event)
    }
}
impl EventContext for Gui {
    fn get_by_type(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
        if type_id == TypeId::of::<Gui>() {
            Some(self)
        } else {
            None
        }
    }
}
