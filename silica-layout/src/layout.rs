use crate::*;

pub trait LayoutProvider {
    fn measure<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        available_space: Size,
    ) -> Size;
    fn layout<Id: Key, Widget: LayoutWidget>(
        nodes: &mut SlotMap<Id, Node<Id, Widget>>,
        children: &SecondaryMap<Id, Vec<Id>>,
        id: Id,
        rect: Rect,
    );
}

pub struct BoxLayout;

impl LayoutProvider for BoxLayout {
    fn measure<Id: Key, Widget: LayoutWidget>(
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
    fn layout<Id: Key, Widget: LayoutWidget>(
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
        let align = style.align;
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
        let grow_space = if grow_count > 0 {
            if direction.horizontal() {
                Size::new(
                    (rect.size.width - used_size.width + gap).max(0) / grow_count,
                    0,
                )
            } else {
                Size::new(
                    0,
                    (rect.size.height - used_size.height + gap).max(0) / grow_count,
                )
            }
        } else {
            Size::zero()
        };
        for child_id in child_ids.iter() {
            let child = &nodes[*child_id];
            let mut child_size = child.area.measured_size;
            if child.style.grow {
                child_size += grow_space;
            }
            let mut child_rect = direction.layout_area(&mut rect, child_size, gap);
            child_rect = align.align_area(direction, child_rect, child_size);
            layout(nodes, children, *child_id, child_rect);
        }
    }
}
