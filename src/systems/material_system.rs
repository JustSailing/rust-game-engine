use std::collections::HashMap;

use crate::application::{
    basic::{filesystem::FileHandleError, math::consts::INVALID_ID},
    renderer::renderer_types::{FrontendRendererError, Renderer},
    resources::resource_types::{
        Material, MaterialConfig, MaterialType, ResourceData, ResourceType, TextureUse,
    },
    systems::{
        resource_system::{ResourceSysError, ResourceSystem},
        texture_system::{TextureSysError, TextureSystem},
    },
};

use thiserror::Error;

const DEFAULT_MATERIAL_NAME: &'static str = "default";
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
    #[error("{source}\nmaterial system error:  texture system error in material sys {file} {line}")]
    TextureSystemError {
        source: TextureSysError,
        file: &'static str,
        line: u32,
    },
    #[error(
        "{source}\nmaterial system error:  frontend renderer error in material sys {file} {line}"
    )]
    RendererSystemError {
        source: FrontendRendererError,
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
    ResourceSystemError {
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

pub struct MaterialSystem<'a> {
    config: MaterialSysConfig,
    default_material: Material<'a>,
    registered_materials_hashmap: HashMap<String, MaterialRef>,
    registered_materials: Vec<Material<'a>>,
}

static mut MATERIAL_STATE: Option<MaterialSystem> = None;

impl<'a: 'static> MaterialSystem<'a> {
    pub fn initialize(config: MaterialSysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = MATERIAL_STATE {
                return Err(MaterialSysError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
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

        unsafe {
            MATERIAL_STATE = Some(MaterialSystem {
                config: config,
                default_material: Self::create_default_material()?,
                registered_materials_hashmap: registered_hash_map,
                registered_materials: registered_array,
            })
        }
        Ok(())
    }

    fn create_default_material() -> Result<Material<'a>> {
        let mut material = Material::default();
        material.name = String::from(DEFAULT_MATERIAL_NAME);
        material.material_type = MaterialType::World;
        material.diffuse_map.texture = Some(TextureSystem::get_default_texture().map_err(|e| {
            MaterialSysError::TextureSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?);

        material.diffuse_map.use_type = TextureUse::MapDiffuse;
        Renderer::create_material(&mut material).map_err(|e| {
            MaterialSysError::RendererSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        Ok(material)
    }

    pub fn get_default_material() -> Result<&'a mut Material<'a>> {
        unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                Ok(&mut state.default_material)
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    pub fn release(name: &str) -> Result<()> {
        if name == DEFAULT_MATERIAL_NAME {
            return Ok(());
        }
        let state = unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                state
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let mat_ref = match state.registered_materials_hashmap.get_mut(name) {
            Some(t) => t,
            None => {
                return Err(MaterialSysError::ReleaseTextureDoesNotExist {
                    name: name.to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if mat_ref.reference_count == 0 {
            println!("WARN tried to release a non-loaded texture.");
            return Ok(());
        }
        mat_ref.reference_count -= 1;
        if mat_ref.reference_count == 0 && mat_ref.auto_release {
            let mat = &state.registered_materials[mat_ref.handle];
            Renderer::destroy_material(mat).map_err(|e| MaterialSysError::RendererSystemError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

            Self::reset_material_at_index(mat_ref.handle)?;
            // don't think i need the 2 lines below
            mat_ref.handle = INVALID_ID;
            mat_ref.auto_release = false;

            state.registered_materials_hashmap.remove(name);
        }

        Ok(())
    }
    fn reset_material_at_index(index: usize) -> Result<()> {
        unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                state.registered_materials[index] = Material::default();
                return Ok(());
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
    }
    pub fn acquire(config: &mut MaterialConfig) -> Result<&'a mut Material<'a>> {
        let mut material_res =
            ResourceSystem::load(&config.name, ResourceType::Material).map_err(|e| {
                MaterialSysError::ResourceSystemError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?;

        let config = match material_res.data {
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
            ResourceData::MaterialResourceData(ref mut material_config) => material_config,
        };

        let mat = Self::acquire_from_config(config)?;

        ResourceSystem::unload(&mut material_res).map_err(|e| {
            MaterialSysError::ResourceSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        Ok(mat)
    }

    pub fn acquire_from_config(config: &mut MaterialConfig) -> Result<&'a mut Material<'a>> {
        let state = unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                state
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if config.name == DEFAULT_MATERIAL_NAME {
            return Ok(&mut state.default_material);
        }
        let mat_ref = match state.registered_materials_hashmap.get_mut(&config.name) {
            Some(m) => m,
            None => {
                let mat_ref = MaterialRef::default().auto_release(config.auto_release);
                state
                    .registered_materials_hashmap
                    .insert(config.name.clone(), mat_ref);
                state
                    .registered_materials_hashmap
                    .get_mut(&config.name)
                    .unwrap()
            }
        };
        if mat_ref.reference_count == 0 {
            mat_ref.auto_release = config.auto_release;
        }

        mat_ref.reference_count += 1;
        if mat_ref.handle == INVALID_ID {
            for (i, mat) in state.registered_materials.iter_mut().enumerate() {
                if mat.id == INVALID_ID {
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
            //let name = config.name.clone().as_str();
            state.registered_materials[mat_ref.handle] = Self::load_material(config)?;
            if state.registered_materials[mat_ref.handle].generation == INVALID_ID {
                state.registered_materials[mat_ref.handle].generation = INVALID_ID;
            } else {
                state.registered_materials[mat_ref.handle].generation += 1;
            }
            state.registered_materials[mat_ref.handle].id = mat_ref.handle;
        }

        Self::insert_hashmap(&config.name, mat_ref)?;
        Ok(&mut state.registered_materials[mat_ref.handle])
    }

    fn insert_hashmap(name: &String, mat_ref: &MaterialRef) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                state
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state
            .registered_materials_hashmap
            .insert(name.to_string(), *mat_ref);
        Ok(())
    }

    fn load_material(config: &mut MaterialConfig) -> Result<Material<'a>> {
        let mut mat = Material::default();
        mat.material_type = config.material_type;
        mat.name = config.name.clone();
        mat.diffuse_colour = config.diffuse_colour.clone();
        if config.diffuse_map_name.len() > 0 {
            mat.diffuse_map.use_type = TextureUse::MapDiffuse;
            let tex = TextureSystem::acquire(config.diffuse_map_name.clone(), config.auto_release)
                .map_err(|e| MaterialSysError::TextureSystemError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            mat.diffuse_map.texture = Some(tex);
        }

        let res = Renderer::create_material(&mut mat).map_err(|e| {
            MaterialSysError::RendererSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        });
        if res.is_err() {
            return Err(res.err().unwrap());
        }

        Ok(mat)
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = MATERIAL_STATE {
                for mat in state.registered_materials.iter() {
                    if mat.id != INVALID_ID {
                        Self::destroy_material(mat)?;
                    }
                }
                MATERIAL_STATE = None;
            } else {
                return Err(MaterialSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }

        Ok(())
    }

    pub fn destroy_material(material: &'a Material) -> Result<()> {
        TextureSystem::release(&material.name).map_err(|e| {
            MaterialSysError::TextureSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Renderer::destroy_material(&material).map_err(|e| {
            MaterialSysError::RendererSystemError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        Ok(())
    }
}
