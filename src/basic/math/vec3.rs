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
            self.data[0] - other.data[1],
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
        return true;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vector3D {
    pub position: Vec3,
    pub normal: Vec3,
    pub texcoord: Vec2,
    pub colour: Vec4,
    pub tangent: Vec4,
}

pub fn geometry_generate_tangents(vertices: &mut [Vector3D], indices: &mut [u32]) {
    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i + 0] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let edge1 = vertices[i1].position - vertices[i0].position;
        let edge2 = vertices[i2].position - vertices[i0].position;

        let delta_u1 = vertices[i1].texcoord.data[0] - vertices[i0].texcoord.data[0];
        let delta_v1 = vertices[i1].texcoord.data[1] - vertices[i0].texcoord.data[1];

        let delta_u2 = vertices[i2].texcoord.data[0] - vertices[i0].texcoord.data[0];
        let delta_v2 = vertices[i2].texcoord.data[1] - vertices[i0].texcoord.data[1];

        let dividend = delta_u1 * delta_v2 - delta_u2 * delta_v1;
        let fc = 1.0 / dividend;

        let mut tangent = Vec3::new(
            fc * (delta_v2 * edge1.data[0] - delta_v1 * edge2.data[0]),
            fc * (delta_v2 * edge1.data[1] - delta_v1 * edge2.data[1]),
            fc * (delta_v2 * edge1.data[2] - delta_v1 * edge2.data[2]),
        );

        tangent.normalize();

        let sx = delta_u1;
        let sy = delta_u2;
        let tx = delta_v1;
        let ty = delta_v2;
        let handedness = if (tx * sy - ty * sx) < 0.0 { -1.0 } else { 1.0 };
        let t4 = Vec4::vec3_to_vec4(&tangent, handedness);

        vertices[i0].tangent = t4;
        vertices[i1].tangent = t4;
        vertices[i2].tangent = t4;
    }
}

pub fn geometry_generate_normals(vertices: &mut [Vector3D], indices: &mut [u32]) {
    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i + 0] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let edge1 = vertices[i1].position - vertices[i0].position;
        let edge2 = vertices[i2].position - vertices[i0].position;

        let mut normal = edge1.cross(&edge2);
        normal.normalize();

        vertices[i0].normal = normal;
        vertices[i1].normal = normal;
        vertices[i2].normal = normal;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Vector2D {
    pub position: Vec2,
    pub texcoord: Vec2,
}
