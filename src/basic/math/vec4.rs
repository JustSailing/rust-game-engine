use crate::application::basic::math::matrix4::Matrix4;

use super::consts::FLOAT_EPSILON;
use std::ops::{Add, Div, Mul, Sub};

use super::vec3::Vec3;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vec4 {
    pub data: [f32; 4],
}

impl Vec4 {
    pub fn new(first: f32, second: f32, third: f32, fourth: f32) -> Self {
        Self {
            data: [first, second, third, fourth],
        }
    }
    pub fn new_zeroes() -> Self {
        Self {
            data: [0.0, 0.0, 0.0, 0.0],
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

pub type Quat = Vec4;

impl Quat {
    pub fn identity() -> Self {
        Self {
            data: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn normal(&self) -> f32 {
        (self.data[0] * self.data[0]
            + self.data[1] * self.data[1]
            + self.data[2] * self.data[2]
            + self.data[3] * self.data[3])
            .sqrt()
    }

    pub fn q_normalize(&mut self) {
        let length = self.length();
        self.data[0] /= length;
        self.data[1] /= length;
        self.data[2] /= length;
        self.data[3] /= length;
    }

    pub fn from_axis_angle(axis: Vec3, angle: f32, normalize: bool) -> Self {
        let half_angle: f32 = 0.5 * angle;
        let s = half_angle.sin();
        let c = half_angle.cos();

        let mut q = Self {
            data: [s * axis.data[0], s * axis.data[1], s * axis.data[2], c],
        };
        if normalize {
            q.q_normalize();
        }
        q
    }

    pub fn to_rotation_matrix(q: Quat, center: Vec3) -> Matrix4 {
        let mut out_matrix = Matrix4::new_zeros();

        let o = &mut out_matrix.data;
        o[0] = (q.data[0] * q.data[0]) - (q.data[1] * q.data[1]) - (q.data[2] * q.data[2])
            + (q.data[3] * q.data[3]);
        o[1] = 2.0 * ((q.data[0] * q.data[1]) + (q.data[2] * q.data[3]));
        o[2] = 2.0 * ((q.data[0] * q.data[2]) - (q.data[1] * q.data[3]));
        o[3] =
            center.data[0] - center.data[0] * o[0] - center.data[1] * o[1] - center.data[2] * o[2];

        o[4] = 2.0 * ((q.data[0] * q.data[1]) - (q.data[2] * q.data[3]));
        o[5] = -(q.data[0] * q.data[0]) + (q.data[1] * q.data[1]) - (q.data[2] * q.data[2])
            + (q.data[3] * q.data[3]);
        o[6] = 2.0 * ((q.data[1] * q.data[2]) + (q.data[0] * q.data[3]));
        o[7] =
            center.data[1] - center.data[0] * o[4] - center.data[1] * o[5] - center.data[2] * o[6];

        o[8] = 2.0 * ((q.data[0] * q.data[2]) + (q.data[1] * q.data[3]));
        o[9] = 2.0 * ((q.data[1] * q.data[2]) - (q.data[0] * q.data[3]));
        o[10] = -(q.data[0] * q.data[0]) - (q.data[1] * q.data[1])
            + (q.data[2] * q.data[2])
            + (q.data[3] * q.data[3]);
        o[11] =
            center.data[2] - center.data[0] * o[8] - center.data[1] * o[9] - center.data[2] * o[10];

        o[12] = 0.0;
        o[13] = 0.0;
        o[14] = 0.0;
        o[15] = 1.0;
        out_matrix
    }
}
