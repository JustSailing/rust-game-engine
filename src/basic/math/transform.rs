use std::{cell::RefCell, rc::Rc};

use crate::application::basic::math::{matrix4::Matrix4, vec3::Vec3, vec4::Quat};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    is_dirty: bool,
    local: Matrix4,
    parent: Option<Rc<RefCell<Transform>>>,
}

impl Transform {
    pub fn create() -> Self {
        Self {
            position: Vec3::new_zeroes(),
            rotation: Quat::identity(),
            scale: Vec3::new_ones(),
            is_dirty: true,
            local: Matrix4::identity(),
            parent: None,
        }
    }

    pub fn from_pos(position: Vec3) -> Self {
        Self {
            position,
            ..Self::create()
        }
    }
    pub fn from_rot(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::create()
        }
    }
    pub fn from_pos_and_rot(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Self::create()
        }
    }
    pub fn from_pos_rot_scale(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
            ..Self::create()
        }
    }

    pub fn parent(&self) -> Option<Rc<RefCell<Transform>>> {
        self.parent.as_ref().map(Rc::clone)
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }
    pub fn rotation(&self) -> Quat {
        self.rotation
    }
    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn local_matrix(&mut self) -> Matrix4 {
        if self.is_dirty {
            self.local = Matrix4::translation(&self.position)
                * Quat::to_matrix4(self.rotation)
                * Matrix4::scale(self.scale);
        }
        self.is_dirty = false;
        self.local
    }

    pub fn get_world(&mut self) -> Matrix4 {
        let l = self.local_matrix();
        let result = if let Some(ref mut parent) = self.parent {
            parent.borrow_mut().get_world() * l
        } else {
            l
        };

        result
    }
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.mark_dirty();
    }
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.mark_dirty();
    }
    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.mark_dirty();
    }
    pub fn set_parent(&mut self, parent: Rc<RefCell<Transform>>) {
        self.parent = Some(parent);
        self.mark_dirty();
    }
    pub fn set_pos_rot_scale(&mut self, position: Vec3, rotation: Quat, scale: Vec3) {
        self.position = position;
        self.rotation = rotation;
        self.scale = scale;
        self.mark_dirty();
    }

    pub fn translate(&mut self, delta: Vec3) {
        self.position = self.position + delta;
        self.mark_dirty();
    }

    pub fn rotate(&mut self, delta: Quat) {
        self.rotation = self.rotation * delta;
        self.mark_dirty();
    }

    pub fn rotate_world(&mut self, delta: Quat) {
        self.rotation = delta * self.rotation;
        self.mark_dirty();
    }

    pub fn translate_and_rotate(&mut self, delta: Vec3, delta_rot: Quat) {
        self.position = self.position + delta;
        self.rotation = self.rotation * delta_rot;
        self.mark_dirty();
    }
}
