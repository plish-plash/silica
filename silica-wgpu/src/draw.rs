use euclid::{Box2D, SideOffsets2D, point2, vec2};
use silica_color::Rgba;

use crate::{Texture, TextureRect, TextureSize, Uv, UvRect};

pub trait RectExt<T, U> {
    fn top_left(&self) -> euclid::Point2D<T, U>;
    fn top_right(&self) -> euclid::Point2D<T, U>;
    fn bottom_left(&self) -> euclid::Point2D<T, U>;
    fn bottom_right(&self) -> euclid::Point2D<T, U>;
}

impl<T, U> RectExt<T, U> for euclid::Box2D<T, U>
where
    T: Copy,
{
    fn top_left(&self) -> euclid::Point2D<T, U> {
        self.min
    }
    fn top_right(&self) -> euclid::Point2D<T, U> {
        point2(self.max.x, self.min.y)
    }
    fn bottom_left(&self) -> euclid::Point2D<T, U> {
        point2(self.min.x, self.max.y)
    }
    fn bottom_right(&self) -> euclid::Point2D<T, U> {
        self.max
    }
}

pub trait DrawQuad<T, U> {
    fn draw_quad(&mut self, rect: Box2D<T, U>, uv: UvRect, color: Rgba);
}

pub fn draw_border<U>(
    drawer: &mut impl DrawQuad<i32, U>,
    mut rect: Box2D<i32, U>,
    border: SideOffsets2D<i32, U>,
    color: Rgba,
) {
    rect.max -= vec2(1, 1);
    if rect.is_empty() {
        return;
    }
    let tl = rect.top_left();
    let tr = rect.top_right();
    let bl = rect.bottom_left();
    let br = rect.bottom_right();
    if border.top > 0 {
        drawer.draw_quad(Box2D::new(tl, tr + vec2(0, border.top)), Uv::ZERO, color);
    }
    if border.bottom > 0 {
        drawer.draw_quad(Box2D::new(bl - vec2(0, border.bottom), br), Uv::ZERO, color);
    }
    if border.left > 0 {
        drawer.draw_quad(Box2D::new(tl, bl + vec2(border.left, 0)), Uv::ZERO, color);
    }
    if border.right > 0 {
        drawer.draw_quad(Box2D::new(tr - vec2(border.right, 0), br), Uv::ZERO, color);
    }
}

pub struct NineSlice<U> {
    uv_outer: UvRect,
    uv_inner: UvRect,
    insets: SideOffsets2D<i32, U>,
}

impl<U> NineSlice<U> {
    pub fn new(
        texture_size: TextureSize,
        rect: TextureRect,
        insets: SideOffsets2D<u32, Texture>,
    ) -> Self {
        let uv_outer = Uv::normalize(rect, texture_size);
        let uv_inner = Uv::normalize(rect.inner_box(insets), texture_size);
        NineSlice {
            uv_outer,
            uv_inner,
            insets: SideOffsets2D::new(
                insets.top as i32,
                insets.right as i32,
                insets.bottom as i32,
                insets.left as i32,
            ),
        }
    }
    pub fn draw(&self, drawer: &mut impl DrawQuad<i32, U>, rect: Box2D<i32, U>, color: Rgba) {
        let rect_center = rect.inner_box(self.insets);
        drawer.draw_quad(
            Box2D::new(rect.min, rect_center.min),
            UvRect::new(self.uv_outer.min, self.uv_inner.min),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect_center.min.x, rect.min.y),
                rect_center.top_right(),
            ),
            UvRect::new(
                point2(self.uv_inner.min.x, self.uv_outer.min.y),
                self.uv_inner.top_right(),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect_center.max.x, rect.min.y),
                point2(rect.max.x, rect_center.min.y),
            ),
            UvRect::new(
                point2(self.uv_inner.max.x, self.uv_outer.min.y),
                point2(self.uv_outer.max.x, self.uv_inner.min.y),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect.min.x, rect_center.min.y),
                rect_center.bottom_left(),
            ),
            UvRect::new(
                point2(self.uv_outer.min.x, self.uv_inner.min.y),
                self.uv_inner.bottom_left(),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(rect_center.min, rect_center.max),
            UvRect::new(self.uv_inner.min, self.uv_inner.max),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                rect_center.top_right(),
                point2(rect.max.x, rect_center.max.y),
            ),
            UvRect::new(
                self.uv_inner.top_right(),
                point2(self.uv_outer.max.x, self.uv_inner.max.y),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect.min.x, rect_center.max.y),
                point2(rect_center.min.x, rect.max.y),
            ),
            UvRect::new(
                point2(self.uv_outer.min.x, self.uv_inner.max.y),
                point2(self.uv_inner.min.x, self.uv_outer.max.y),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                rect_center.bottom_left(),
                point2(rect_center.max.x, rect.max.y),
            ),
            UvRect::new(
                self.uv_inner.bottom_left(),
                point2(self.uv_inner.max.x, self.uv_outer.max.y),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(rect_center.max, rect.max),
            UvRect::new(self.uv_inner.max, self.uv_outer.max),
            color,
        );
    }
    pub fn draw_top(&self, drawer: &mut impl DrawQuad<i32, U>, rect: Box2D<i32, U>, color: Rgba) {
        let rect_center = rect.inner_box(self.insets);
        drawer.draw_quad(
            Box2D::new(rect.min, rect_center.min),
            UvRect::new(self.uv_outer.min, self.uv_inner.min),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect_center.min.x, rect.min.y),
                rect_center.top_right(),
            ),
            UvRect::new(
                point2(self.uv_inner.min.x, self.uv_outer.min.y),
                self.uv_inner.top_right(),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect_center.max.x, rect.min.y),
                point2(rect.max.x, rect_center.min.y),
            ),
            UvRect::new(
                point2(self.uv_inner.max.x, self.uv_outer.min.y),
                point2(self.uv_outer.max.x, self.uv_inner.min.y),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(
                point2(rect.min.x, rect_center.min.y),
                point2(rect_center.min.x, rect.max.y),
            ),
            UvRect::new(
                point2(self.uv_outer.min.x, self.uv_inner.min.y),
                self.uv_inner.bottom_left(),
            ),
            color,
        );
        drawer.draw_quad(
            Box2D::new(rect_center.min, point2(rect_center.max.x, rect.max.y)),
            UvRect::new(self.uv_inner.min, self.uv_inner.max),
            color,
        );
        drawer.draw_quad(
            Box2D::new(rect_center.top_right(), rect.max),
            UvRect::new(
                self.uv_inner.top_right(),
                point2(self.uv_outer.max.x, self.uv_inner.max.y),
            ),
            color,
        );
    }
}
