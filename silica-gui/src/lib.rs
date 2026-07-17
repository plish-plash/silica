pub mod render;
pub mod theme;
mod widget;

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    rc::Rc,
};

pub use glyphon;
use silica_asset::{AssetError, AssetSource};
pub use silica_color::Rgba;
pub use silica_layout::*;
use silica_wgpu::{Context, ImmediateBatcher, draw::draw_border, wgpu};
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use crate::render::GuiRenderer;
pub use crate::{theme::Theme, widget::*};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub key: char,
    pub mod1: bool,
    pub mod2: bool,
}

impl Hotkey {
    pub fn new(key: char) -> Self {
        Hotkey {
            key,
            mod1: false,
            mod2: false,
        }
    }
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

pub trait EventContainer {
    fn get_by_type(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;
}

pub struct EventContext<'a> {
    container: &'a mut dyn EventContainer,
    executor: &'a mut EventExecutor,
}

impl<'a> EventContext<'a> {
    pub fn node_id(&self) -> NodeId {
        self.executor.current_node
    }
    pub fn widget_id<T>(&self) -> WidgetId<T> {
        WidgetId(self.node_id(), PhantomData)
    }
    pub fn widget<T: Widget>(&mut self) -> &mut T {
        let id = self.widget_id();
        self.gui().get_widget_mut(id).unwrap()
    }
    pub fn container<C: 'static>(&mut self) -> &mut C {
        self.container
            .get_by_type(TypeId::of::<C>())
            .expect("EventContext doesn't provide requested type")
            .downcast_mut()
            .expect("EventContext::get_by_type return type mismatch")
    }
    pub fn gui(&mut self) -> &mut Gui {
        self.container()
    }
    pub fn executor(&mut self) -> &mut EventExecutor {
        self.executor
    }
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct EventFn(Rc<dyn Fn(EventContext, Option<Box<dyn Any>>)>);

impl EventFn {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(EventContext) + 'static,
    {
        EventFn(Rc::new(move |context, _param| {
            f(context);
        }))
    }
    pub fn new_param<P, F>(f: F) -> Self
    where
        P: 'static,
        F: Fn(EventContext, P) + 'static,
    {
        EventFn(Rc::new(move |context, param| {
            let param = param
                .expect("no parameter for event")
                .downcast()
                .expect("event parameter wrong type");
            f(context, *param);
        }))
    }
}

#[must_use]
#[derive(Default)]
pub struct EventExecutor {
    current_node: NodeId,
    funcs: Vec<(NodeId, EventFn, Option<Box<dyn Any>>)>,
    redraw: bool,
}

impl EventExecutor {
    pub fn new() -> Self {
        EventExecutor::default()
    }
    pub fn queue(&mut self, event: EventFn, param: Option<Box<dyn Any>>) {
        self.queue_node(self.current_node, event, param);
    }
    pub fn queue_node(&mut self, node: impl Into<NodeId>, event: EventFn, param: Option<Box<dyn Any>>) {
        self.funcs.push((node.into(), event, param));
    }
    pub fn execute(mut self, container: &mut impl EventContainer) {
        while !self.funcs.is_empty() {
            for func in std::mem::take(&mut self.funcs) {
                self.current_node = func.0;
                let context = EventContext {
                    container,
                    executor: &mut self,
                };
                func.1.0(context, func.2);
            }
        }
    }
    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }
    pub fn needs_redraw(&self) -> bool {
        self.redraw
    }
}

#[derive(Clone)]
pub struct FontSystem(Rc<RefCell<glyphon::FontSystem>>);

impl FontSystem {
    pub fn get_system_locale() -> String {
        sys_locale::get_locale().unwrap_or_else(|| {
            log::warn!("failed to get system locale, falling back to en-US");
            "en-US".to_string()
        })
    }
    pub fn new(db: glyphon::fontdb::Database) -> Self {
        FontSystem(Rc::new(RefCell::new(glyphon::FontSystem::new_with_locale_and_db(
            Self::get_system_locale(),
            db,
        ))))
    }
    pub fn with_font_assets<'a, S: AssetSource>(
        asset_source: &mut S,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, AssetError> {
        let mut db = glyphon::fontdb::Database::new();
        for path in paths {
            db.load_font_data(silica_asset::load_bytes(asset_source, path)?);
        }
        Ok(Self::new(db))
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, glyphon::FontSystem> {
        self.0.borrow_mut()
    }
}

pub trait Upcast {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[allow(unused)]
pub trait Widget: Upcast + 'static {
    fn measure(&mut self, available_space: Size) -> Size {
        Size::zero()
    }
    fn layout(&mut self, area: &Area) {}
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, area: &Area) -> InputAction {
        InputAction::Pass
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area);
}

impl<T: Widget> Upcast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl LayoutWidget for Box<dyn Widget> {
    fn measure(&mut self, available_space: Size) -> Size {
        Widget::measure(self.as_mut(), available_space)
    }
    fn layout(&mut self, area: &Area) {
        Widget::layout(self.as_mut(), area)
    }
}

new_key_type! { pub struct NodeId; }

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

pub type Node = silica_layout::Node<NodeId, Box<dyn Widget>>;

pub struct Gui {
    theme: Rc<dyn Theme>,
    nodes: SlotMap<NodeId, Node>,
    parents: SecondaryMap<NodeId, NodeId>,
    children: SecondaryMap<NodeId, Vec<NodeId>>,
    root: NodeId,
    popups: Vec<NodeId>,
    input: GuiInput,
    grabbed_node: Option<NodeId>,
    layout_area: Rect,
    needs_layout: bool,
    batcher: Option<ImmediateBatcher<render::Quad>>,
    exit_requested: bool,
}

impl Gui {
    pub fn new(theme: Rc<dyn Theme>) -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(Node::default());
        Gui {
            theme,
            nodes,
            parents: SecondaryMap::new(),
            children: SecondaryMap::new(),
            root,
            popups: Vec::new(),
            input: GuiInput::default(),
            grabbed_node: None,
            layout_area: Rect::zero(),
            needs_layout: false,
            batcher: None,
            exit_requested: false,
        }
    }

    pub fn font_system(&self) -> &FontSystem {
        self.theme.font_system()
    }
    pub fn background_color(&self) -> Rgba {
        self.theme.color(Color::Background)
    }
    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn set_root(&mut self, root: impl Into<NodeId>) {
        self.root = root.into();
        self.needs_layout = true;
    }
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.parents.clear();
        self.children.clear();
        self.root = self.nodes.insert(Node::default());
        self.needs_layout = true;
    }

    pub fn get_widget<W: Widget>(&self, id: WidgetId<W>) -> Option<&W> {
        self.nodes
            .get(id.into())
            .and_then(|node| node.widget.as_ref())
            .map(|widget| widget.as_any().downcast_ref().expect("WidgetId has incorrect type"))
    }
    pub fn get_widget_mut<W: Widget>(&mut self, id: WidgetId<W>) -> Option<&mut W> {
        self.nodes
            .get_mut(id.into())
            .and_then(|node| node.widget.as_mut())
            .map(|widget| widget.as_any_mut().downcast_mut().expect("WidgetId has incorrect type"))
    }
    #[must_use]
    pub fn create_widget<W: Widget>(&mut self, style: Style, widget: W) -> WidgetId<W> {
        WidgetId(self.nodes.insert(Node::new(style, Some(Box::new(widget)))), PhantomData)
    }
    #[must_use]
    pub fn create_node(&mut self, style: Style) -> NodeId {
        self.nodes.insert(Node::new(style, None))
    }
    pub(crate) fn set_node_children(&mut self, node: impl Into<NodeId>, children: Vec<NodeId>) {
        // assumes the node does not already have children
        if !children.is_empty() {
            let node = node.into();
            for child in children.iter() {
                self.parents.insert(*child, node);
            }
            self.children.insert(node, children);
            self.needs_layout = true;
        }
    }
    pub fn delete(&mut self, node: impl Into<NodeId>) {
        let node = node.into();
        if let Some(parent) = self.parents.remove(node) {
            self.remove_child(parent, node);
        }
        self.delete_children(node);
        self.nodes.remove(node);
    }
    pub fn delete_children(&mut self, parent: impl Into<NodeId>) {
        if let Some(children) = self.children.remove(parent.into()) {
            for child in children {
                self.delete_children(child);
                self.parents.remove(child);
                self.nodes.remove(child);
            }
            self.needs_layout = true;
        }
    }

    pub fn add_child(&mut self, parent: impl Into<NodeId>, child: impl Into<NodeId>) {
        let parent = parent.into();
        let child = child.into();
        if let Some(prev_parent) = self.parents.insert(child, parent) {
            self.remove_child(prev_parent, child);
        }
        self.children.entry(parent).unwrap().or_default().push(child);
        self.needs_layout = true;
    }
    pub fn remove_child(&mut self, parent: impl Into<NodeId>, child: impl Into<NodeId>) {
        if let Some(children) = self.children.get_mut(parent.into()) {
            let child = child.into();
            children.retain(|c| *c != child);
            self.parents.remove(child);
            self.needs_layout = true;
        }
    }
    pub fn get_parent(&self, child: impl Into<NodeId>) -> Option<NodeId> {
        self.parents.get(child.into()).copied()
    }

    pub fn add_popup(&mut self, popup: impl Into<NodeId>, point: Point, grab_input: bool) {
        let popup = popup.into();
        measure_and_layout(
            &mut self.nodes,
            &self.children,
            popup,
            Rect::new(point, Size::splat(i32::MAX)),
            true,
        );
        self.popups.push(popup);
        if grab_input {
            self.grabbed_node = Some(popup);
        }
    }
    pub fn remove_popup(&mut self, popup: impl Into<NodeId>) {
        let popup = popup.into();
        if let Some(index) = self.popups.iter().position(|p| *p == popup) {
            self.popups.remove(index);
        }
        if self.grabbed_node == Some(popup) {
            self.grabbed_node = None;
        }
    }

    pub fn get_style(&self, node: impl Into<NodeId>) -> &Style {
        &self.nodes.get(node.into()).unwrap().style
    }
    pub fn set_style(&mut self, node: impl Into<NodeId>, style: Style) {
        self.nodes.get_mut(node.into()).unwrap().style = style;
        self.needs_layout = true;
    }
    pub fn modify_style<F>(&mut self, node: impl Into<NodeId>, f: F)
    where
        F: FnOnce(&mut Style),
    {
        f(&mut self.nodes.get_mut(node.into()).unwrap().style);
        self.needs_layout = true;
    }
    pub fn get_area(&self, node: impl Into<NodeId>) -> &Area {
        &self.nodes.get(node.into()).unwrap().area
    }

    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }
    pub fn request_layout(&mut self) {
        self.needs_layout = true;
    }
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn set_area(&mut self, area: Rect) {
        if self.layout_area != area {
            self.layout_area = area;
            self.needs_layout = true;
        }
    }
    pub fn layout(&mut self) {
        if self.needs_layout {
            measure_and_layout(&mut self.nodes, &self.children, self.root, self.layout_area, false);
            self.needs_layout = false;
        }
    }

    fn render_node(
        id: NodeId,
        nodes: &mut SlotMap<NodeId, Node>,
        children: &SecondaryMap<NodeId, Vec<NodeId>>,
        renderer: &mut GuiRenderer,
    ) {
        let node = nodes.get_mut(id).unwrap();
        if node.area.hidden {
            return;
        }
        if let Some(background_color) = node.style.background_color {
            let color = renderer.theme().color(background_color);
            renderer.draw_theme_quad(render::Quad {
                rect: node.area.background_rect.to_box2d(),
                uv: GuiRenderer::UV_WHITE,
                color,
            });
        }
        if let Some(border_color) = node.style.border_color {
            let color = renderer.theme().color(border_color);
            draw_border(
                renderer,
                node.area.background_rect.to_box2d(),
                node.style.border,
                GuiRenderer::UV_WHITE,
                color,
            );
        }
        let scroll_count = renderer.scroll.len();
        if let Some(widget) = node.widget.as_mut() {
            widget.draw(renderer, &node.area);
        }
        if let Some(node_children) = children.get(id) {
            for child in node_children.iter() {
                Self::render_node(*child, nodes, children, renderer);
            }
        }
        while renderer.scroll.len() > scroll_count {
            renderer.pop_scroll_area();
        }
    }
    pub fn render(&mut self, context: &Context, pass: &mut wgpu::RenderPass, resources: &mut render::GuiResources) {
        self.layout();
        let batcher = self.batcher.take().unwrap_or_else(|| ImmediateBatcher::new(context));
        let mut renderer = GuiRenderer {
            theme: self.theme.clone(),
            resources,
            batcher,
            context,
            pass,
            scroll: Vec::new(),
        };
        Self::render_node(self.root, &mut self.nodes, &self.children, &mut renderer);
        for popup in self.popups.iter() {
            Self::render_node(*popup, &mut self.nodes, &self.children, &mut renderer);
        }
        renderer.finish();
        self.batcher = Some(renderer.batcher);
    }

    fn dispatch_input_event(
        id: NodeId,
        nodes: &mut SlotMap<NodeId, Node>,
        children: &SecondaryMap<NodeId, Vec<NodeId>>,
        input: &mut GuiInput,
        grabbed_node: &mut Option<NodeId>,
        executor: &mut EventExecutor,
    ) {
        if nodes.get(id).unwrap().area.hidden {
            return;
        }
        if let Some(node_children) = children.get(id) {
            for child in node_children.iter().rev() {
                Self::dispatch_input_event(*child, nodes, children, input, grabbed_node, executor);
            }
        }
        let node = nodes.get_mut(id).unwrap();
        if let Some(widget) = node.widget.as_mut() {
            executor.current_node = id;
            match widget.input(input, executor, &node.area) {
                InputAction::Pass => {}
                InputAction::Block => {
                    input.blocked = true;
                }
                InputAction::Grab => {
                    input.blocked = true;
                    *grabbed_node = Some(id);
                }
            }
        } else if node.style.background_color.is_some() && node.area.background_rect.contains(input.pointer) {
            input.blocked = true;
        }
    }
    pub fn handle_input<K: KeyboardEvent, M: MouseButtonEvent>(
        &mut self,
        event: InputEvent<K, M>,
    ) -> (EventExecutor, Option<InputEvent<K, M>>) {
        self.input.process(&event);
        let mut executor = EventExecutor::new();
        if let Some(id) = self.grabbed_node.take() {
            self.input.grabbed = true;
            Self::dispatch_input_event(
                id,
                &mut self.nodes,
                &self.children,
                &mut self.input,
                &mut self.grabbed_node,
                &mut executor,
            );
        } else {
            Self::dispatch_input_event(
                self.root,
                &mut self.nodes,
                &self.children,
                &mut self.input,
                &mut self.grabbed_node,
                &mut executor,
            );
        }
        let unhandled_event = if self.input.blocked { None } else { Some(event) };
        self.input.reset();
        (executor, unhandled_event)
    }
}
impl EventContainer for Gui {
    fn get_by_type(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
        if type_id == TypeId::of::<Gui>() {
            Some(self)
        } else {
            None
        }
    }
}
