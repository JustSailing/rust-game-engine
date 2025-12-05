use std::{cell::RefCell, rc::Rc, rc::Weak};

use crate::basic::math::{matrix4::Matrix4, vec3::Vec3, vec4::Quat};

pub type TransformWeak = Weak<RefCell<Transform>>;
pub type TransformRc = Rc<RefCell<Transform>>;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    is_dirty: bool,
    local: Matrix4,
    world: Matrix4,
    parent: Option<TransformRc>,
    children: Vec<TransformWeak>,
}

impl Transform {
    pub fn create() -> Self {
        Self {
            position: Vec3::new_zeroes(),
            rotation: Quat::identity(),
            scale: Vec3::new_ones(),
            is_dirty: true,
            local: Matrix4::identity(),
            world: Matrix4::identity(),
            parent: None,
            children: Default::default(),
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

    pub fn position(&self) -> &Vec3 {
        &self.position
    }
    pub fn rotation(&self) -> &Quat {
        &self.rotation
    }
    pub fn scale(&self) -> &Vec3 {
        &self.scale
    }

    fn mark_dirty(&mut self) {
        if !self.is_dirty {
            self.is_dirty = true;
            for child_weak in self.children.iter() {
                if let Some(child_rc) = child_weak.upgrade() {
                    child_rc.borrow_mut().mark_dirty();
                }
            }
        }
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
        if self.is_dirty {
            let local_matrix = self.local_matrix();
            if let Some(ref mut parent) = self.parent {
                let parent_world = parent.borrow_mut().get_world();
                self.world = parent_world * local_matrix;
            } else {
                self.world = local_matrix;
            }
            self.is_dirty = false;
        }
        self.world
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

    pub fn set_parent(child_rc: &TransformRc, parent_rc: &TransformRc) {
        let mut child = child_rc.borrow_mut();

        if let Some(old_parent_rc) = &child.parent {
            let mut old_parent = old_parent_rc.borrow_mut();

            old_parent
                .children
                .retain(|weak_child| match weak_child.upgrade() {
                    Some(strong_child) => !Rc::ptr_eq(&strong_child, child_rc),
                    None => false,
                });
        }

        child.parent = Some(parent_rc.clone());

        let mut new_parent = parent_rc.borrow_mut();
        new_parent.children.push(Rc::downgrade(child_rc));

        child.mark_dirty();
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
