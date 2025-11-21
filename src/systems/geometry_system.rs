use crate::application::{
    basic::math::{
        consts::INVALID_ID,
        vec2::Vec2,
        vec3::{Vec3, Vector2D, Vector3D, VectorKey},
        vec4::Vec4,
    },
    renderer::renderer_types::{Renderer, RendererError},
    resources::resource_types::{Geometry, GeometryConfig, MaterialConfig},
    systems::material_system::{MaterialSysError, MaterialSystem},
    systems::shader_system::{ShaderSysError, ShaderSystem},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};
use thiserror::Error;
pub struct GeometrySysConfig {
    pub max_count: usize,
}

pub const DEFAULT_GEOMETRY_NAME: &'static str = "default";
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
    #[error("{source}\nshader system error: error returned from shader system {file} {line}")]
    ShaderSysError {
        source: ShaderSysError,
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
    // NOTE: should change this. It's only here to release shader resources
    // have issues destroying samplers since they are being used by a descriptor set
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
}

impl<'a> GeometrySystem<'a> {
    pub fn initialize(
        config: GeometrySysConfig,
        frontend_renderer: Rc<RefCell<Renderer>>,
        material_system: Rc<RefCell<MaterialSystem<'a>>>,
        shader_system: Rc<RefCell<ShaderSystem<'a>>>,
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
            config,
            default_geometry: Rc::new(RefCell::new(geo)),
            default_geometry_2d: Rc::new(RefCell::new(geo_2d)),
            registered_geometries: registered_array,
            registered_geometries_hashmap: registered_hash_map,
            frontend_renderer,
            material_system,
            shader_system,
        })
    }

    pub fn get_default_geometry(&self) -> Result<Rc<RefCell<Geometry>>> {
        Ok(Rc::clone(&self.default_geometry))
    }

    pub fn get_default_geometry_2d(&self) -> Result<Rc<RefCell<Geometry>>> {
        Ok(Rc::clone(&self.default_geometry_2d))
    }

    pub fn destroy_default_geometry(&self) -> Result<()> {
        Ok(self
            .frontend_renderer
            .borrow_mut()
            .destroy_geometry(&self.default_geometry.borrow())
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?)
    }

    pub fn destroy_default_geometry2d(&self) -> Result<()> {
        Ok(self
            .frontend_renderer
            .borrow_mut()
            .destroy_geometry(&self.default_geometry_2d.borrow())
            .map_err(|e| GeometrySysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?)
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
        Ok(Rc::clone(&self.registered_geometries[geo_ref.handle]))
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
                geo.borrow_mut().id = i;
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
        self.create_geometry(geo_ref.handle, config, auto_release)?;
        Ok(Rc::clone(geometry))
    }

    pub fn generate_cube_config(
        &self,
        width: f32,
        height: f32,
        depth: f32,
        tile_x: f32,
        tile_y: f32,
        name: &str,
        material_name: &str,
    ) -> Result<GeometryConfig<Vector3D, u32>> {
        let mut config: GeometryConfig<Vector3D, u32> = GeometryConfig {
            vertices: Vec::with_capacity(4 * 6),
            indices: Vec::with_capacity(6 * 6),
            center: Default::default(),
            min_extents: Default::default(),
            max_extents: Default::default(),
            name: name.to_string(),
            material_name: material_name.to_string(),
        };

        let half_width = width * 0.5;
        let half_height = height * 0.5;
        let half_depth = depth * 0.5;
        let min_x = -half_width;
        let min_y = -half_height;
        let min_z = -half_depth;
        let max_x = half_width;
        let max_y = half_height;
        let max_z = half_depth;
        let min_uvx = 0.0;
        let min_uvy = 0.0;
        let max_uvx = tile_x;
        let max_uvy = tile_y;

        let vert0 = Vector3D {
            position: Vec3::new(min_x, min_y, max_z),
            normal: Vec3::new(0.0, 0.0, -1.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert0);

        let vert1: Vector3D = Vector3D {
            position: Vec3::new(max_x, max_y, max_z),
            normal: Vec3::new(0.0, 0.0, -1.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert1);

        let vert2: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, max_z),
            normal: Vec3::new(0.0, 0.0, -1.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert2);

        let vert3: Vector3D = Vector3D {
            position: Vec3::new(max_x, min_y, max_z),
            normal: Vec3::new(0.0, 0.0, -1.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert3);

        let vert4 = Vector3D {
            position: Vec3::new(max_x, min_y, min_z),
            normal: Vec3::new(0.0, 0.0, 1.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert4);

        let vert5: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, min_z),
            normal: Vec3::new(0.0, 0.0, 1.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert5);

        let vert6: Vector3D = Vector3D {
            position: Vec3::new(max_x, max_y, min_z),
            normal: Vec3::new(0.0, 0.0, 1.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert6);

        let vert7: Vector3D = Vector3D {
            position: Vec3::new(min_x, min_y, min_z),
            normal: Vec3::new(0.0, 0.0, 1.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert7);

        let vert8 = Vector3D {
            position: Vec3::new(min_x, min_y, min_z),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert8);

        let vert9: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, max_z),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert9);

        let vert10: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, min_z),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert10);

        let vert11: Vector3D = Vector3D {
            position: Vec3::new(min_x, min_y, max_z),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert11);

        let vert12: Vector3D = Vector3D {
            position: Vec3::new(max_x, min_y, max_z),
            normal: Vec3::new(1.0, 0.0, 0.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert12);

        let vert13: Vector3D = Vector3D {
            position: Vec3::new(max_x, max_y, min_z),
            normal: Vec3::new(1.0, 0.0, 0.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert13);

        let vert14: Vector3D = Vector3D {
            position: Vec3::new(max_x, max_y, max_z),
            normal: Vec3::new(1.0, 0.0, 0.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert14);

        let vert15 = Vector3D {
            position: Vec3::new(max_x, min_y, min_z),
            normal: Vec3::new(1.0, 0.0, 0.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert15);

        let vert16: Vector3D = Vector3D {
            position: Vec3::new(max_x, min_y, max_z),
            normal: Vec3::new(0.0, -1.0, 0.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert16);

        let vert17: Vector3D = Vector3D {
            position: Vec3::new(min_x, min_y, min_z),
            normal: Vec3::new(0.0, -1.0, 0.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert17);

        let vert18: Vector3D = Vector3D {
            position: Vec3::new(max_x, min_y, min_z),
            normal: Vec3::new(0.0, -1.0, 0.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert18);

        let vert19 = Vector3D {
            position: Vec3::new(min_x, min_y, max_z),
            normal: Vec3::new(0.0, -1.0, 0.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert19);

        let vert20: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, max_z),
            normal: Vec3::new(0.0, 1.0, 0.0),
            coord: Vec2::new(min_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert20);

        let vert21: Vector3D = Vector3D {
            position: Vec3::new(max_x, max_y, min_z),
            normal: Vec3::new(0.0, 1.0, 0.0),
            coord: Vec2::new(max_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert21);

        let vert22: Vector3D = Vector3D {
            position: Vec3::new(min_x, max_y, min_z),
            normal: Vec3::new(0.0, 1.0, 0.0),
            coord: Vec2::new(min_uvx, max_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert22);

        let vert23 = Vector3D {
            position: Vec3::new(max_x, max_y, max_z),
            normal: Vec3::new(0.0, 1.0, 0.0),
            coord: Vec2::new(max_uvx, min_uvy),
            colour: Vec4::new_zeroes(),
            tangent: Vec4::new_zeroes(),
        };
        config.vertices.push(vert23);

        for i in 0..6 {
            let v_offset = i * 4;
            config.indices.push(v_offset + 0);
            config.indices.push(v_offset + 1);
            config.indices.push(v_offset + 2);
            config.indices.push(v_offset + 0);
            config.indices.push(v_offset + 3);
            config.indices.push(v_offset + 1);
        }

        config.name = name.to_string();
        config.material_name = material_name.to_string();
        geometry_generate_tangents(&mut config.vertices, &mut config.indices);
        Ok(config)
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
        auto_release: bool,
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

        let mut material_config = MaterialConfig::default()
            .name(&config.material_name)
            .auto_release(auto_release);
        let mut g = geo.borrow_mut();

        (g.material, g.material_instance_id) = self
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
        if self.registered_geometries[index].borrow().id == INVALID_ID {
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
            .get_mut(&self.registered_geometries[index].borrow().id)
            .unwrap();

        self.material_system
            .borrow_mut()
            .release(
                &self.registered_geometries[index]
                    .borrow()
                    .material
                    .borrow()
                    .name,
            )
            .map_err(|e| GeometrySysError::MaterialSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        *self.registered_geometries[geo_ref.handle].borrow_mut() = Geometry::default();
        self.registered_geometries_hashmap
            .remove(&self.registered_geometries[index].borrow().id);
        Ok(())
    }

    pub fn create_default_geometries(&mut self) -> Result<()> {
        const FACTOR: f32 = 10.0;
        const VERT_COUNT: usize = 4;
        let verts: [Vector3D; VERT_COUNT] = [
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                normal: Vec3::new_zeroes(),
                coord: Vec2::new(0.0, 0.0),
                colour: Vec4::new_zeroes(),
                tangent: Vec4::new_zeroes(),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                normal: Vec3::new_zeroes(),
                coord: Vec2::new(1.0, 1.0),
                colour: Vec4::new_zeroes(),
                tangent: Vec4::new_zeroes(),
            },
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, 0.5 * FACTOR, 0.0),
                normal: Vec3::new_zeroes(),
                coord: Vec2::new(0.0, 1.0),
                colour: Vec4::new_zeroes(),
                tangent: Vec4::new_zeroes(),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, -0.5 * FACTOR, 0.0),
                normal: Vec3::new_zeroes(),
                coord: Vec2::new(1.0, 0.0),
                colour: Vec4::new_zeroes(),
                tangent: Vec4::new_zeroes(),
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
                coord: Vec2::new_zeroes(),
            },
            Vector2D {
                position: Vec2::new(0.5 * FACTOR, 0.5 * FACTOR),
                coord: Vec2::new_ones(),
            },
            Vector2D {
                position: Vec2::new(-0.5 * FACTOR, 0.5 * FACTOR),
                coord: Vec2::new(0.0, 1.0),
            },
            Vector2D {
                position: Vec2::new(0.5 * FACTOR, -0.5 * FACTOR),
                coord: Vec2::new(1.0, 0.0),
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
        for geo in self.registered_geometries.iter_mut() {
            if geo.borrow().id == INVALID_ID {
                continue;
            }
            match self
                .shader_system
                .borrow()
                .get_shader_by_id(geo.borrow().material.borrow().shader_id)
            {
                Ok(s) => {
                    let _ = self
                        .frontend_renderer
                        .borrow_mut()
                        .shader_release_instance_resources(
                            &mut s.borrow_mut(),
                            geo.borrow().material_instance_id as u32,
                        );
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
    }
}

pub fn geometry_generate_tangents(vertices: &mut [Vector3D], indices: &mut [u32]) {
    for i in (0..indices.len()).step_by(3) {
        let i0 = indices[i + 0] as usize;
        let i1 = indices[i + 1] as usize;
        let i2 = indices[i + 2] as usize;

        let edge1 = vertices[i1].position - vertices[i0].position;
        let edge2 = vertices[i2].position - vertices[i0].position;

        let delta_u1 = vertices[i1].coord.data[0] - vertices[i0].coord.data[0];
        let delta_v1 = vertices[i1].coord.data[1] - vertices[i0].coord.data[1];

        let delta_u2 = vertices[i2].coord.data[0] - vertices[i0].coord.data[0];
        let delta_v2 = vertices[i2].coord.data[1] - vertices[i0].coord.data[1];

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

pub fn geometry_deduplicate_vertices(
    geometry_config: &mut GeometryConfig<Vector3D, u32>,
) -> Vec<Vector3D> {
    let original_vertices = &geometry_config.vertices;
    let indices = &mut geometry_config.indices;

    let mut vertex_to_new_index = BTreeMap::new();
    let mut out_vertices = Vec::with_capacity(original_vertices.len());

    let mut old_to_new_index_map = Vec::with_capacity(original_vertices.len());

    for (_, vertex) in original_vertices.iter().enumerate() {
        let key = VectorKey::from(vertex);

        let new_index = *vertex_to_new_index.entry(key).or_insert_with(|| {
            // Vertex is new. Assign it the next available index.
            let next_index = out_vertices.len() as u32;
            out_vertices.push(vertex.clone());
            next_index
        });

        old_to_new_index_map.push(new_index);
    }

    for index in indices.iter_mut() {
        *index = old_to_new_index_map[*index as usize];
    }
    println!(
        "geometry_deduplicate_vertices: removed {:?}, orig/now {:?}/{:?}",
        original_vertices.len() - out_vertices.len(),
        original_vertices.len(),
        out_vertices.len()
    );

    out_vertices
}
