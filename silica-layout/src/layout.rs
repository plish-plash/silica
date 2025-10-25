use crate::*;

pub struct BoxLayout;

impl BoxLayout {
    pub fn measure<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        mut available_space: Size,
    ) -> Size {
        let child_ids = if let Some(child_ids) = children.get(id) {
            child_ids
        } else {
            return Size::zero();
        };
        let style = &nodes[id].style;
        let direction = style.direction;
        let gap = style.gap;
        let mut size = Size::zero();
        for child_id in child_ids.iter() {
            let child_size = measure(nodes, children, *child_id, available_space);
            if direction.horizontal() {
                available_space.width -= child_size.width + gap;
                if size.width > 0 {
                    size.width += gap;
                }
                size.width += child_size.width;
                size.height = size.height.max(child_size.height);
            } else {
                available_space.height -= child_size.height + gap;
                size.width = size.width.max(child_size.width);
                if size.height > 0 {
                    size.height += gap;
                }
                size.height += child_size.height;
            }
        }
        size
    }
    pub fn layout<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        mut rect: Rect,
    ) {
        let child_ids = if let Some(child_ids) = children.get(id) {
            child_ids
        } else {
            return;
        };
        let style = &nodes[id].style;
        let direction = style.direction;
        let main_align = style.main_align;
        let cross_align = style.cross_align;
        let gap = style.gap;
        let mut used_size = Size::zero();
        let mut grow_count = 0;
        for child_id in child_ids.iter() {
            let child = &nodes[*child_id];
            if direction.horizontal() {
                used_size.width += child.area.measured_size.width + gap;
            } else {
                used_size.height += child.area.measured_size.height + gap;
            }
            if child.style.grow {
                grow_count += 1;
            }
        }
        let unused_size = if direction.horizontal() {
            Size::new((rect.size.width - used_size.width + gap).max(0), 0)
        } else {
            Size::new(0, (rect.size.height - used_size.height + gap).max(0))
        };
        let grow_space = if grow_count > 0 {
            unused_size / grow_count
        } else {
            match main_align {
                Align::End => {
                    direction.layout_area(&mut rect, unused_size, 0);
                }
                Align::Center => {
                    direction.layout_area(&mut rect, unused_size / 2, 0);
                }
                _ => {}
            }
            Size::zero()
        };
        for child_id in child_ids.iter() {
            let child = &nodes[*child_id];
            let mut child_size = child.area.measured_size;
            if child.style.grow {
                child_size += grow_space;
            }
            let mut child_rect = direction.layout_area(&mut rect, child_size, gap);
            child_rect = cross_align.align_area(!direction.horizontal(), child_rect, child_size);
            layout(nodes, children, *child_id, child_rect);
        }
    }
}

pub struct StackLayout;

impl StackLayout {
    pub fn measure<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        available_space: Size,
    ) -> Size {
        let mut size = Size::zero();
        if let Some(child_ids) = children.get(id) {
            for child_id in child_ids.iter() {
                let child_size = measure(nodes, children, *child_id, available_space);
                size = size.max(child_size);
            }
        }
        size
    }
    pub fn layout<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        rect: Rect,
    ) {
        let style = &nodes[id].style;
        let direction = style.direction;
        let main_align = style.main_align;
        if let Some(child_ids) = children.get(id) {
            for child_id in child_ids.iter() {
                let child = &nodes[*child_id];
                let child_size = child.area.measured_size;
                let grow_align = if child.style.grow { Align::Stretch } else { main_align };
                let mut child_rect = grow_align.align_area(direction.horizontal(), rect, child_size);
                child_rect = child
                    .style
                    .cross_align
                    .align_area(!direction.horizontal(), child_rect, child_size);
                layout(nodes, children, *child_id, child_rect);
            }
        }
    }
}

pub struct GridLayout;

impl GridLayout {
    pub fn measure<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        mut available_space: Size,
        columns: usize,
    ) -> Size {
        let child_ids = if let Some(child_ids) = children.get(id) {
            child_ids
        } else {
            return Size::zero();
        };
        let style = &nodes[id].style;
        let direction = style.direction;
        let gap = style.gap;
        let mut size = Size::zero();
        for column in 0..columns {
            let mut child_size = Size::zero();
            for i in (column..child_ids.len()).step_by(columns) {
                child_size = child_size.max(measure(nodes, children, child_ids[i], available_space));
            }
            for i in (column..child_ids.len()).step_by(columns) {
                nodes[child_ids[i]].area.measured_size = child_size;
            }
            if direction.horizontal() {
                available_space.width -= child_size.width + gap;
                if size.width > 0 {
                    size.width += gap;
                }
                size.width += child_size.width;
                size.height = size.height.max(child_size.height);
            } else {
                available_space.height -= child_size.height + gap;
                size.width = size.width.max(child_size.width);
                if size.height > 0 {
                    size.height += gap;
                }
                size.height += child_size.height;
            }
        }
        let rows = child_ids.len().div_ceil(columns) as i32;
        if rows > 0 {
            if direction.horizontal() {
                size.height = (size.height * rows) + (gap * (rows - 1));
            } else {
                size.width = (size.width * rows) + (gap * (rows - 1));
            }
        }
        size
    }
    pub fn layout<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        mut rect: Rect,
        columns: usize,
    ) {
        let child_ids = if let Some(child_ids) = children.get(id) {
            child_ids
        } else {
            return;
        };
        let rows = child_ids.len().div_ceil(columns) as i32;
        let style = &nodes[id].style;
        let direction = style.direction;
        let main_align = style.main_align;
        let gap = style.gap;
        let first_child_size = child_ids
            .first()
            .map(|id| nodes[*id].area.measured_size)
            .unwrap_or_default();
        let row_size = if direction.horizontal() {
            let row_size = first_child_size.height;
            let unused_size = rect.size.height - ((row_size * rows) + (gap * (rows - 1)));
            match style.cross_align {
                Align::End => {
                    rect.origin.y += unused_size;
                    rect.size.height -= unused_size;
                }
                Align::Center => {
                    rect.origin.y += unused_size / 2;
                    rect.size.height -= unused_size / 2;
                }
                _ => {}
            }
            row_size
        } else {
            let row_size = first_child_size.width;
            let unused_size = rect.size.width - ((row_size * rows) + (gap * (rows - 1)));
            match style.cross_align {
                Align::End => {
                    rect.origin.x += unused_size;
                    rect.size.width -= unused_size;
                }
                Align::Center => {
                    rect.origin.x += unused_size / 2;
                    rect.size.width -= unused_size / 2;
                }
                _ => {}
            }
            row_size
        };
        let row_ids = &child_ids[0..columns.min(child_ids.len())];
        let mut used_size = Size::zero();
        let mut grow_count = 0;
        for child_id in row_ids.iter() {
            let child = &nodes[*child_id];
            if direction.horizontal() {
                used_size.width += child.area.measured_size.width + gap;
            } else {
                used_size.height += child.area.measured_size.height + gap;
            }
            if child.style.grow {
                grow_count += 1;
            }
        }
        let unused_size = if direction.horizontal() {
            Size::new((rect.size.width - used_size.width + gap).max(0), 0)
        } else {
            Size::new(0, (rect.size.height - used_size.height + gap).max(0))
        };
        let grow_space = if grow_count > 0 {
            unused_size / grow_count
        } else {
            match main_align {
                Align::End => {
                    direction.layout_area(&mut rect, unused_size, 0);
                }
                Align::Center => {
                    direction.layout_area(&mut rect, unused_size / 2, 0);
                }
                _ => {}
            }
            Size::zero()
        };
        for (row_index, child_id) in row_ids.iter().enumerate() {
            let child = &nodes[*child_id];
            let mut child_size = child.area.measured_size;
            if child.style.grow {
                child_size += grow_space;
            }
            let mut child_rect = direction.layout_area(&mut rect, child_size, gap);
            if direction.horizontal() {
                child_rect.size.height = row_size;
            } else {
                child_rect.size.width = row_size;
            }
            for i in (row_index..child_ids.len()).step_by(columns) {
                layout(nodes, children, child_ids[i], child_rect);
                if direction.horizontal() {
                    child_rect.origin.y += row_size + gap;
                } else {
                    child_rect.origin.x += row_size + gap;
                }
            }
        }
    }
}
