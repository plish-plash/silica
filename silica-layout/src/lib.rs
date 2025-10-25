mod layout;

use std::marker::PhantomData;

use euclid::{point2, size2};
use silica_color::Rgba;
use slotmap::{Key, SecondaryMap, SlotMap};

use crate::layout::*;

#[derive(Clone, Copy)]
pub struct Pixel;

pub type Point = euclid::Point2D<i32, Pixel>;
pub type Vector = euclid::Vector2D<i32, Pixel>;
pub type Size = euclid::Size2D<i32, Pixel>;
pub type Rect = euclid::Rect<i32, Pixel>;
pub type SideOffsets = euclid::SideOffsets2D<i32, Pixel>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Background,
    Border,
    Accent,
    Foreground,
    Custom(Rgba),
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    None,
    #[default]
    Box,
    Stack,
    Grid(usize),
}

impl Layout {
    fn measure<Id: Key, Widget: LayoutWidget>(
        self,
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        available_space: Size,
    ) -> Size {
        match self {
            Layout::None => Size::zero(),
            Layout::Box => BoxLayout::measure(nodes, children, id, available_space),
            Layout::Stack => StackLayout::measure(nodes, children, id, available_space),
            Layout::Grid(columns) => GridLayout::measure(nodes, children, id, available_space, columns),
        }
    }
    fn layout<Id: Key, Widget: LayoutWidget>(
        self,
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        rect: Rect,
    ) {
        match self {
            Layout::None => (),
            Layout::Box => BoxLayout::layout(nodes, children, id, rect),
            Layout::Stack => StackLayout::layout(nodes, children, id, rect),
            Layout::Grid(columns) => GridLayout::layout(nodes, children, id, rect, columns),
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl Direction {
    fn horizontal(&self) -> bool {
        *self == Direction::Row || *self == Direction::RowReverse
    }
    fn layout_area(&self, rect: &mut Rect, size: Size, gap: i32) -> Rect {
        match self {
            Direction::Row => {
                let area = Rect::new(rect.origin, size2(size.width, rect.height()));
                rect.origin.x += size.width + gap;
                rect.size.width -= size.width + gap;
                area
            }
            Direction::Column => {
                let area = Rect::new(rect.origin, size2(rect.width(), size.height));
                rect.origin.y += size.height + gap;
                rect.size.height -= size.height + gap;
                area
            }
            Direction::RowReverse => {
                rect.size.width -= size.width + gap;
                Rect::new(
                    point2(rect.max_x() + gap, rect.min_y()),
                    size2(size.width, rect.height()),
                )
            }
            Direction::ColumnReverse => {
                rect.size.height -= size.height + gap;
                Rect::new(
                    point2(rect.min_x(), rect.max_y() + gap),
                    size2(rect.width(), size.height),
                )
            }
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    End,
    Center,
}

impl Align {
    fn align_area(&self, horizontal: bool, mut rect: Rect, size: Size) -> Rect {
        if *self != Align::Stretch {
            let (inner_size, outer_size) = if horizontal {
                (size.width, std::mem::replace(&mut rect.size.width, size.width))
            } else {
                (size.height, std::mem::replace(&mut rect.size.height, size.height))
            };
            let offset = match self {
                Align::End => outer_size - inner_size,
                Align::Center => (outer_size - inner_size) / 2,
                _ => 0,
            };
            if horizontal {
                rect.origin.x += offset;
            } else {
                rect.origin.y += offset;
            }
        }
        rect
    }
}

#[derive(Clone)]
pub struct Style {
    pub hidden: bool,
    pub background_color: Option<Color>,
    pub border_color: Option<Color>,

    pub min_size: Size,
    pub max_size: Size,
    pub grow: bool,

    pub layout: Layout,
    pub direction: Direction,
    pub main_align: Align,
    pub cross_align: Align,
    pub gap: i32,
    pub margin: SideOffsets,
    pub border: SideOffsets,
    pub padding: SideOffsets,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }
    fn box_offsets(&self) -> SideOffsets {
        self.margin + self.border + self.padding
    }
    fn box_size(&self) -> Size {
        let offsets = self.box_offsets();
        Size::new(offsets.horizontal(), offsets.vertical())
    }
    fn apply_min_max(&self, size: Size) -> Size {
        size.max(self.min_size).min(self.max_size)
    }
}
impl Default for Style {
    fn default() -> Self {
        Style {
            hidden: false,
            background_color: None,
            border_color: Some(Color::Border),
            min_size: Size::zero(),
            max_size: Size::new(i32::MAX, i32::MAX),
            grow: false,
            layout: Layout::default(),
            direction: Direction::default(),
            main_align: Align::default(),
            cross_align: Align::default(),
            gap: 0,
            margin: SideOffsets::zero(),
            border: SideOffsets::zero(),
            padding: SideOffsets::zero(),
        }
    }
}

#[derive(Default, Clone)]
pub struct Area {
    pub measured_size: Size,
    pub hidden: bool,
    pub content_rect: Rect,
    pub background_rect: Rect,
}

impl Area {
    pub fn new() -> Self {
        Self::default()
    }
}

pub trait LayoutWidget {
    fn measure(&mut self, available_space: Size) -> Size;
    fn layout(&mut self, area: &Area);
}

pub struct Node<Id, Widget> {
    pub style: Style,
    pub area: Area,
    pub widget: Option<Widget>,
    _id: PhantomData<Id>,
}

impl<Id, Widget> Node<Id, Widget> {
    pub fn new(style: Style, widget: Option<Widget>) -> Self {
        Node {
            style,
            area: Area::new(),
            widget,
            _id: PhantomData,
        }
    }
}
impl<Id, Widget> Default for Node<Id, Widget> {
    fn default() -> Self {
        Self {
            style: Style::default(),
            area: Area::new(),
            widget: None,
            _id: PhantomData,
        }
    }
}

pub fn measure<Id: Key, Widget: LayoutWidget>(
    nodes: &mut SlotMap<Id, Node<Id, Widget>>,
    children: &SecondaryMap<Id, Vec<Id>>,
    id: Id,
    mut available_space: Size,
) -> Size {
    let node = &nodes[id];
    let box_size = node.style.box_size();
    available_space = node.style.apply_min_max(available_space - box_size);
    let mut size = node.style.layout.measure(nodes, children, id, available_space);
    let node = &mut nodes[id];
    if let Some(widget) = node.widget.as_mut() {
        size = size.max(widget.measure(available_space));
    }
    size = node.style.apply_min_max(size) + box_size;
    node.area.measured_size = size;
    size
}
pub fn layout<Id: Key, Widget: LayoutWidget>(
    nodes: &mut SlotMap<Id, Node<Id, Widget>>,
    children: &SecondaryMap<Id, Vec<Id>>,
    id: Id,
    mut rect: Rect,
) {
    let node = &mut nodes[id];
    let box_offsets = node.style.box_offsets();
    if rect.width() <= box_offsets.horizontal() || rect.height() <= box_offsets.vertical() {
        node.area.hidden = true;
        return;
    }
    rect = rect.inner_rect(box_offsets);
    node.area.hidden = node.style.hidden || node.style.layout == Layout::None;
    node.area.content_rect = rect;
    node.area.background_rect = rect.outer_rect(node.style.padding);
    if node.area.hidden {
        return;
    }
    if let Some(widget) = node.widget.as_mut() {
        widget.layout(&node.area);
    }
    node.style.layout.layout(nodes, children, id, rect);
}
pub fn measure_and_layout<Id: Key, Widget: LayoutWidget>(
    nodes: &mut SlotMap<Id, Node<Id, Widget>>,
    children: &SecondaryMap<Id, Vec<Id>>,
    id: Id,
    rect: Rect,
) {
    measure(nodes, children, id, rect.size);
    layout(nodes, children, id, rect);
}
