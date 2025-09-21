use super::consts::FLOAT_EPSILON;
use std::{
    ops::{Add, Div, Mul, Sub},
    vec,
};

use super::vec3::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Vec4 {
    pub data: [f32; 4],
}

impl Vec4 {
    pub fn new(first: f32, second: f32, third: f32, fourth: f32) -> Self {
        Self {
            data: [first, second, third, fourth],
        }
    }
    pub fn new_ones() -> Self {
        Self {
            data: [1.0, 1.0, 1.0, 1.0],
        }
    }
    pub fn new_right() -> Self {
        Self {
            data: [1.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn new_left() -> Self {
        Self {
            data: [-1.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn new_down() -> Self {
        Self {
            data: [0.0, -1.0, 0.0, 0.0],
        }
    }

    pub fn new_up() -> Self {
        Self {
            data: [0.0, 1.0, 0.0, 0.0],
        }
    }

    pub fn new_forward() -> Self {
        Self {
            data: [0.0, 0.0, -1.0, 0.0],
        }
    }

    pub fn new_back() -> Self {
        Self {
            data: [0.0, 0.0, 1.0, 0.0],
        }
    }

    pub fn vec3_to_vec4(vec3: &Vec3, w: f32) -> Self {
        Self {
            data: [vec3.data[0], vec3.data[1], vec3.data[2], w],
        }
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3 {
            data: [self.data[0], self.data[1], self.data[2]],
        }
    }

    pub fn mul_scalar(&self, scalar: f32) -> Self {
        Self {
            data: [
                self.data[0] * scalar,
                self.data[1] * scalar,
                self.data[2] * scalar,
                self.data[3] * scalar,
            ],
        }
    }

    pub fn length_squared(&self) -> f32 {
        self.data[0] * self.data[0]
            + self.data[1] * self.data[1]
            + self.data[2] * self.data[2]
            + self.data[3] * self.data[3]
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        Vec4::new(
            self.data[0] - other.data[1],
            self.data[1] - other.data[1],
            self.data[2] - other.data[2],
            self.data[3] - other.data[3],
        )
        .length()
    }

    pub fn dot(&self, other: &Self) -> f32 {
        let mut p: f32 = 0.0;
        p += self.data[0] * other.data[0];
        p += self.data[1] * other.data[1];
        p += self.data[2] * other.data[2];
        p += self.data[3] * other.data[3];
        p
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        self.data[0] /= length;
        self.data[1] /= length;
        self.data[2] /= length;
        self.data[3] /= length;
    }

    pub fn set_x(&mut self, x: f32) {
        self.data[0] = x;
    }
    pub fn set_y(&mut self, y: f32) {
        self.data[1] = y;
    }
    pub fn set_z(&mut self, z: f32) {
        self.data[2] = z;
    }
    pub fn set_w(&mut self, w: f32) {
        self.data[3] = w;
    }
}

impl Default for Vec4 {
    fn default() -> Self {
        Self {
            data: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl Add for Vec4 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] + rhs.data[0],
                self.data[1] + rhs.data[1],
                self.data[2] + rhs.data[2],
                self.data[3] + rhs.data[3],
            ],
        }
    }
}

impl Sub for Vec4 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] - rhs.data[0],
                self.data[1] - rhs.data[1],
                self.data[2] - rhs.data[2],
                self.data[3] - rhs.data[3],
            ],
        }
    }
}

impl Mul for Vec4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] * rhs.data[0],
                self.data[1] * rhs.data[1],
                self.data[2] * rhs.data[2],
                self.data[3] * rhs.data[3],
            ],
        }
    }
}

impl Div for Vec4 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] / rhs.data[0],
                self.data[1] / rhs.data[1],
                self.data[2] / rhs.data[2],
                self.data[3] / rhs.data[3],
            ],
        }
    }
}

impl PartialEq for Vec4 {
    fn eq(&self, other: &Self) -> bool {
        if (self.data[0] - other.data[0]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.data[1] - other.data[1]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.data[2] - other.data[2]).abs() > FLOAT_EPSILON {
            return false;
        }
        if (self.data[3] - other.data[3]).abs() > FLOAT_EPSILON {
            return false;
        }
        return true;
    }
}
