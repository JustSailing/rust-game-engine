use super::consts::FLOAT_EPSILON;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    values: [f32; 2],
}

impl Vec2 {
    pub fn new(first: f32, second: f32) -> Self {
        Self {
            values: [first, second],
        }
    }
    pub fn new_ones() -> Self {
        Self { values: [1.0, 1.0] }
    }
    pub fn new_right() -> Self {
        Self { values: [1.0, 0.0] }
    }

    pub fn new_left() -> Self {
        Self {
            values: [-1.0, 0.0],
        }
    }

    pub fn new_down() -> Self {
        Self {
            values: [0.0, -1.0],
        }
    }

    pub fn new_up() -> Self {
        Self { values: [0.0, 1.0] }
    }

    pub fn length_squared(&self) -> f32 {
        self.values[0] * self.values[0] + self.values[1] * self.values[1]
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        Vec2::new(
            self.values[0] - other.values[1],
            self.values[1] - other.values[1],
        )
        .length()
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        self.values[0] /= length;
        self.values[1] /= length;
    }

    pub fn set_x(&mut self, x: f32) {
        self.values[0] = x;
    }
    pub fn set_y(&mut self, y: f32) {
        self.values[1] = y;
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self { values: [0.0, 0.0] }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] + rhs.values[0],
                self.values[1] + rhs.values[1],
            ],
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] - rhs.values[0],
                self.values[1] - rhs.values[1],
            ],
        }
    }
}

impl Mul for Vec2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] * rhs.values[0],
                self.values[1] * rhs.values[1],
            ],
        }
    }
}

impl Div for Vec2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] / rhs.values[0],
                self.values[1] / rhs.values[1],
            ],
        }
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        if (self.values[0] - other.values[0]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.values[1] - other.values[1]).abs() > FLOAT_EPSILON {
            return false;
        }
        return true;
    }
}
