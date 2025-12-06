use super::consts::FLOAT_EPSILON;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vec2 {
    pub data: [f32; 2],
}

impl Vec2 {
    pub fn new(first: f32, second: f32) -> Self {
        Self {
            data: [first, second],
        }
    }
    pub fn new_ones() -> Self {
        Self { data: [1.0, 1.0] }
    }

    pub fn new_zeroes() -> Self {
        Self { data: [0.0, 0.0] }
    }

    pub fn new_right() -> Self {
        Self { data: [1.0, 0.0] }
    }

    pub fn new_left() -> Self {
        Self { data: [-1.0, 0.0] }
    }

    pub fn new_down() -> Self {
        Self { data: [0.0, -1.0] }
    }

    pub fn new_up() -> Self {
        Self { data: [0.0, 1.0] }
    }

    pub fn length_squared(&self) -> f32 {
        self.data[0] * self.data[0] + self.data[1] * self.data[1]
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        Vec2::new(self.data[0] - other.data[1], self.data[1] - other.data[1]).length()
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        self.data[0] /= length;
        self.data[1] /= length;
    }

    pub fn set_x(&mut self, x: f32) {
        self.data[0] = x;
    }
    pub fn set_y(&mut self, y: f32) {
        self.data[1] = y;
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self { data: [0.0, 0.0] }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] + rhs.data[0], self.data[1] + rhs.data[1]],
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] - rhs.data[0], self.data[1] - rhs.data[1]],
        }
    }
}

impl Mul for Vec2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] * rhs.data[0], self.data[1] * rhs.data[1]],
        }
    }
}

impl Div for Vec2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            data: [self.data[0] / rhs.data[0], self.data[1] / rhs.data[1]],
        }
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        if (self.data[0] - other.data[0]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.data[1] - other.data[1]).abs() > FLOAT_EPSILON {
            return false;
        }
        true
    }
}

impl Eq for Vec2 {}

pub struct Extents2D {
    min: Vec2,
    max: Vec2,
}
