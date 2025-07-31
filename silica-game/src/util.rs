use euclid::{Point2D, Size2D};
use serde::{Deserialize, Serialize};

pub struct Percent(pub f32);

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", (self.0 * 100.0) as i32)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Timer {
    time: f32,
    max: f32,
}

impl Timer {
    pub fn new(time: f32) -> Self {
        Timer {
            time: 0.0,
            max: time,
        }
    }
    pub fn reset(&mut self) {
        self.time -= self.max;
    }
    pub fn update(&mut self, dt: f32) -> bool {
        self.time += dt;
        self.time >= self.max
    }
    pub fn progress(&self) -> f32 {
        (self.time / self.max).min(1.0)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Grid<T> {
    size: Size2D<usize, T>,
    data: Vec<T>,
}

impl<T: Clone> Grid<T> {
    pub fn new(size: Size2D<usize, T>, value: T) -> Self {
        let mut data = Vec::new();
        data.resize(size.area(), value);
        Grid { size, data }
    }
}
impl<T> Grid<T> {
    pub fn size(&self) -> Size2D<usize, T> {
        self.size
    }
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.data.iter_mut()
    }
}
impl<T> std::ops::Index<Point2D<usize, T>> for Grid<T> {
    type Output = T;
    fn index(&self, index: Point2D<usize, T>) -> &Self::Output {
        &self.data[index.x + (index.y * self.size.width)]
    }
}
impl<T> std::ops::IndexMut<Point2D<usize, T>> for Grid<T> {
    fn index_mut(&mut self, index: Point2D<usize, T>) -> &mut Self::Output {
        &mut self.data[index.x + (index.y * self.size.width)]
    }
}
