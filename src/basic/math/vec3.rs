use crate::application::basic::math::vec4::Vec4;

use super::{consts::FLOAT_EPSILON, vec2::Vec2};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vec3 {
    pub data: [f32; 3],
}

impl Vec3 {
    pub fn new(first: f32, second: f32, third: f32) -> Self {
        Self {
            data: [first, second, third],
        }
    }
    pub fn new_zeroes() -> Self {
        Self {
            data: [0.0, 0.0, 0.0],
        }
    }

    pub fn new_ones() -> Self {
        Self {
            data: [1.0, 1.0, 1.0],
        }
    }
    pub fn new_right() -> Self {
        Self {
            data: [1.0, 0.0, 0.0],
        }
    }

    pub fn new_left() -> Self {
        Self {
            data: [-1.0, 0.0, 0.0],
        }
    }

    pub fn new_down() -> Self {
        Self {
            data: [0.0, -1.0, 0.0],
        }
    }

    pub fn new_up() -> Self {
        Self {
            data: [0.0, 1.0, 0.0],
        }
    }

    pub fn new_forward() -> Self {
        Self {
            data: [0.0, 0.0, -1.0],
        }
    }

    pub fn new_back() -> Self {
        Self {
            data: [0.0, 0.0, 1.0],
        }
    }

    pub fn mul_scalar(&self, scalar: f32) -> Self {
        Self {
            data: [
                self.data[0] * scalar,
                self.data[1] * scalar,
                self.data[2] * scalar,
            ],
        }
    }

    pub fn length_squared(&self) -> f32 {
        self.data[0] * self.data[0] + self.data[1] * self.data[1] + self.data[2] * self.data[2]
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(&self, other: &Self) -> f32 {
        Vec3::new(
            self.data[0] - other.data[0],
            self.data[1] - other.data[1],
            self.data[2] - other.data[2],
        )
        .length()
    }

    pub fn dot(&self, other: &Self) -> f32 {
        let mut p: f32 = 0.0;
        p += self.data[0] * other.data[0];
        p += self.data[1] * other.data[1];
        p += self.data[2] * other.data[2];
        p
    }

    pub fn cross(&self, other: &Self) -> Vec3 {
        Vec3 {
            data: [
                self.data[1] * other.data[2] - self.data[2] * other.data[1],
                self.data[2] * other.data[0] - self.data[0] * other.data[2],
                self.data[0] * other.data[1] - self.data[1] * other.data[0],
            ],
        }
    }

    pub fn normalize(&mut self) {
        let length = self.length();
        self.data[0] /= length;
        self.data[1] /= length;
        self.data[2] /= length;
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
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            data: [0.0, 0.0, 0.0],
        }
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] + rhs.data[0],
                self.data[1] + rhs.data[1],
                self.data[2] + rhs.data[2],
            ],
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] - rhs.data[0],
                self.data[1] - rhs.data[1],
                self.data[2] - rhs.data[2],
            ],
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] * rhs.data[0],
                self.data[1] * rhs.data[1],
                self.data[2] * rhs.data[2],
            ],
        }
    }
}

impl Div for Vec3 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            data: [
                self.data[0] / rhs.data[0],
                self.data[1] / rhs.data[1],
                self.data[2] / rhs.data[2],
            ],
        }
    }
}

impl PartialEq for Vec3 {
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
        true
    }
}

impl Eq for Vec3 {}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vector3D {
    pub position: Vec3,
    pub normal: Vec3,
    pub coord: Vec2,
    pub colour: Vec4,
    pub tangent: Vec4,
}

impl PartialEq for Vector3D {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.normal == other.normal
            && self.coord == other.coord
            && self.colour == other.colour
            && self.tangent == other.tangent
    }
}

impl Eq for Vector3D {}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vector2D {
    pub position: Vec2,
    pub coord: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VectorKey([i32; 16]);

impl From<&Vector3D> for VectorKey {
    fn from(v: &Vector3D) -> Self {
        let mut data = [0i32; 16];
        let mut i = 0;
        const QUANTIZE_MULTIPLIER: f32 = 1_000_000.0;

        macro_rules! process_vec {
            ($vec:expr) => {
                for val in $vec.data.iter() {
                    // Quantize: (float * Multiplier) -> round() -> i32
                    data[i] = (val * QUANTIZE_MULTIPLIER).round() as i32;
                    i += 1;
                }
            };
        }

        process_vec!(v.position);
        process_vec!(v.normal);
        process_vec!(v.coord);
        process_vec!(v.colour);
        process_vec!(v.tangent);

        VectorKey(data)
    }
}
