use crate::application::basic::math::{consts::INVALID_ID, matrix4::Matrix4, vec3::Vec3};

pub struct Camera {
    pub id: usize,
    position: Vec3,
    euler_rotation: Vec3,
    view_matrix: Matrix4,
    is_dirty: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            position: Vec3::new_zeroes(),
            euler_rotation: Vec3::new_zeroes(),
            view_matrix: Matrix4::identity(),
            is_dirty: false,
        }
    }
}

impl Camera {
    pub fn create() -> Self {
        Self {
            id: INVALID_ID,
            position: Vec3::new_zeroes(),
            euler_rotation: Vec3::new_zeroes(),
            view_matrix: Matrix4::identity(),
            is_dirty: false,
        }
    }

    pub fn set_dirty(&mut self, is_dirty: bool) {
        self.is_dirty = is_dirty;
    }

    pub fn get_position(&self) -> &Vec3 {
        &self.position
    }

    pub fn set_position(&mut self, position: &Vec3) {
        self.position = *position;
        self.is_dirty = true;
    }

    pub fn get_euler_rotation(&self) -> &Vec3 {
        &self.euler_rotation
    }

    pub fn set_euler_rotation(&mut self, rotation: &Vec3) {
        self.euler_rotation = *rotation;
        self.is_dirty = true;
    }

    pub fn get_view(&self) -> &Matrix4 {
        &self.view_matrix
    }

    pub fn set_view(&mut self, view: &Matrix4) {
        self.view_matrix = *view;
    }

    pub fn recalculate_view(&mut self) {
        if self.is_dirty {
            let rotation = Matrix4::euler_xyz(
                self.euler_rotation.data[0],
                self.euler_rotation.data[1],
                self.euler_rotation.data[2],
            );
            let translation = Matrix4::translation(&self.position);
            self.view_matrix = rotation * translation;
            self.view_matrix = Matrix4::inverse(&self.view_matrix);
            self.is_dirty = false;
        }
    }

    pub fn get_forward(&self) -> Vec3 {
        self.view_matrix.forward()
    }

    pub fn get_backward(&self) -> Vec3 {
        self.view_matrix.backward()
    }

    pub fn get_left(&self) -> Vec3 {
        self.view_matrix.left()
    }

    pub fn get_right(&self) -> Vec3 {
        self.view_matrix.right()
    }

    pub fn move_forward(&mut self, amount: f32) {
        let mut direction = self.get_forward();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn move_backward(&mut self, amount: f32) {
        let mut direction = self.get_backward();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn move_left(&mut self, amount: f32) {
        let mut direction = self.get_left();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn move_right(&mut self, amount: f32) {
        let mut direction = self.get_right();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn move_down(&mut self, amount: f32) {
        let mut direction = Vec3::new_down();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn move_up(&mut self, amount: f32) {
        let mut direction = Vec3::new_up();
        direction = direction.mul_scalar(amount);
        self.position = self.position + direction;
        self.is_dirty = true;
    }

    pub fn yaw(&mut self, amount: f32) {
        self.euler_rotation.data[1] += amount;
        self.is_dirty = true;
    }

    pub fn pitch(&mut self, amount: f32) {
        const LIMIT: f32 = 1.55334306;
        self.euler_rotation.data[0] += amount;
        self.euler_rotation.data[0] = self.euler_rotation.data[0].clamp(-LIMIT, LIMIT);
        self.is_dirty = true;
    }
}
