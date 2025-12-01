use crate::application::{basic::math::consts::INVALID_ID, renderer::camera::Camera};
use std::collections::HashMap;
use thiserror::Error;

type Result<T> = std::result::Result<T, CameraSysError>;

#[derive(Debug, Clone, Copy)]
struct CameraRef {
    handle: usize,
    reference_count: usize,
    auto_release: bool,
}

impl Default for CameraRef {
    fn default() -> Self {
        Self {
            handle: INVALID_ID,
            reference_count: 0,
            auto_release: false,
        }
    }
}

impl CameraRef {
    fn handle(mut self, handle: usize) -> Self {
        self.handle = handle;
        self
    }

    fn reference_count(mut self, reference_count: usize) -> Self {
        self.reference_count = reference_count;
        self
    }

    fn auto_release(mut self, auto_release: bool) -> Self {
        self.auto_release = auto_release;
        self
    }
}

#[derive(Error, Debug)]
pub enum CameraSysError {
    #[error("camera system error: {issue} {file} {line} ")]
    MaxCameraCountMet {
        issue: &'static str,
        file: &'static str,
        line: u32,
    },
    #[error("camera system error: tried to release a camera that does not exist {file} {line}")]
    ReleaseCameraThatDoesNotExist { file: &'static str, line: u32 },
    #[error("camera system error: {issue} {file} {line}")]
    SysConfigCount {
        issue: &'static str,
        file: &'static str,
        line: u32,
    },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CameraSysConfig {
    max_count: usize,
}

impl CameraSysConfig {
    pub fn max_count(mut self, max_count: usize) -> Self {
        self.max_count = max_count;
        self
    }
}

pub const DEFAULT_CAMERA_NAME: &'static str = "default";
pub type CameraHandle = usize;

pub struct CameraSystem {
    config: CameraSysConfig,
    default_camera: CameraHandle,
    registered_cameras: Vec<Camera>,
    registered_cameras_hashmap: HashMap<String, CameraRef>,
}

impl CameraSystem {
    pub fn initialize(config: &CameraSysConfig) -> Result<CameraSystem> {
        if config.max_count == 0 {
            return Err(CameraSysError::SysConfigCount {
                issue: "camera system config max count should be greater than zero",
                file: file!(),
                line: line!(),
            });
        }
        let mut registered_cameras = Vec::<Camera>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_cameras.push(Camera::default());
        }
        let idx = registered_cameras
            .iter_mut()
            .enumerate()
            .find_map(|(i, c)| if c.id == INVALID_ID { Some(i) } else { None });

        registered_cameras[idx.unwrap()].id = idx.unwrap();

        let registered_cameras_hashmap = HashMap::<String, CameraRef>::new();
        Ok(Self {
            config: *config,
            default_camera: idx.unwrap(),
            registered_cameras,
            registered_cameras_hashmap,
        })
    }

    pub fn acquire(&mut self, name: &str, auto_release: bool) -> Result<CameraHandle> {
        let mut camera_ref = self
            .registered_cameras_hashmap
            .remove(name)
            .unwrap_or_default();

        if camera_ref.handle == INVALID_ID {
            let idx = self
                .registered_cameras
                .iter()
                .enumerate()
                .find_map(|(i, c_ref)| {
                    if c_ref.id == INVALID_ID {
                        Some(i)
                    } else {
                        None
                    }
                });
            let index = if idx.is_none() {
                return Err(CameraSysError::MaxCameraCountMet {
                    issue: "registered camera array is full",
                    file: file!(),
                    line: line!(),
                });
            } else {
                idx.unwrap()
            };
            self.registered_cameras[index].id = index;
            camera_ref.handle = index;
            camera_ref.reference_count = 1;
            camera_ref.auto_release = auto_release;
            self.registered_cameras_hashmap
                .insert(name.to_string(), camera_ref);
            Ok(index)
        } else {
            camera_ref.reference_count += 1;
            self.registered_cameras_hashmap
                .insert(name.to_string(), camera_ref);
            Ok(camera_ref.handle)
        }
    }

    pub fn release(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_CAMERA_NAME {
            return Ok(());
        }
        let mut camera_ref = self
            .registered_cameras_hashmap
            .remove(name)
            .unwrap_or_default();
        if camera_ref.handle == INVALID_ID {
            return Err(CameraSysError::ReleaseCameraThatDoesNotExist {
                file: file!(),
                line: line!(),
            });
        }

        // TODO: add this check in other systems
        // there might be an underflow problem when auto_release is set to false and
        // try to release when the reference_count is already zero
        if camera_ref.reference_count > 0 {
            camera_ref.reference_count -= 1
        };

        if camera_ref.reference_count == 0 {
            self.registered_cameras[camera_ref.handle] = Camera::default();
            Ok(())
        } else {
            self.registered_cameras_hashmap
                .insert(name.to_string(), camera_ref);
            Ok(())
        }
    }

    pub fn get_default_camera(&self) -> &Camera {
        &self.registered_cameras[self.default_camera]
    }

    pub fn get_mut_default_camera(&mut self) -> &mut Camera {
        &mut self.registered_cameras[self.default_camera]
    }
}
