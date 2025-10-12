use crate::application::{
    basic::math::{
        consts::INVALID_ID,
        vec2::Vec2,
        vec3::{Vec3, Vector2D, Vector3D},
    },
    renderer::renderer_types::{RendererError, Renderer},
    resources::resource_types::{Geometry, MaterialConfig},
    systems::material_system::{MaterialSysError, MaterialSystem},
};
use std::collections::HashMap;
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
    default_geometry: Geometry<'a>,
    default_geometry_2d: Geometry<'a>,
    registered_geometries: Vec<Geometry<'a>>,
    registered_geometries_hashmap: HashMap<usize, GeometryRef>,
}

static mut GEOMETRY_STATE: Option<GeometrySystem> = None;

impl<'a: 'static> GeometrySystem<'a> {
    pub fn initialize(config: GeometrySysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = GEOMETRY_STATE {
                return Err(GeometrySysError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
        if config.max_count == 0 {
            return Err(GeometrySysError::GeometryCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Geometry>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<usize, GeometryRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Geometry::default());
        }
        let (geo, geo_2d) = Self::create_default_geometries()?;

        unsafe {
            GEOMETRY_STATE = Some(GeometrySystem {
                config: config,
                default_geometry: geo,
                default_geometry_2d: geo_2d,
                registered_geometries: registered_array,
                registered_geometries_hashmap: registered_hash_map,
            })
        }

        Ok(())
    }

    pub fn get_default_geometry() -> Result<&'a mut Geometry<'a>> {
        unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                return Ok(&mut state.default_geometry);
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
    }

    pub fn get_default_geometry_2d() -> Result<&'a mut Geometry<'a>> {
        unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                return Ok(&mut state.default_geometry_2d);
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
    }

    pub fn acquire_by_id(id: usize) -> Result<&'a mut Geometry<'a>> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let geo_ref = match state.registered_geometries_hashmap.get_mut(&id) {
            Some(gr) => gr,
            None => {
                return Err(GeometrySysError::IdIsInvalid {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        geo_ref.reference_count += 1;
        return Ok(&mut state.registered_geometries[geo_ref.handle]);
    }

    pub fn acquire_from_config<T: Clone, U: Clone>(
        config: GeometryConfig<T, U>,
        auto_release: bool,
    ) -> Result<&'a mut Geometry<'a>> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let mut geo_ref = GeometryRef::default();
        for (i, geo) in state.registered_geometries.iter_mut().enumerate() {
            if geo.id == INVALID_ID {
                geo_ref = geo_ref
                    .auto_release(auto_release)
                    .handle(i)
                    .reference_count(1);
                state.registered_geometries_hashmap.insert(i, geo_ref);
            }
        }
        if geo_ref.handle == INVALID_ID {
            return Err(GeometrySysError::RegisteredGeometryFull {
                file: file!(),
                line: line!(),
            });
        }
        let geometry = &mut state.registered_geometries[geo_ref.handle];
        Self::create_geometry(geo_ref.handle, config)?;
        return Ok(geometry);
    }

    pub fn release(geometry: &mut Geometry<'a>) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if geometry.id != INVALID_ID {
            let geo_ref = state
                .registered_geometries_hashmap
                .get_mut(&geometry.id)
                .unwrap();
            if geo_ref.reference_count > 0 {
                geo_ref.reference_count -= 1;
            }

            if geo_ref.reference_count < 1 && geo_ref.auto_release {
                Self::destroy_geometry(&mut state.registered_geometries[geo_ref.handle])?;
                state.registered_geometries_hashmap.remove(&geometry.id);
            }
        }
        // maybe warn if invalid id
        Ok(())
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        handle: usize,
        config: GeometryConfig<T, U>,
    ) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let geo = &mut state.registered_geometries[handle];
        Renderer::create_geometry(geo, &config.vertices, &config.indices).map_err(|e| {
            GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        let mut material_config = MaterialConfig::default().name(&config.material_name);

        geo.material = Some(MaterialSystem::acquire(&mut material_config).map_err(|e| {
            GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?);
        Ok(())
    }

    pub fn destroy_geometry(geometry: &'a Geometry) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(GeometrySysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        Renderer::destroy_geometry(geometry).map_err(|e| {
            GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        let geo_ref = state
            .registered_geometries_hashmap
            .get_mut(&geometry.id)
            .unwrap();

        MaterialSystem::release(&geometry.material.as_ref().unwrap().name).map_err(|e| {
            GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        state.registered_geometries[geo_ref.handle] = Geometry::default();
        state.registered_geometries_hashmap.remove(&geometry.id);
        Ok(())
    }

    pub fn create_default_geometries() -> Result<(Geometry<'a>, Geometry<'a>)> {
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
        Renderer::create_geometry(&mut geometry, &verts, &indices).map_err(|e| {
            GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        geometry.material = Some(MaterialSystem::get_default_material().map_err(|e| {
            GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?);

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

        Renderer::create_geometry(&mut geometry_2d, &verts_2d, &indices_2d).map_err(|e| {
            GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        geometry_2d.material = Some(MaterialSystem::get_default_material().map_err(|e| {
            GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?);
        Ok((geometry, geometry_2d))
    }
}
