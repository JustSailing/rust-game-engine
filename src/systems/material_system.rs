use std::{cell::RefCell, collections::HashMap, ffi::c_void, rc::Rc, u16};

use crate::application::{
    basic::{
        filesystem::FileHandleError,
        math::{consts::INVALID_ID, matrix4::Matrix4, vec3::Vec3, vec4::Vec4},
    },
    renderer::renderer_types::{Renderer, RendererError},
    resources::resource_types::{Material, MaterialConfig, ResourceData, ResourceType, TextureUse},
    systems::{
        resource_system::{ResourceSysError, ResourceSystem},
        shader_system::{ShaderSysError, ShaderSystem},
        texture_system::{
            DEFAULT_TEXTURE_NAME, DEFAULT_TEXTURE_NORMAL_NAME, DEFAULT_TEXTURE_SPECULAR_NAME,
            TextureSysError, TextureSystem,
        },
    },
};

use thiserror::Error;

type Result<T> = std::result::Result<T, MaterialSysError>;

#[derive(Error, Debug)]
pub enum MaterialSysError {
    #[error("material system error: already initialized {file} {line}")]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("material system error: not initialized {file} {line}")]
    NotInitialized { file: &'static str, line: u32 },
    #[error("material system error: already shutdown {file} {line}")]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("material system error: config provided has a max count less than 1 {file} {line}")]
    MaterialCountZero { file: &'static str, line: u32 },
    #[error(
        "material system error: material system apply global unrecognized by shader {file} {line}"
    )]
    UnrecognizedApplyGlobalCall { file: &'static str, line: u32 },
    #[error(
        "material system error: material system apply instance unrecognized by shader {file} {line}"
    )]
    UnrecognizedApplyInstanceCall { file: &'static str, line: u32 },
    #[error("material system error: material system apply local {file} {line}")]
    CouldNotApplyLocal { file: &'static str, line: u32 },
    #[error("material system error:  releasing texture that doesn't exist: {name} {file} {line}")]
    ReleaseTextureDoesNotExist {
        name: String,
        file: &'static str,
        line: u32,
    },
    #[error(
        "material system error: registered materials are at max count increase config max count {file} {line}"
    )]
    MaterialRegistedMaterialsFull { file: &'static str, line: u32 },
    #[error("material system error:  acquire texture that doesn't exist: {name} {file} {line}")]
    FailedToAcquireMaterial {
        name: String,
        file: &'static str,
        line: u32,
    },
    #[error("material system error: wrong resource data type {ty} {file} {line}")]
    WrongResourceDataType {
        ty: String,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nmaterial system error:  shader system error in material sys {file} {line}")]
    ShaderSysError {
        source: ShaderSysError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nmaterial system error:  texture system error in material sys {file} {line}")]
    TextureSysError {
        source: TextureSysError,
        file: &'static str,
        line: u32,
    },
    #[error(
        "{source}\nmaterial system error:  frontend renderer error in material sys {file} {line}"
    )]
    RendererSysError {
        source: RendererError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nmaterial system error:  file handle error in material sys {file} {line}")]
    FileHandleError {
        source: FileHandleError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nmaterial system error:  texture system error in material sys {file} {line}")]
    ResourceSysError {
        source: ResourceSysError,
        file: &'static str,
        line: u32,
    },
}

pub struct MaterialSysConfig {
    pub max_count: usize,
}

#[derive(Clone, Copy)]
pub struct MaterialRef {
    reference_count: usize,
    handle: usize,
    auto_release: bool,
}

impl Default for MaterialRef {
    fn default() -> Self {
        Self {
            reference_count: 0,
            handle: INVALID_ID,
            auto_release: false,
        }
    }
}

impl MaterialRef {
    fn auto_release(mut self, auto_release: bool) -> Self {
        self.auto_release = auto_release;
        self
    }
}

pub struct MaterialShaderUniformLocations {
    pub projection: u16,
    pub view: u16,
    pub view_position: u16,
    pub ambient_colour: u16,
    pub diffuse_colour: u16,
    pub diffuse_texture: u16,
    pub specular_texture: u16,
    pub normal_texture: u16,
    pub shininess: u16,
    pub model: u16,
    pub render_mode: u16,
}

impl Default for MaterialShaderUniformLocations {
    fn default() -> Self {
        Self {
            projection: u16::MAX,
            view: u16::MAX,
            view_position: u16::MAX,
            ambient_colour: u16::MAX,
            diffuse_colour: u16::MAX,
            diffuse_texture: u16::MAX,
            model: u16::MAX,
            specular_texture: u16::MAX,
            normal_texture: u16::MAX,
            shininess: u16::MAX,
            render_mode: u16::MAX,
        }
    }
}

pub struct MaterialUiUniformLocations {
    pub projection: u16,
    pub view: u16,
    pub diffuse_colour: u16,
    pub diffuse_texture: u16,
    pub model: u16,
}

impl Default for MaterialUiUniformLocations {
    fn default() -> Self {
        Self {
            projection: u16::MAX,
            view: u16::MAX,
            diffuse_colour: u16::MAX,
            diffuse_texture: u16::MAX,
            model: u16::MAX,
        }
    }
}

const DEFAULT_MATERIAL_NAME: &'static str = "default";
pub const BUILTIN_SHADER_NAME_MATERIAL: &'static str = "Shader.Builtin.Material";
pub const BUILTIN_SHADER_NAME_UI: &'static str = "Shader.Builtin.UI";

pub struct MaterialSystem<'a> {
    config: MaterialSysConfig,
    default_material: Rc<RefCell<Material>>,
    default_material_2d: Rc<RefCell<Material>>,
    registered_materials_hashmap: HashMap<String, MaterialRef>,
    registered_materials: Vec<Rc<RefCell<Material>>>,
    material_shader_id: usize,
    material_locations: MaterialShaderUniformLocations,
    ui_shader_id: usize,
    ui_locations: MaterialUiUniformLocations,
    texture_system: Rc<RefCell<TextureSystem>>,
    frontend_renderer: Rc<RefCell<Renderer>>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
}

impl<'a> MaterialSystem<'a> {
    pub fn initialize(
        config: MaterialSysConfig,
        texture_system: Rc<RefCell<TextureSystem>>,
        frontend_renderer: Rc<RefCell<Renderer>>,
        resource_system: Rc<RefCell<ResourceSystem>>,
        shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    ) -> Result<Self> {
        if config.max_count == 0 {
            return Err(MaterialSysError::MaterialCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Rc<RefCell<Material>>>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<String, MaterialRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Rc::new(RefCell::new(Material::default())));
        }

        Ok(Self {
            config,
            default_material: Rc::new(RefCell::new(Material::default())),
            default_material_2d: Rc::new(RefCell::new(Material::default())),
            registered_materials_hashmap: registered_hash_map,
            registered_materials: registered_array,
            material_locations: MaterialShaderUniformLocations::default(),
            material_shader_id: INVALID_ID,
            ui_shader_id: INVALID_ID,
            ui_locations: MaterialUiUniformLocations::default(),
            texture_system,
            frontend_renderer,
            resource_system,
            shader_system,
        })
    }

    pub fn create_default_materials(&mut self) -> Result<()> {
        let mut material = Material::default();
        material.name = String::from(DEFAULT_MATERIAL_NAME);
        material.diffuse_colour = Vec4 {
            data: [0.0, 0.0, 0.0, 0.0],
        };

        material.diffuse_map.texture =
            self.texture_system
                .borrow()
                .get_default_texture()
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        material.diffuse_map.use_type = TextureUse::MapDiffuse;
        material.diffuse_map_name = DEFAULT_TEXTURE_NAME.to_string();

        material.specular_map.texture = self
            .texture_system
            .borrow()
            .get_default_specular_texture()
            .map_err(|e| MaterialSysError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        material.specular_map.use_type = TextureUse::MapSpecular;
        material.specular_map_name = DEFAULT_TEXTURE_SPECULAR_NAME.to_string();

        material.normal_map.texture = self
            .texture_system
            .borrow()
            .get_default_normal_texture()
            .map_err(|e| MaterialSysError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        material.normal_map.use_type = TextureUse::MapNormal;
        material.normal_map_name = DEFAULT_TEXTURE_NORMAL_NAME.to_string();

        material.shininess = 32.0;

        // I don't think I should create the shader in material system
        // should be handled somewhere else
        let shader = self
            .shader_system
            .borrow()
            .get_shader_by_name(BUILTIN_SHADER_NAME_MATERIAL)
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        material.shader_id = shader.borrow().id;
        //self.material_shader_id = material.shader_id;
        material.internal_id = self
            .frontend_renderer
            .borrow()
            .shader_acquire_instance_resources(&mut shader.borrow_mut())
            .map_err(|e| MaterialSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })? as usize;

        self.default_material.replace(material);
        Ok(())
    }

    pub fn get_default_material(&self) -> Result<Rc<RefCell<Material>>> {
        Ok(Rc::clone(&self.default_material))
    }

    pub fn acquire(
        &mut self,
        config: &mut MaterialConfig,
    ) -> Result<(Rc<RefCell<Material>>, usize)> {
        let mut material_res = self
            .resource_system
            .borrow()
            .load(&config.name, ResourceType::Material)
            .map_err(|e| MaterialSysError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        let mat_config = match material_res.data {
            ResourceData::MaterialResourceData(ref mut material_config) => material_config,
            ResourceData::Unknown => {
                return Err(MaterialSysError::WrongResourceDataType {
                    ty: "Unknown".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ImageResourceData(_) => {
                return Err(MaterialSysError::WrongResourceDataType {
                    ty: "ImageResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::BinaryResourceData(_) => {
                return Err(MaterialSysError::WrongResourceDataType {
                    ty: "BinaryResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ShaderResourceData(_) => {
                return Err(MaterialSysError::WrongResourceDataType {
                    ty: "ShaderResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MeshResourceData(_) => {
                return Err(MaterialSysError::WrongResourceDataType {
                    ty: "MeshResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };

        mat_config.auto_release = config.auto_release;

        let (material, shader_instance_id) = self.acquire_from_config(mat_config)?;

        self.resource_system
            .borrow()
            .unload(&mut material_res)
            .map_err(|e| MaterialSysError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        Ok((material, shader_instance_id))
    }

    pub fn acquire_from_config(
        &mut self,
        config: &mut MaterialConfig,
    ) -> Result<(Rc<RefCell<Material>>, usize)> {
        if config.name == DEFAULT_MATERIAL_NAME {
            return Ok((
                Rc::clone(&self.default_material),
                self.default_material.borrow().internal_id,
            ));
        }
        let mut mat_ref = MaterialRef::default();
        {
            let mat_ref_ = match self.registered_materials_hashmap.get_mut(&config.name) {
                Some(m) => m,
                None => {
                    let mat_ref = MaterialRef::default().auto_release(config.auto_release);
                    self.registered_materials_hashmap
                        .insert(config.name.clone(), mat_ref);
                    self.registered_materials_hashmap
                        .get_mut(&config.name)
                        .unwrap()
                }
            };
            mat_ref = *mat_ref_;
        }
        if mat_ref.reference_count == 0 {
            mat_ref.auto_release = config.auto_release;
        }

        mat_ref.reference_count += 1;
        if mat_ref.handle == INVALID_ID {
            for (i, mat) in self.registered_materials.iter_mut().enumerate() {
                if mat.borrow().id == INVALID_ID {
                    mat.borrow_mut().id = i;
                    mat_ref.handle = i;
                    break;
                }
            }
            if mat_ref.handle == INVALID_ID {
                return Err(MaterialSysError::MaterialRegistedMaterialsFull {
                    file: file!(),
                    line: line!(),
                });
            }
            let material = self.load_material(config)?;
            self.registered_materials[mat_ref.handle].replace(material);

            let shader = self
                .shader_system
                .borrow()
                .get_shader_by_name(&config.shader_name)
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            if self.material_shader_id == INVALID_ID
                && config.shader_name == BUILTIN_SHADER_NAME_MATERIAL
            {
                self.material_shader_id = shader.borrow().id;
                self.material_locations.projection = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "projection")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.view = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "view")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.view_position = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "view_position")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.ambient_colour = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "ambient_colour")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_locations.diffuse_colour = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "diffuse_colour")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.diffuse_texture = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "diffuse_texture")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.specular_texture = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "specular_texture")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;

                self.material_locations.normal_texture = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "normal_texture")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.shininess = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "shininess")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.model = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "model")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.material_locations.render_mode = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "mode")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
            } else if self.ui_shader_id == INVALID_ID
                && config.shader_name == BUILTIN_SHADER_NAME_UI
            {
                self.ui_shader_id = shader.borrow().id;
                self.ui_locations.projection = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "projection")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.ui_locations.view = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "view")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.ui_locations.diffuse_colour = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "diffuse_colour")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.ui_locations.diffuse_texture = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "diffuse_texture")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                self.ui_locations.model = self
                    .shader_system
                    .borrow()
                    .uniform_index(&shader.borrow(), "model")
                    .map_err(|e| MaterialSysError::ShaderSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
            }
            if self.registered_materials[mat_ref.handle]
                .borrow()
                .generation
                == INVALID_ID
            {
                self.registered_materials[mat_ref.handle]
                    .borrow_mut()
                    .generation = 0;
            } else {
                self.registered_materials[mat_ref.handle]
                    .borrow_mut()
                    .generation += 1;
            }
            self.registered_materials[mat_ref.handle].borrow_mut().id = mat_ref.handle;
        }

        let shader = self
            .shader_system
            .borrow()
            .get_shader_by_name(&config.shader_name)
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        // FIXME: changing internal id means nothing
        let material_instance_id = self
            .frontend_renderer
            .borrow()
            .shader_acquire_instance_resources(&mut shader.borrow_mut())
            .map_err(|e| MaterialSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })? as usize;

        self.insert_hashmap(&config.name, &mat_ref)?;
        Ok((
            Rc::clone(&self.registered_materials[mat_ref.handle]),
            material_instance_id,
        ))
    }

    pub fn release(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_MATERIAL_NAME {
            return Ok(());
        }
        let mut mat_ref = MaterialRef::default();
        {
            let mat_ref_ = match self.registered_materials_hashmap.get_mut(name) {
                Some(t) => t,
                None => {
                    return Err(MaterialSysError::ReleaseTextureDoesNotExist {
                        name: name.to_string(),
                        file: file!(),
                        line: line!(),
                    });
                }
            };
            mat_ref = *mat_ref_;
        }
        if mat_ref.reference_count == 0 {
            println!("WARN tried to release a non-loaded texture.");
            return Ok(());
        }
        mat_ref.reference_count -= 1;
        if mat_ref.reference_count == 0 && mat_ref.auto_release {
            let mat = &self.registered_materials[mat_ref.handle];

            self.destroy_material(&mat.borrow())?;
            self.registered_materials[mat_ref.handle].replace(Material::default());

            // don't think i need the 2 lines below
            mat_ref.handle = INVALID_ID;
            mat_ref.auto_release = false;
            self.registered_materials_hashmap.remove(name);
        }

        Ok(())
    }

    pub fn apply_global(
        &self,
        shader_id: u32,
        projection: &Matrix4,
        view: &Matrix4,
        view_positon: &Vec3,
        ambient_colour: &Vec4,
        mode: u32,
    ) -> Result<()> {
        if shader_id == self.material_shader_id as u32 {
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.projection,
                    projection as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.view,
                    view as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.view_position,
                    view_positon as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.ambient_colour,
                    ambient_colour as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.render_mode,
                    &mode as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        } else if shader_id == self.ui_shader_id as u32 {
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.ui_locations.projection,
                    projection as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(self.ui_locations.view, view as *const _ as *const c_void)
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        } else {
            return Err(MaterialSysError::UnrecognizedApplyGlobalCall {
                file: file!(),
                line: line!(),
            });
        }
        self.shader_system
            .borrow()
            .apply_globals()
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    pub fn apply_instance(
        &self,
        material: &Material,
        material_instance_id: usize,
        model: &Matrix4,
    ) -> Result<()> {
        self.shader_system
            .borrow()
            .bind_instance(material_instance_id)
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        if material.shader_id == self.material_shader_id {
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.diffuse_colour,
                    &material.diffuse_colour as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.diffuse_texture,
                    material.diffuse_map.texture.as_ptr() as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.specular_texture,
                    material.specular_map.texture.as_ptr() as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.normal_texture,
                    material.normal_map.texture.as_ptr() as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.shininess,
                    &material.shininess as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.material_locations.model,
                    model as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        } else if material.shader_id == self.ui_shader_id {
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.ui_locations.diffuse_colour,
                    &material.diffuse_colour as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(
                    self.ui_locations.diffuse_texture,
                    material.diffuse_map.texture.as_ptr() as *const _ as *const c_void,
                )
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.shader_system
                .borrow()
                .uniform_set_by_index(self.ui_locations.model, model as *const _ as *const c_void)
                .map_err(|e| MaterialSysError::ShaderSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        } else {
            return Err(MaterialSysError::UnrecognizedApplyGlobalCall {
                file: file!(),
                line: line!(),
            });
        }
        self.shader_system
            .borrow()
            .apply_instance()
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    // pub fn apply_local(&self, material: &Material, model: &Matrix4) -> Result<()> {
    //     if material.shader_id == self.material_shader_id {
    //     } else if material.shader_id == self.ui_shader_id {
    //         return self
    //             .shader_system
    //             .borrow()
    //             .uniform_set_by_index(self.ui_locations.model, model as *const _ as *const c_void)
    //             .map_err(|e| MaterialSysError::ShaderSysError {
    //                 source: e,
    //                 file: file!(),
    //                 line: line!(),
    //             });
    //     }
    //     return Err(MaterialSysError::CouldNotApplyLocal {
    //         file: file!(),
    //         line: line!(),
    //     });
    // }

    fn insert_hashmap(&mut self, name: &String, mat_ref: &MaterialRef) -> Result<()> {
        self.registered_materials_hashmap
            .insert(name.to_string(), *mat_ref);
        Ok(())
    }

    fn load_material(&mut self, config: &mut MaterialConfig) -> Result<Material> {
        let mut mat = Material::default();
        mat.shader_id = self
            .shader_system
            .borrow()
            .get_shader_id(&config.shader_name)
            .map_err(|e| MaterialSysError::ShaderSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        mat.name = config.name.clone();
        mat.diffuse_colour = config.diffuse_colour.clone();
        mat.shininess = config.shininess;
        if config.diffuse_map_name.len() > 0 {
            mat.diffuse_map_name = config.diffuse_map_name.clone();
            mat.diffuse_map.use_type = TextureUse::MapDiffuse;
            mat.diffuse_map.texture = self
                .texture_system
                .borrow_mut()
                .acquire(config.diffuse_map_name.clone(), config.auto_release)
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        } else {
            mat.diffuse_map.use_type = TextureUse::MapDiffuse;
            mat.diffuse_map.texture =
                self.texture_system
                    .borrow()
                    .get_default_texture()
                    .map_err(|e| MaterialSysError::TextureSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
        }

        if config.specular_map_name.len() > 0 {
            mat.specular_map_name = config.specular_map_name.clone();
            mat.specular_map.use_type = TextureUse::MapSpecular;
            let texture = self
                .texture_system
                .borrow_mut()
                .acquire(config.specular_map_name.clone(), config.auto_release)
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            mat.specular_map.texture = texture;
        } else {
            mat.specular_map.use_type = TextureUse::MapDiffuse;
            mat.specular_map.texture = self
                .texture_system
                .borrow()
                .get_default_specular_texture()
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }

        if config.normal_map_name.len() > 0 {
            mat.normal_map_name = config.normal_map_name.clone();
            mat.normal_map.use_type = TextureUse::MapNormal;
            let texture = self
                .texture_system
                .borrow_mut()
                .acquire(config.normal_map_name.clone(), config.auto_release)
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            mat.normal_map.texture = texture;
        } else {
            mat.normal_map.use_type = TextureUse::MapDiffuse;
            mat.normal_map.texture = self
                .texture_system
                .borrow()
                .get_default_normal_texture()
                .map_err(|e| MaterialSysError::TextureSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }
        Ok(mat)
    }

    pub fn destroy_material(&self, material: &Material) -> Result<()> {
        let id = material.diffuse_map.texture.borrow().id;
        self.texture_system
            .borrow_mut()
            .release_by_id(id)
            .map_err(|e| MaterialSysError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let id = material.specular_map.texture.borrow().id;
        self.texture_system
            .borrow_mut()
            .release_by_id(id)
            .map_err(|e| MaterialSysError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let id = material.normal_map.texture.borrow().id;
        self.texture_system
            .borrow_mut()
            .release_by_id(id)
            .map_err(|e| MaterialSysError::TextureSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        if material.shader_id != INVALID_ID && material.internal_id != INVALID_ID {
            self.frontend_renderer
                .borrow()
                .shader_release_instance_resources(
                    &mut self
                        .shader_system
                        .borrow()
                        .get_shader_by_id(material.shader_id)
                        .map_err(|e| MaterialSysError::ShaderSysError {
                            source: e,
                            file: file!(),
                            line: line!(),
                        })?
                        .borrow_mut(),
                    material.internal_id as u32,
                )
                .map_err(|e| MaterialSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }

        Ok(())
    }
}
