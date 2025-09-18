use super::consts::FLOAT_EPSILON;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    values: [f32; 3],
}

impl Vec3 {
    pub fn new(first: f32, second: f32, third: f32) -> Self {
        Self {
            values: [first, second, third],
        }
    }
    pub fn new_ones() -> Self {
        Self {
            values: [1.0, 1.0, 1.0],
        }
    }
    pub fn new_right() -> Self {
        Self {
            values: [1.0, 0.0, 0.0],
        }
    }

    pub fn new_left() -> Self {
        Self {
            values: [-1.0, 0.0, 0.0],
        }
    }

    pub fn new_down() -> Self {
        Self {
            values: [0.0, -1.0, 0.0],
        }
    }

    pub fn new_up() -> Self {
        Self {
            values: [0.0, 1.0, 0.0],
        }
    }

    pub fn new_forward() -> Self {
        Self {
            values: [0.0, 0.0, -1.0],
        }
    }

    pub fn new_back() -> Self {
        Self {
            values: [0.0, 0.0, 1.0],
        }
    }

    pub fn mul_scalar(&self, scalar: f32) -> Self {
        Self {
            values: [
                self.values[0] * scalar,
                self.values[1] * scalar,
                self.values[2] * scalar,
            ],
        }
    }

    pub fn length_squared(&self) -> f32 {
        self.values[0] * self.values[0]
            + self.values[1] * self.values[1]
            + self.values[2] * self.values[2]
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        Vec3::new(
            self.values[0] - other.values[1],
            self.values[1] - other.values[1],
            self.values[2] - other.values[2],
        )
        .length()
    }

    pub fn dot(&self, other: &Self) -> f32 {
        let mut p: f32 = 0.0;
        p += self.values[0] * other.values[0];
        p += self.values[1] * other.values[1];
        p += self.values[2] * other.values[2];
        p
    }

    pub fn cross(&self, other: &Self) -> Vec3 {
        Vec3 {
            values: [
                self.values[1] * other.values[2] - self.values[2] * other.values[1],
                self.values[2] * other.values[0] - self.values[0] * other.values[2],
                self.values[0] * other.values[1] - self.values[1] * other.values[0],
            ],
        }
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        self.values[0] /= length;
        self.values[1] /= length;
        self.values[2] /= length;
    }

    pub fn set_x(&mut self, x: f32) {
        self.values[0] = x;
    }
    pub fn set_y(&mut self, y: f32) {
        self.values[1] = y;
    }
    pub fn set_z(&mut self, z: f32) {
        self.values[2] = z;
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            values: [0.0, 0.0, 0.0],
        }
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] + rhs.values[0],
                self.values[1] + rhs.values[1],
                self.values[2] + rhs.values[2],
            ],
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] - rhs.values[0],
                self.values[1] - rhs.values[1],
                self.values[2] - rhs.values[2],
            ],
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] * rhs.values[0],
                self.values[1] * rhs.values[1],
                self.values[2] * rhs.values[2],
            ],
        }
    }
}

impl Div for Vec3 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            values: [
                self.values[0] / rhs.values[0],
                self.values[1] / rhs.values[1],
                self.values[2] / rhs.values[2],
            ],
        }
    }
}

impl PartialEq for Vec3 {
    fn eq(&self, other: &Self) -> bool {
        if (self.values[0] - other.values[0]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.values[1] - other.values[1]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.values[2] - other.values[2]).abs() > FLOAT_EPSILON {
            return false;
        }
        return true;
    }
}
