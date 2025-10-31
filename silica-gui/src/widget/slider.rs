use std::{cell::Cell, rc::Rc};

use euclid::Vector2D;

use crate::{render::GuiRenderer, *};

pub struct Slider {
    vertical: bool,
    value: f32,
    scroll_size: Option<Rc<Cell<Size>>>,
    state: ButtonState,
    on_changed: EventFn,
}

impl Slider {
    const MIN_SIZE: Size = Size::new(32, 32);
    fn scrollbar_style() -> Style {
        Style {
            background_color: Some(Color::Gutter),
            min_size: Self::MIN_SIZE,
            ..Default::default()
        }
    }
    pub fn new<C, F>(vertical: bool, on_changed: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C, f32) + 'static,
    {
        Slider {
            vertical,
            value: 0.0,
            scroll_size: None,
            state: ButtonState::Normal,
            on_changed: EventFn::new_param(on_changed),
        }
    }
    pub fn new_scrollbar<C, F>(vertical: bool, scroll_size: Option<Rc<Cell<Size>>>, on_changed: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C, f32) + 'static,
    {
        Slider {
            vertical,
            value: 0.0,
            scroll_size,
            state: ButtonState::Normal,
            on_changed: EventFn::new_param(on_changed),
        }
    }
    fn handle_size(&self, area: &Area) -> i32 {
        if self.vertical {
            let scroll_size = self
                .scroll_size
                .as_ref()
                .map(|size| (area.content_rect.size.height as f32) / (size.get().height as f32).max(1.0))
                .unwrap_or_default()
                .min(1.0);
            ((scroll_size * (area.content_rect.size.height as f32)) as i32).max(32)
        } else {
            let scroll_size = self
                .scroll_size
                .as_ref()
                .map(|size| (area.content_rect.size.width as f32) / (size.get().width as f32).max(1.0))
                .unwrap_or_default()
                .min(1.0);
            ((scroll_size * (area.content_rect.size.width as f32)) as i32).max(32)
        }
    }
}
impl Widget for Slider {
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, area: &Area) -> InputAction {
        let state_input = self.state.handle_input(input, None, area.content_rect);
        if state_input.changed {
            executor.request_redraw();
        }
        if self.state == ButtonState::Press {
            let handle_size = self.handle_size(area);
            self.value = if self.vertical {
                ((input.pointer.y - area.content_rect.origin.y - (handle_size / 2)) as f32)
                    / ((area.content_rect.size.height - handle_size) as f32)
            } else {
                ((input.pointer.x - area.content_rect.origin.x - (handle_size / 2)) as f32)
                    / ((area.content_rect.size.width - handle_size) as f32)
            };
            self.value = self.value.clamp(0.0, 1.0);
            executor.queue(self.on_changed.clone(), Some(Box::new(self.value)));
            executor.request_redraw();
            InputAction::Grab
        } else {
            state_input.action
        }
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        let handle_size = self.handle_size(area);
        let handle_rect = if self.vertical {
            let handle_pos = area.content_rect.origin.y
                + (self.value * ((area.content_rect.size.height - handle_size) as f32)) as i32;
            Rect::new(
                Point::new(area.content_rect.origin.x, handle_pos),
                Size::new(area.content_rect.size.width, handle_size),
            )
        } else {
            let handle_pos = area.content_rect.origin.x
                + (self.value * ((area.content_rect.size.width - handle_size) as f32)) as i32;
            Rect::new(
                Point::new(handle_pos, area.content_rect.origin.y),
                Size::new(handle_size, area.content_rect.size.height),
            )
        };
        renderer
            .theme()
            .draw_button(renderer, handle_rect, ButtonStyle::Normal, false, self.state);
    }
}

pub struct ScrollArea {
    size: Option<Rc<Cell<Size>>>,
    scroll: Vector2D<f32, Pixel>,
}

impl ScrollArea {
    pub fn new(scroll_size: Option<Rc<Cell<Size>>>) -> Self {
        ScrollArea {
            size: scroll_size,
            scroll: Vector2D::zero(),
        }
    }
    pub fn scroll(&self) -> Vector2D<f32, Pixel> {
        self.scroll
    }
    pub fn set_scroll(&mut self, scroll: f32, vertical: bool) {
        if vertical {
            self.scroll.y = scroll;
        } else {
            self.scroll.x = scroll;
        }
    }
}
impl Widget for ScrollArea {
    fn layout(&mut self, area: &Area) {
        if let Some(size) = self.size.as_ref() {
            size.set(area.children_size);
        }
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        renderer.push_scroll_area(
            area.content_rect,
            self.scroll
                .component_mul((area.content_rect.size.to_vector() - area.children_size.to_vector()).to_f32())
                .to_i32(),
        );
    }
}
impl WidgetId<ScrollArea> {
    pub fn scroll(&self, gui: &Gui) -> Vector2D<f32, Pixel> {
        gui.get_widget(*self).map(|button| button.scroll()).unwrap_or_default()
    }
    pub fn set_scroll(&self, gui: &mut Gui, scroll: f32, vertical: bool) {
        if let Some(button) = gui.get_widget_mut(*self) {
            button.set_scroll(scroll, vertical);
        }
    }
}

#[must_use]
pub struct ScrollAreaBuilder {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    size: Rc<Cell<Size>>,
    area: WidgetId<ScrollArea>,
    horizontal_scrollbar: Option<WidgetId<Slider>>,
    vertical_scrollbar: Option<WidgetId<Slider>>,
}

impl ScrollAreaBuilder {
    pub fn new(gui: &mut Gui, style: Style) -> Self {
        let size = Rc::new(Cell::new(Size::zero()));
        let area = gui.create_widget(style, ScrollArea::new(Some(size.clone())));
        ScrollAreaBuilder {
            parent: None,
            children: Vec::new(),
            size,
            area,
            horizontal_scrollbar: None,
            vertical_scrollbar: None,
        }
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
    pub fn horizontal_scroll(mut self, gui: &mut Gui) -> Self {
        let area = self.area;
        let scrollbar = Slider::new_scrollbar(false, Some(self.size.clone()), move |gui, value| {
            area.set_scroll(gui, value, false);
        });
        self.horizontal_scrollbar = Some(gui.create_widget(Slider::scrollbar_style(), scrollbar));
        self
    }
    pub fn vertical_scroll(mut self, gui: &mut Gui) -> Self {
        let area = self.area;
        let scrollbar = Slider::new_scrollbar(true, Some(self.size.clone()), move |gui, value| {
            area.set_scroll(gui, value, true);
        });
        self.vertical_scrollbar = Some(gui.create_widget(Slider::scrollbar_style(), scrollbar));
        self
    }
    pub fn build(self, gui: &mut Gui) -> NodeId {
        assert!(
            self.horizontal_scrollbar.is_some() || self.vertical_scrollbar.is_some(),
            "no scrollbars"
        );
        gui.modify_style(self.area, |style| {
            style.overflow.x = self.horizontal_scrollbar.is_some();
            style.overflow.y = self.vertical_scrollbar.is_some();
        });
        let container = if let Some(horizontal_scrollbar) = self.horizontal_scrollbar {
            let container = gui.create_node(Style {
                direction: Direction::ColumnReverse,
                border: SideOffsets::new_all_same(1),
                ..Default::default()
            });
            gui.add_child(container, horizontal_scrollbar);
            gui.add_child(container, self.area);
            container
        } else if let Some(vertical_scrollbar) = self.vertical_scrollbar {
            let container = gui.create_node(Style {
                direction: Direction::RowReverse,
                border: SideOffsets::new_all_same(1),
                ..Default::default()
            });
            gui.add_child(container, vertical_scrollbar);
            gui.add_child(container, self.area);
            container
        } else {
            todo!()
        };
        gui.set_node_children(self.area, self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, container);
        }
        container
    }
}
