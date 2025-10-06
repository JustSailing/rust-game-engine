use std::{collections::HashMap, fmt};

use crate::application::{
    basic::math::{
        consts::INVALID_ID,
        vec2::Vec2,
        vec3::{Vec3, Vector3D},
    },
    renderer::renderer_types::Renderer,
    resources::resource_types::Geometry,
    systems::material_system::{MaterialConfig, MaterialSystem},
};
pub struct GeometrySysConfig {
    pub max_count: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct GeometryConfig {
    vertices: Vec<Vector3D>,
    indices: Vec<u32>,
    name: String,
    material_name: String,
}

const DEFAULT_GEOMETRY_NAME: &'static str = "default";
type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub enum Error {
    AlreadyInitialized,
    NotInitialized,
    AlreadyShutdown,
    GeometryCountZero,
    IdIsInvalid,
    RegisteredGeometryFull,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyInitialized => write!(
                f,
                "Geometry System State Already Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::NotInitialized => write!(
                f,
                "Geometry System State Not Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::AlreadyShutdown => write!(
                f,
                "Geometry System State Already Shutdown {}  {}",
                file!(),
                line!()
            ),
            Error::GeometryCountZero => write!(
                f,
                "Geometry System State given a count of zero in GeometrySysConfig {}  {}",
                file!(),
                line!()
            ),
            Error::IdIsInvalid => write!(
                f,
                "Geometry System State was given an invalid id {} {}",
                file!(),
                line!()
            ),
            Error::RegisteredGeometryFull => write!(
                f,
                "Geometry System State the amount of registered geometries exceed. Increase amount in GeometrySysConfig {} {}",
                file!(),
                line!()
            ),
        }
    }
}

impl std::error::Error for Error {}

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
    registered_geometries: Vec<Geometry<'a>>,
    registered_geometries_hashmap: HashMap<usize, GeometryRef>,
}

static mut GEOMETRY_STATE: Option<GeometrySystem> = None;

impl<'a: 'static> GeometrySystem<'a> {
    pub fn initialize(config: GeometrySysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = GEOMETRY_STATE {
                return Err(Error::AlreadyInitialized.into());
            }
        }
        if config.max_count == 0 {
            return Err(Error::GeometryCountZero.into());
        }

        let mut registered_array = Vec::<Geometry>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<usize, GeometryRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Geometry::default());
        }

        unsafe {
            GEOMETRY_STATE = Some(GeometrySystem {
                config: config,
                default_geometry: Self::create_default_geometry()?,
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
                return Err(Error::NotInitialized.into());
            }
        };
    }

    pub fn acquire_by_id(id: usize) -> Result<&'a mut Geometry<'a>> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
            }
        };
        let geo_ref = match state.registered_geometries_hashmap.get_mut(&id) {
            Some(gr) => gr,
            None => return Err(Error::IdIsInvalid.into()),
        };

        geo_ref.reference_count += 1;
        return Ok(&mut state.registered_geometries[geo_ref.handle]);
    }

    pub fn acquire_from_config(
        config: GeometryConfig,
        auto_release: bool,
    ) -> Result<&'a mut Geometry<'a>> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
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
            return Err(Error::RegisteredGeometryFull.into());
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
                return Err(Error::NotInitialized.into());
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

    pub fn create_geometry(handle: usize, config: GeometryConfig) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
            }
        };
        let geo = &mut state.registered_geometries[handle];
        Renderer::create_geometry(geo, &config.vertices, &config.indices)?;
        let mut material_config = MaterialConfig::default().name(&config.name);
        geo.material = Some(MaterialSystem::acquire(&mut material_config)?);
        Ok(())
    }

    pub fn destroy_geometry(geometry: &'a Geometry) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = GEOMETRY_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
            }
        };
        Renderer::destroy_geometry(geometry)?;
        let geo_ref = state
            .registered_geometries_hashmap
            .get_mut(&geometry.id)
            .unwrap();
        MaterialSystem::release(&geometry.material.as_ref().unwrap().name)?;
        state.registered_geometries[geo_ref.handle] = Geometry::default();
        state.registered_geometries_hashmap.remove(&geometry.id);
        Ok(())
    }

    pub fn create_default_geometry() -> Result<Geometry<'a>> {
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
        Renderer::create_geometry(&mut geometry, &verts, &indices)?;
        geometry.material = Some(MaterialSystem::get_default_material()?);
        Ok(geometry)
    }
}
