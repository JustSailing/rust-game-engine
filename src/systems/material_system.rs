use std::{cell::RefCell, collections::HashMap, ffi::c_void, rc::Rc, u16};

use crate::{
    basic::{
        filesystem::FileHandleError,
        math::{consts::INVALID_ID, matrix4::Matrix4, vec3::Vec3, vec4::Vec4},
    },
    renderer::frontend_renderer::{
        BUILTIN_SHADER_NAME_MATERIAL, BUILTIN_SHADER_NAME_UI, Renderer, RendererError,
    },
    resources::resource_types::{
        Material, MaterialConfig, MaterialHandle, ResourceData, ResourceFlags, ResourceType,
        TextureFilter, TextureRepeat, TextureUse,
    },
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
    #[error("material system error: shader system error: {0}")]
    ShaderSysErr(#[from] ShaderSysError),

    #[error("material system error: texture system error: {0}")]
    TextureSysErr(#[from] TextureSysError),

    #[error("texture system error: frontend renderer error: {0}")]
    RendererSysError(Box<RendererError>),

    #[error("material system error: file handle error: {0}")]
    FileHandleErr(#[from] FileHandleError),

    #[error("material system error: resource system error: {0}")]
    ResourceSysErr(#[from] ResourceSysError),
}

impl From<RendererError> for MaterialSysError {
    fn from(err: RendererError) -> Self {
        MaterialSysError::RendererSysError(Box::new(err))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MaterialSysConfig {
    pub max_count: usize,
}

impl MaterialSysConfig {
    pub fn max_count(mut self, max_count: usize) -> Self {
        self.max_count = max_count;
        self
    }
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

    fn handle(mut self, handle: usize) -> Self {
        self.handle = handle;
        self
    }

    fn reference_count(mut self, reference_count: usize) -> Self {
        self.reference_count = reference_count;
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

pub const DEFAULT_MATERIAL_NAME: &'static str = "default";

pub struct MaterialSystem<'a> {
    config: MaterialSysConfig,
    default_material: Material,
    default_material_2d: Material,
    registered_materials_hashmap: HashMap<String, MaterialRef>,
    registered_materials: Vec<Material>,
    material_shader_id: usize,
    material_locations: MaterialShaderUniformLocations,
    ui_shader_id: usize,
    ui_locations: MaterialUiUniformLocations,
    texture_system: Rc<RefCell<TextureSystem<'a>>>,
    frontend_renderer: Rc<RefCell<Renderer<'a>>>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
}

impl<'a> MaterialSystem<'a> {
    pub fn initialize(
        config: MaterialSysConfig,
        texture_system: Rc<RefCell<TextureSystem<'a>>>,
        frontend_renderer: Rc<RefCell<Renderer<'a>>>,
        resource_system: Rc<RefCell<ResourceSystem>>,
        shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    ) -> Result<Self> {
        if config.max_count == 0 {
            return Err(MaterialSysError::MaterialCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Material>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<String, MaterialRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Material::default());
        }

        Ok(Self {
            config,
            default_material: Material::default(),
            default_material_2d: Material::default(),
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

        material.diffuse_map.texture_handle = self
            .texture_system
            .borrow_mut()
            .acquire(DEFAULT_TEXTURE_NAME, true)?;
        material.diffuse_map.use_type = TextureUse::DiffuseMap;
        material.diffuse_map.texture_name = DEFAULT_TEXTURE_NAME.to_string();

        material.specular_map.texture_handle = self
            .texture_system
            .borrow_mut()
            .acquire(DEFAULT_TEXTURE_SPECULAR_NAME, true)?;
        material.specular_map.use_type = TextureUse::SpecularMap;
        material.specular_map.texture_name = DEFAULT_TEXTURE_SPECULAR_NAME.to_string();

        material.normal_map.texture_handle = self
            .texture_system
            .borrow_mut()
            .acquire(DEFAULT_TEXTURE_NORMAL_NAME, true)?;
        material.normal_map.use_type = TextureUse::NormalMap;
        material.normal_map.texture_name = DEFAULT_TEXTURE_NORMAL_NAME.to_string();

        material.shininess = 32.0;

        // I don't think I should create the shader in material system
        // should be handled somewhere else
        material.shader_id = self
            .shader_system
            .borrow_mut()
            .get_mut_shader_by_name(BUILTIN_SHADER_NAME_MATERIAL)?
            .id;
        //self.material_shader_id = material.shader_id;
        let maps = vec![
            &material.diffuse_map,
            &material.specular_map,
            &material.normal_map,
        ];
        material.internal_id = self
            .frontend_renderer
            .borrow()
            .acquire_shader_instance_resources(
                self.shader_system
                    .borrow_mut()
                    .get_mut_shader_by_name(BUILTIN_SHADER_NAME_MATERIAL)?,
                &maps,
            )? as usize;

        self.default_material = material;
        Ok(())
    }

    pub fn get_default_material(&self) -> Result<&Material> {
        Ok(&self.default_material)
    }

    pub fn get_material(&self, material_handle: MaterialHandle) -> Result<&Material> {
        Ok(&self.registered_materials[material_handle])
    }

    pub fn get_mut_material(&mut self, material_handle: MaterialHandle) -> Result<&mut Material> {
        Ok(&mut self.registered_materials[material_handle])
    }

    pub fn acquire(&mut self, config: &mut MaterialConfig) -> Result<(MaterialHandle, usize)> {
        if config.name == DEFAULT_MATERIAL_NAME || config.name.len() < 1 {
            // This might cause an issue with shader aquire resources not being called
            // TODO: I need to call shader aquire resources to get an material_instance id
            return Ok((self.default_material.id, self.default_material.internal_id));
        }

        let mut material_res = self.resource_system.borrow().load(
            &config.name,
            ResourceType::Material,
            ResourceFlags::empty(),
        )?;

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

        self.resource_system.borrow().unload(&mut material_res)?;

        Ok((material, shader_instance_id))
    }

    pub fn acquire_from_config(
        &mut self,
        config: &mut MaterialConfig,
    ) -> Result<(MaterialHandle, usize)> {
        if config.name == DEFAULT_MATERIAL_NAME {
            // This might cause an issue with shader aquire resources not being called
            // TODO: I need to call shader aquire resources to get an material_instance id
            return Ok((self.default_material.id, self.default_material.internal_id));
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
                if mat.id == INVALID_ID {
                    mat.id = i;
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
            self.registered_materials[mat_ref.handle] = material;

            let shader_sys = self.shader_system.borrow();

            let shader = shader_sys.get_shader_by_name(&config.shader_name)?;
            if self.material_shader_id == INVALID_ID
                && config.shader_name == BUILTIN_SHADER_NAME_MATERIAL
            {
                self.material_shader_id = shader.id;
                self.material_locations.projection =
                    shader_sys.get_uniform_index(shader, "projection")?;
                self.material_locations.view = shader_sys.get_uniform_index(shader, "view")?;
                self.material_locations.view_position =
                    shader_sys.get_uniform_index(shader, "view_position")?;
                self.material_locations.ambient_colour =
                    shader_sys.get_uniform_index(shader, "ambient_colour")?;

                self.material_locations.diffuse_colour =
                    shader_sys.get_uniform_index(shader, "diffuse_colour")?;
                self.material_locations.diffuse_texture =
                    shader_sys.get_uniform_index(shader, "diffuse_texture")?;
                self.material_locations.specular_texture =
                    shader_sys.get_uniform_index(shader, "specular_texture")?;

                self.material_locations.normal_texture =
                    shader_sys.get_uniform_index(shader, "normal_texture")?;
                self.material_locations.shininess =
                    shader_sys.get_uniform_index(shader, "shininess")?;
                self.material_locations.model = shader_sys.get_uniform_index(shader, "model")?;
                self.material_locations.render_mode =
                    shader_sys.get_uniform_index(shader, "mode")?;
            } else if self.ui_shader_id == INVALID_ID
                && config.shader_name == BUILTIN_SHADER_NAME_UI
            {
                self.ui_shader_id = shader.id;
                self.ui_locations.projection =
                    shader_sys.get_uniform_index(&shader, "projection")?;
                self.ui_locations.view = shader_sys.get_uniform_index(&shader, "view")?;
                self.ui_locations.diffuse_colour =
                    shader_sys.get_uniform_index(&shader, "diffuse_colour")?;
                self.ui_locations.diffuse_texture =
                    shader_sys.get_uniform_index(&shader, "diffuse_texture")?;
                self.ui_locations.model = shader_sys.get_uniform_index(&shader, "model")?;
            }
            if self.registered_materials[mat_ref.handle].generation == INVALID_ID {
                self.registered_materials[mat_ref.handle].generation = 0;
            } else {
                self.registered_materials[mat_ref.handle].generation += 1;
            }
            self.registered_materials[mat_ref.handle].id = mat_ref.handle;
        }

        // FIXME: changing internal id means nothing
        let mut material_instance_id = INVALID_ID;
        {
            let material = &self.registered_materials[mat_ref.handle];
            let maps = vec![
                &material.diffuse_map,
                &material.specular_map,
                &material.normal_map,
            ];
            material_instance_id = self
                .frontend_renderer
                .borrow()
                .acquire_shader_instance_resources(
                    self.shader_system
                        .borrow_mut()
                        .get_mut_shader_by_name(&config.shader_name)?,
                    &maps,
                )? as usize;
        }
        self.insert_hashmap(&config.name, &mat_ref)?;
        Ok((mat_ref.handle, material_instance_id))
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
            self.destroy_material(mat_ref.handle)?;
            self.registered_materials[mat_ref.handle] = Material::default();

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
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.projection,
                projection as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.view,
                view as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.view_position,
                view_positon as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.ambient_colour,
                ambient_colour as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.render_mode,
                &mode as *const _ as *const c_void,
            )?;
        } else if shader_id == self.ui_shader_id as u32 {
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.ui_locations.projection,
                projection as *const _ as *const c_void,
            )?;
            self.shader_system
                .borrow_mut()
                .set_uniform_by_index(self.ui_locations.view, view as *const _ as *const c_void)?;
        } else {
            return Err(MaterialSysError::UnrecognizedApplyGlobalCall {
                file: file!(),
                line: line!(),
            });
        }
        self.frontend_renderer.borrow().bind_globals_for_shader(
            self.shader_system
                .borrow_mut()
                .get_mut_shader_by_id(self.material_shader_id)?,
        )?;
        self.shader_system.borrow_mut().apply_globals()?;
        Ok(())
    }

    pub fn apply_instance(
        &self,
        material: &Material,
        material_instance_id: usize,
        model: &Matrix4,
    ) -> Result<()> {
        self.shader_system
            .borrow_mut()
            .bind_instance(material_instance_id)?;
        if material.shader_id == self.material_shader_id {
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.diffuse_colour,
                &material.diffuse_colour as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.diffuse_texture,
                &material.diffuse_map as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.specular_texture,
                &material.specular_map as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.normal_texture,
                &material.normal_map as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.shininess,
                &material.shininess as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.material_locations.model,
                model as *const _ as *const c_void,
            )?;
        } else if material.shader_id == self.ui_shader_id {
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.ui_locations.diffuse_colour,
                &material.diffuse_colour as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.ui_locations.diffuse_texture,
                &material.diffuse_map as *const _ as *const c_void,
            )?;
            self.shader_system.borrow_mut().set_uniform_by_index(
                self.ui_locations.model,
                model as *const _ as *const c_void,
            )?;
        } else {
            return Err(MaterialSysError::UnrecognizedApplyGlobalCall {
                file: file!(),
                line: line!(),
            });
        }
        self.shader_system.borrow_mut().apply_instance()?;
        Ok(())
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
            .get_shader_id(&config.shader_name)?;
        mat.name = config.name.clone();
        mat.diffuse_colour = config.diffuse_colour.clone();
        mat.shininess = config.shininess;

        mat.diffuse_map.texture_name = config.diffuse_map_name.clone();
        mat.diffuse_map.filter_minify = TextureFilter::Linear;
        mat.diffuse_map.filter_magnify = TextureFilter::Linear;
        mat.diffuse_map.repeat_u = TextureRepeat::Repeat;
        mat.diffuse_map.repeat_v = TextureRepeat::Repeat;
        mat.diffuse_map.repeat_w = TextureRepeat::Repeat;
        self.frontend_renderer
            .borrow()
            .acquire_texture_map_resources(&mut mat.diffuse_map)?;

        if config.diffuse_map_name.len() > 0 {
            mat.diffuse_map.use_type = TextureUse::DiffuseMap;
            mat.diffuse_map.texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(&config.diffuse_map_name, config.auto_release)?;
        } else {
            mat.diffuse_map.use_type = TextureUse::DiffuseMap;
            mat.diffuse_map.texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(DEFAULT_TEXTURE_NAME, true)?;
        }

        mat.specular_map.texture_name = config.specular_map_name.clone();
        mat.specular_map.filter_minify = TextureFilter::Linear;
        mat.specular_map.filter_magnify = TextureFilter::Linear;
        mat.specular_map.repeat_u = TextureRepeat::Repeat;
        mat.specular_map.repeat_v = TextureRepeat::Repeat;
        mat.specular_map.repeat_w = TextureRepeat::Repeat;
        self.frontend_renderer
            .borrow()
            .acquire_texture_map_resources(&mut mat.specular_map)?;

        if config.specular_map_name.len() > 0 {
            mat.specular_map.use_type = TextureUse::SpecularMap;
            let texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(&config.specular_map_name, config.auto_release)?;
            mat.specular_map.texture_handle = texture_handle;
        } else {
            mat.specular_map.use_type = TextureUse::SpecularMap;
            mat.specular_map.texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(DEFAULT_TEXTURE_SPECULAR_NAME, true)?;
        }

        mat.normal_map.texture_name = config.normal_map_name.clone();
        mat.normal_map.filter_minify = TextureFilter::Linear;
        mat.normal_map.filter_magnify = TextureFilter::Linear;
        mat.normal_map.repeat_u = TextureRepeat::Repeat;
        mat.normal_map.repeat_v = TextureRepeat::Repeat;
        mat.normal_map.repeat_w = TextureRepeat::Repeat;
        self.frontend_renderer
            .borrow()
            .acquire_texture_map_resources(&mut mat.normal_map)?;

        if config.normal_map_name.len() > 0 {
            mat.normal_map.use_type = TextureUse::NormalMap;
            let texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(&config.normal_map_name, config.auto_release)?;
            mat.normal_map.texture_handle = texture_handle;
        } else {
            mat.normal_map.use_type = TextureUse::NormalMap;
            mat.normal_map.texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(DEFAULT_TEXTURE_NORMAL_NAME, true)?;
        }
        Ok(mat)
    }

    pub fn destroy_material(&mut self, material_handle: MaterialHandle) -> Result<()> {
        let material = &mut self.registered_materials[material_handle];
        self.texture_system
            .borrow_mut()
            .release(&material.diffuse_map.texture_name)?;
        self.texture_system
            .borrow_mut()
            .release(&material.specular_map.texture_name)?;
        self.texture_system
            .borrow_mut()
            .release(&material.normal_map.texture_name)?;
        self.frontend_renderer
            .borrow()
            .release_texture_map_resources(&mut material.diffuse_map)?;
        self.frontend_renderer
            .borrow()
            .release_texture_map_resources(&mut material.specular_map)?;
        self.frontend_renderer
            .borrow()
            .release_texture_map_resources(&mut material.normal_map)?;
        if material.shader_id != INVALID_ID && material.internal_id != INVALID_ID {
            self.frontend_renderer
                .borrow()
                .release_shader_instance_resources(
                    self.shader_system
                        .borrow_mut()
                        .get_mut_shader_by_id(material.shader_id)?,
                    material.internal_id as u32,
                )?;
        }
        Ok(())
    }
}

impl<'a> Drop for MaterialSystem<'a> {
    fn drop(&mut self) {
        // mainly used to to destroy samplers
        for mat in self.registered_materials.iter_mut() {
            if mat.id == INVALID_ID {
                continue;
            }
            let _ = self
                .frontend_renderer
                .borrow()
                .release_texture_map_resources(&mut mat.diffuse_map);
            let _ = self
                .frontend_renderer
                .borrow()
                .release_texture_map_resources(&mut mat.specular_map);
            let _ = self
                .frontend_renderer
                .borrow()
                .release_texture_map_resources(&mut mat.normal_map);
        }
    }
}
