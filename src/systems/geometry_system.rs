use crate::application::{
    basic::math::{
        consts::INVALID_ID,
        vec2::Vec2,
        vec3::{Vec3, Vector2D, Vector3D},
    },
    renderer::renderer_types::{Renderer, RendererError},
    resources::resource_types::{Geometry, MaterialConfig},
    systems::material_system::{MaterialSysError, MaterialSystem},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use thiserror::Error;
pub struct GeometrySysConfig {
    pub max_count: usize,
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct GeometryConfig<T: Clone, U: Clone> {
    pub vertices: Vec<T>,
    pub indices: Vec<U>,
    pub name: String,
    pub material_name: String,
}

const DEFAULT_GEOMETRY_NAME: &'static str = "default";
type Result<T> = std::result::Result<T, GeometrySysError>;

#[derive(Error, Debug)]
pub enum GeometrySysError {
    #[error("geometry system error: already initialized {file} {line}")]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("geometry system error: not initialized {file} {line}")]
    NotInitialized { file: &'static str, line: u32 },
    #[error("geometry system error: already shutdown {file} {line}")]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("geometry system error: config max count is less than 1 {file} {line}")]
    GeometryCountZero { file: &'static str, line: u32 },
    #[error("geometry system error: id is invalid {file} {line}")]
    IdIsInvalid { file: &'static str, line: u32 },
    #[error(
        "geometry system error: registered geometries has reached max count. adjust config {file} {line}"
    )]
    RegisteredGeometryFull { file: &'static str, line: u32 },
    #[error("{source}\ngeometry system error: error returned from material system {file} {line}")]
    MaterialSysError {
        source: MaterialSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\ngeometry system error: error returned from frontend renderer {file} {line}")]
    RendererSysError {
        source: RendererError,
        file: &'static str,
        line: u32,
    },
}

#[derive(Copy, Clone, Default)]
pub struct GeometryRef {
    pub reference_count: usize,
    pub handle: usize,
    pub auto_release: bool,
}

impl GeometryRef {
    pub fn reference_count(mut self, reference_count: usize) -> Self {
        self.reference_count = reference_count;
        self
    }

    pub fn handle(mut self, handle: usize) -> Self {
        self.handle = handle;
        self
    }

    pub fn auto_release(mut self, auto_release: bool) -> Self {
        self.auto_release = auto_release;
        self
    }
}

pub struct GeometrySystem<'a> {
    config: GeometrySysConfig,
    default_geometry: Rc<RefCell<Geometry>>,
    default_geometry_2d: Rc<RefCell<Geometry>>,
    registered_geometries: Vec<Rc<RefCell<Geometry>>>,
    registered_geometries_hashmap: HashMap<usize, GeometryRef>,
    frontend_renderer: Rc<RefCell<Renderer>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
}

impl<'a> GeometrySystem<'a> {
    pub fn initialize(
        config: GeometrySysConfig,
        frontend_renderer: Rc<RefCell<Renderer>>,
        material_system: Rc<RefCell<MaterialSystem<'a>>>,
    ) -> Result<Self> {
        if config.max_count == 0 {
            return Err(GeometrySysError::GeometryCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Rc<RefCell<Geometry>>>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<usize, GeometryRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Rc::new(RefCell::new(Geometry::default())));
        }
        let (geo, geo_2d) = (Geometry::default(), Geometry::default());

        Ok(Self {
            config: config,
            default_geometry: Rc::new(RefCell::new(geo)),
            default_geometry_2d: Rc::new(RefCell::new(geo_2d)),
            registered_geometries: registered_array,
            registered_geometries_hashmap: registered_hash_map,
            frontend_renderer,
            material_system,
        })
    }

    pub fn get_default_geometry(&self) -> Result<Rc<RefCell<Geometry>>> {
        return Ok(Rc::clone(&self.default_geometry));
    }

    pub fn get_default_geometry_2d(&self) -> Result<Rc<RefCell<Geometry>>> {
        return Ok(Rc::clone(&self.default_geometry_2d));
    }

    pub fn destroy_default_geometry(&self) -> Result<()> {
        if self.default_geometry.borrow().id == INVALID_ID {
            return Ok(());
        }
        return Ok(self
            .frontend_renderer
            .borrow_mut()
            .destroy_geometry(&self.default_geometry.borrow())
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?);
    }

    pub fn destroy_default_geometry2d(&self) -> Result<()> {
        if self.default_geometry_2d.borrow().id == INVALID_ID {
            return Ok(());
        }
        return Ok(self
            .frontend_renderer
            .borrow_mut()
            .destroy_geometry(&self.default_geometry_2d.borrow())
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?);
    }

    pub fn acquire_by_id(&mut self, id: usize) -> Result<Rc<RefCell<Geometry>>> {
        let geo_ref = match self.registered_geometries_hashmap.get_mut(&id) {
            Some(gr) => gr,
            None => {
                return Err(GeometrySysError::IdIsInvalid {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        geo_ref.reference_count += 1;
        return Ok(Rc::clone(&self.registered_geometries[geo_ref.handle]));
    }

    pub fn acquire_from_config<T: Clone, U: Clone>(
        &mut self,
        config: GeometryConfig<T, U>,
        auto_release: bool,
    ) -> Result<Rc<RefCell<Geometry>>> {
        let mut geo_ref = GeometryRef::default();
        for (i, geo) in self.registered_geometries.iter_mut().enumerate() {
            if geo.borrow().id == INVALID_ID {
                geo_ref = geo_ref
                    .auto_release(auto_release)
                    .handle(i)
                    .reference_count(1);
                self.registered_geometries_hashmap.insert(i, geo_ref);
                break;
            }
        }
        if geo_ref.handle == INVALID_ID {
            return Err(GeometrySysError::RegisteredGeometryFull {
                file: file!(),
                line: line!(),
            });
        }
        let geometry = &self.registered_geometries[geo_ref.handle];
        self.create_geometry(geo_ref.handle, config)?;
        return Ok(Rc::clone(geometry));
    }

    pub fn shutdown(&mut self) -> Result<()> {
        for i in 0..self.registered_geometries.len() {
            self.destroy_geometry(i)?;
        }
        Ok(())
    }

    pub fn release(&mut self, geometry: Rc<RefCell<Geometry>>) -> Result<()> {
        if geometry.borrow().id != INVALID_ID {
            let geo_ref = self
                .registered_geometries_hashmap
                .get_mut(&geometry.borrow().id)
                .unwrap();
            if geo_ref.reference_count > 0 {
                geo_ref.reference_count -= 1;
            }

            if geo_ref.reference_count < 1 && geo_ref.auto_release {
                self.frontend_renderer
                    .borrow_mut()
                    .destroy_geometry(&self.registered_geometries[geo_ref.handle].borrow())
                    .map_err(|e| GeometrySysError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_system
                    .borrow_mut()
                    .release(&geometry.borrow().material.borrow().name)
                    .map_err(|e| GeometrySysError::MaterialSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                *self.registered_geometries[geo_ref.handle].borrow_mut() = Geometry::default();
                self.registered_geometries_hashmap
                    .remove(&geometry.borrow().id);
            }
        }
        // maybe warn if invalid id
        Ok(())
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        &self,
        handle: usize,
        config: GeometryConfig<T, U>,
    ) -> Result<()> {
        let geo = &self.registered_geometries[handle];

        self.frontend_renderer
            .borrow_mut()
            .create_geometry(&mut geo.borrow_mut(), &config.vertices, &config.indices)
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let mut material_config = MaterialConfig::default().name(&config.material_name);

        geo.borrow_mut().material = self
            .material_system
            .borrow_mut()
            .acquire(&mut material_config)
            .map_err(|e| GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(())
    }

    pub fn destroy_geometry(&mut self, index: usize) -> Result<()> {
        let geometry = &self.registered_geometries[index].borrow();
        if geometry.id == INVALID_ID {
            //WARN
            return Ok(());
        }
        self.frontend_renderer
            .borrow_mut()
            .destroy_geometry(&self.registered_geometries[index].borrow())
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let geo_ref = self
            .registered_geometries_hashmap
            .get_mut(&geometry.id)
            .unwrap();

        self.material_system
            .borrow_mut()
            .release(&geometry.material.borrow().name)
            .map_err(|e| GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        *self.registered_geometries[geo_ref.handle].borrow_mut() = Geometry::default();
        self.registered_geometries_hashmap.remove(&geometry.id);
        Ok(())
    }

    pub fn create_default_geometries(&mut self) -> Result<()> {
        const FACTOR: f32 = 10.0;
        const VERT_COUNT: usize = 4;
        let verts: [Vector3D; VERT_COUNT] = [
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(0.0, 0.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(1.0, 1.0),
            },
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(0.0, 1.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                texcoord: Vec2::new(1.0, 0.0),
            },
        ];

        const INDEX_COUNT: usize = 6;
        let indices: [u32; INDEX_COUNT] = [0, 1, 2, 0, 3, 1];

        let mut geometry = Geometry::default();
        //geometry.id = 10;
        self.frontend_renderer
            .borrow_mut()
            .create_geometry(&mut geometry, &verts, &indices)
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        geometry.material = self
            .material_system
            .borrow()
            .get_default_material()
            .map_err(|e| GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let verts_2d: [Vector2D; VERT_COUNT] = [
            Vector2D {
                position: Vec2::new(-0.5 * FACTOR, -0.5 * FACTOR),
                texcoord: Vec2::new_zeroes(),
            },
            Vector2D {
                position: Vec2::new(0.5 * FACTOR, 0.5 * FACTOR),
                texcoord: Vec2::new_ones(),
            },
            Vector2D {
                position: Vec2::new(-0.5 * FACTOR, 0.5 * FACTOR),
                texcoord: Vec2::new(0.0, 1.0),
            },
            Vector2D {
                position: Vec2::new(0.5 * FACTOR, -0.5 * FACTOR),
                texcoord: Vec2::new(1.0, 0.0),
            },
        ];
        let indices_2d: [u32; INDEX_COUNT] = [2, 1, 0, 3, 0, 1];
        let mut geometry_2d = Geometry::default();
        //geometry_2d.id = 11;

        self.frontend_renderer
            .borrow_mut()
            .create_geometry(&mut geometry_2d, &verts_2d, &indices_2d)
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        geometry_2d.material = self
            .material_system
            .borrow()
            .get_default_material()
            .map_err(|e| GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        self.default_geometry.replace(geometry);
        self.default_geometry_2d.replace(geometry_2d);
        Ok(())
    }
}

impl<'a> Drop for GeometrySystem<'a> {
    fn drop(&mut self) {
        let _ = self.destroy_default_geometry();
        let _ = self.destroy_default_geometry2d();
        for i in 0..self.registered_geometries.len() {
            if self.registered_geometries[i].borrow().id == INVALID_ID {
                continue;
            }
            let _ = self.destroy_geometry(i);
        }
    }
}
