use std::ops::Mul;

use super::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct Matrix4 {
    pub data: [f32; 16],
}

impl Matrix4 {
    pub fn new(data: [f32; 16]) -> Self {
        Self { data }
    }

    pub fn identity() -> Self {
        Self::new([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    pub fn new_zeros() -> Self {
        Self::new([
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ])
    }

    pub fn new_ones() -> Self {
        Self::new([
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ])
    }

    pub fn perspective(fovy_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        // 'f' is a scaling factor based on the vertical field of view (fovy).
        // A smaller fovy results in a larger scaling factor (more zoomed in).
        let half_tan_fov = (fovy_rad * 0.5).tan();

        let mut mat = Self::new_zeros();
        mat.data[0] = 1.0 / (aspect * half_tan_fov);
        mat.data[5] = 1.0 / half_tan_fov;
        mat.data[10] = -((far + near) / (far - near));
        mat.data[11] = -1.0;
        mat.data[14] = -((2.0 * far * near) / (far - near));

        mat
    }
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut matrix = Matrix4::identity();

        let lr = 1.0 / (left - right);
        let bt = 1.0 / (bottom - top);
        let nf = 1.0 / (near - far);

        matrix.data[0] = -2.0 * lr;
        matrix.data[5] = -2.0 * bt;
        matrix.data[10] = 2.0 * nf;
        matrix.data[12] = (left + right) * lr;
        matrix.data[13] = (top + bottom) * bt;
        matrix.data[14] = (far + near) * nf;

        matrix
    }

    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> Matrix4 {
        let mut out_matrix: Self = Self::new_zeros();
        let mut z_axis: Vec3 = Vec3::default();
        z_axis.set_x(target.data[0] - position.data[0]);
        z_axis.set_y(target.data[1] - position.data[1]);
        z_axis.set_z(target.data[2] - position.data[2]);

        z_axis.normalize();
        let mut cross = z_axis.cross(&up);
        cross.normalize();
        let x_axis: Vec3 = cross;
        let y_axis = x_axis.cross(&z_axis);

        out_matrix.data[0] = x_axis.data[0];
        out_matrix.data[1] = y_axis.data[0];
        out_matrix.data[2] = -z_axis.data[0];
        out_matrix.data[3] = 0.0;
        out_matrix.data[4] = x_axis.data[1];
        out_matrix.data[5] = y_axis.data[1];
        out_matrix.data[6] = -z_axis.data[1];
        out_matrix.data[7] = 0.0;
        out_matrix.data[8] = x_axis.data[2];
        out_matrix.data[9] = y_axis.data[2];
        out_matrix.data[10] = -z_axis.data[2];
        out_matrix.data[11] = 0.0;
        out_matrix.data[12] = -x_axis.dot(&position);
        out_matrix.data[13] = -y_axis.dot(&position);
        out_matrix.data[14] = z_axis.dot(&position);
        out_matrix.data[15] = 1.0;

        out_matrix
    }

    pub fn transposed(matrix: &Matrix4) -> Matrix4 {
        let mut out_matrix = Matrix4::identity();
        out_matrix.data[0] = matrix.data[0];
        out_matrix.data[1] = matrix.data[4];
        out_matrix.data[2] = matrix.data[8];
        out_matrix.data[3] = matrix.data[12];
        out_matrix.data[4] = matrix.data[1];
        out_matrix.data[5] = matrix.data[5];
        out_matrix.data[6] = matrix.data[9];
        out_matrix.data[7] = matrix.data[13];
        out_matrix.data[8] = matrix.data[2];
        out_matrix.data[9] = matrix.data[6];
        out_matrix.data[10] = matrix.data[10];
        out_matrix.data[11] = matrix.data[14];
        out_matrix.data[12] = matrix.data[3];
        out_matrix.data[13] = matrix.data[7];
        out_matrix.data[14] = matrix.data[11];
        out_matrix.data[15] = matrix.data[15];
        out_matrix
    }

    pub fn inverse(matrix: &Matrix4) -> Matrix4 {
        let m = &matrix.data;

        let t0 = m[10] * m[15];
        let t1 = m[14] * m[11];
        let t2 = m[6] * m[15];
        let t3 = m[14] * m[7];
        let t4 = m[6] * m[11];
        let t5 = m[10] * m[7];
        let t6 = m[2] * m[15];
        let t7 = m[14] * m[3];
        let t8 = m[2] * m[11];
        let t9 = m[10] * m[3];
        let t10 = m[2] * m[7];
        let t11 = m[6] * m[3];
        let t12 = m[8] * m[13];
        let t13 = m[12] * m[9];
        let t14 = m[4] * m[13];
        let t15 = m[12] * m[5];
        let t16 = m[4] * m[9];
        let t17 = m[8] * m[5];
        let t18 = m[0] * m[13];
        let t19 = m[12] * m[1];
        let t20 = m[0] * m[9];
        let t21 = m[8] * m[1];
        let t22 = m[0] * m[5];
        let t23 = m[4] * m[1];

        let mut out_matrix = Matrix4::new_zeros();
        let o = &mut out_matrix.data;

        o[0] = (t0 * m[5] + t3 * m[9] + t4 * m[13]) - (t1 * m[5] + t2 * m[9] + t5 * m[13]);
        o[1] = (t1 * m[1] + t6 * m[9] + t9 * m[13]) - (t0 * m[1] + t7 * m[9] + t8 * m[13]);
        o[2] = (t2 * m[1] + t7 * m[5] + t10 * m[13]) - (t3 * m[1] + t6 * m[5] + t11 * m[13]);
        o[3] = (t5 * m[1] + t8 * m[5] + t11 * m[9]) - (t4 * m[1] + t9 * m[5] + t10 * m[9]);

        let d = 1.0 / (m[0] * o[0] + m[4] * o[1] + m[8] * o[2] + m[12] * o[3]);

        o[0] = d * o[0];
        o[1] = d * o[1];
        o[2] = d * o[2];
        o[3] = d * o[3];
        o[4] = d * ((t1 * m[4] + t2 * m[8] + t5 * m[12]) - (t0 * m[4] + t3 * m[8] + t4 * m[12]));
        o[5] = d * ((t0 * m[0] + t7 * m[8] + t8 * m[12]) - (t1 * m[0] + t6 * m[8] + t9 * m[12]));
        o[6] = d * ((t3 * m[0] + t6 * m[4] + t11 * m[12]) - (t2 * m[0] + t7 * m[4] + t10 * m[12]));
        o[7] = d * ((t4 * m[0] + t9 * m[4] + t10 * m[8]) - (t5 * m[0] + t8 * m[4] + t11 * m[8]));
        o[8] = d
            * ((t12 * m[7] + t15 * m[11] + t16 * m[15]) - (t13 * m[7] + t14 * m[11] + t17 * m[15]));
        o[9] = d
            * ((t13 * m[3] + t18 * m[11] + t21 * m[15]) - (t12 * m[3] + t19 * m[11] + t20 * m[15]));
        o[10] =
            d * ((t14 * m[3] + t19 * m[7] + t22 * m[15]) - (t15 * m[3] + t18 * m[7] + t23 * m[15]));
        o[11] =
            d * ((t17 * m[3] + t20 * m[7] + t23 * m[11]) - (t16 * m[3] + t21 * m[7] + t22 * m[11]));
        o[12] = d
            * ((t14 * m[10] + t17 * m[14] + t13 * m[6]) - (t16 * m[14] + t12 * m[6] + t15 * m[10]));
        o[13] = d
            * ((t20 * m[14] + t12 * m[2] + t19 * m[10]) - (t18 * m[10] + t21 * m[14] + t13 * m[2]));
        o[14] =
            d * ((t18 * m[6] + t23 * m[14] + t15 * m[2]) - (t22 * m[14] + t14 * m[2] + t19 * m[6]));
        o[15] =
            d * ((t22 * m[10] + t16 * m[2] + t21 * m[6]) - (t20 * m[6] + t23 * m[10] + t17 * m[2]));

        return out_matrix;
    }

    pub fn translation(position: &Vec3) -> Self {
        let mut out = Matrix4::identity();
        out.data[12] = position.data[0];
        out.data[13] = position.data[1];
        out.data[14] = position.data[2];
        out
    }

    pub fn scale(scale: Vec3) -> Matrix4 {
        let mut out_matrix = Matrix4::identity();
        out_matrix.data[0] = scale.data[0];
        out_matrix.data[5] = scale.data[1];
        out_matrix.data[10] = scale.data[2];
        return out_matrix;
    }

    pub fn forward(&self) -> Vec3 {
        let mut forward = Vec3::new(-self.data[2], -self.data[6], -self.data[10]);
        forward.normalize();
        forward
    }

    pub fn backward(&self) -> Vec3 {
        let mut backward = Vec3::new(self.data[2], self.data[6], self.data[10]);
        backward.normalize();
        backward
    }

    pub fn left(&self) -> Vec3 {
        let mut left = Vec3::new(-self.data[0], -self.data[4], -self.data[8]);
        left.normalize();
        left
    }
    pub fn right(&self) -> Vec3 {
        let mut right = Vec3::new(self.data[0], self.data[4], self.data[8]);
        right.normalize();
        right
    }

    pub fn euler_x(angle_radians: f32) -> Matrix4 {
        let mut out_matrix = Matrix4::identity();
        let c = angle_radians.cos();
        let s = angle_radians.sin();

        out_matrix.data[5] = c;
        out_matrix.data[6] = s;
        out_matrix.data[9] = -s;
        out_matrix.data[10] = c;
        return out_matrix;
    }
    pub fn euler_y(angle_radians: f32) -> Matrix4 {
        let mut out_matrix = Matrix4::identity();
        let c = angle_radians.cos();
        let s = angle_radians.sin();

        out_matrix.data[0] = c;
        out_matrix.data[2] = -s;
        out_matrix.data[8] = s;
        out_matrix.data[10] = c;
        return out_matrix;
    }
    pub fn euler_z(angle_radians: f32) -> Matrix4 {
        let mut out_matrix = Matrix4::identity();

        let c = angle_radians.cos();
        let s = angle_radians.sin();

        out_matrix.data[0] = c;
        out_matrix.data[1] = s;
        out_matrix.data[4] = -s;
        out_matrix.data[5] = c;
        return out_matrix;
    }
    pub fn euler_xyz(x_radians: f32, y_radians: f32, z_radians: f32) -> Matrix4 {
        let rx = Matrix4::euler_x(x_radians);
        let ry = Matrix4::euler_y(y_radians);
        let rz = Matrix4::euler_z(z_radians);
        let mut out_matrix = rx * ry;
        out_matrix = out_matrix * rz;
        return out_matrix;
    }
}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = Self::new_zeros();

        for i in 0..4 {
            // Row of the result matrix
            for j in 0..4 {
                // Column of the result matrix
                let mut sum = 0.0;
                for k in 0..4 {
                    // Inner loop for dot product
                    sum += self.data[i * 4 + k] * rhs.data[k * 4 + j];
                }
                result.data[i * 4 + j] = sum;
            }
        }
        result
    }
}
