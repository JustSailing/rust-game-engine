use crate::application::{
    basic::{filesystem::FileHandleError, math::consts::INVALID_ID},
    resources::{
        loaders::{binary_loader, image_loader, material_loader},
        resource_types::{Resource, ResourceType},
    },
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResourceSysError {
    #[error("resource system error: system already initialized {} {}", file!(), line!())]
    AlreadyInitialized,
    #[error("resource system error: system not initialized {} {}", file!(), line!())]
    NotInitialized,
    #[error("resource system error: system already shutdown {} {}", file!(), line!())]
    AlreadyShutdown,
    #[error("resource system error: config provide has a max count less than 1 {} {}", file!(), line!())]
    MaxLoaderCountZero,
    #[error("resource system error: loader already exists {} {}", file!(), line!())]
    LoaderAlreadyExists,
    #[error("resource system error: custom loader already exists {} {}", file!(), line!())]
    CustomLoaderAlreadyExists,
    #[error("resource system error: registered loaders is full. change max count in sys config {} {}", file!(), line!())]
    RegisteredLoadersFull,
    #[error("resource system error: loading resourse loader: {name} {} {}", file!(), line!())]
    ResourceLoadError { name: String },
    #[error("resource system error: loading resourse loader: {name} {} {}", file!(), line!())]
    ResourceUnloadError { name: String },
    #[error("resource system error: resource id is invalid {} {}", file!(), line!())]
    ResourceIdInvalid,
    #[error("resource system error: image loader error {} {}", file!(), line!())]
    ImageLoaderError,
    #[error("{source}\nresource system error: image loader error when opening texture file failed {} {}", file!(), line!())]
    ImageError {
        #[from]
        source: image::ImageError,
    },
    #[error("{source}\nresource system error: opening file failed {} {}", file!(), line!())]
    FileError {
        #[from]
        source: FileHandleError,
    },
}

type Result<T> = std::result::Result<T, ResourceSysError>;

pub struct ResourceSysConfig {
    pub max_loader_count: usize,
    pub asset_base_path: String,
}

#[derive(Debug, Clone)]
pub struct ResourceLoader {
    id: usize,
    res_type: ResourceType,
    custom_type: Option<String>,
    path_type: String,
    load: fn(&str, &str) -> Result<Resource>,
    unload: fn(&mut Resource) -> Result<()>,
}

impl ResourceLoader {
    pub fn new_image_loader() -> ResourceLoader {
        ResourceLoader {
            id: INVALID_ID,
            res_type: ResourceType::Image,
            custom_type: None,
            path_type: "textures".to_string(),
            load: image_loader::ImageLoader::load,
            unload: image_loader::ImageLoader::unload,
        }
    }
    pub fn new_material_loader() -> ResourceLoader {
        ResourceLoader {
            id: INVALID_ID,
            res_type: ResourceType::Material,
            custom_type: None,
            path_type: "materials".to_string(),
            load: material_loader::MaterialLoader::load,
            unload: material_loader::MaterialLoader::unload,
        }
    }
    pub fn new_binary_loader() -> ResourceLoader {
        ResourceLoader {
            id: INVALID_ID,
            res_type: ResourceType::Binary,
            custom_type: None,
            path_type: "".to_string(),
            load: binary_loader::BinaryLoader::load,
            unload: binary_loader::BinaryLoader::unload,
        }
    }
}

pub struct ResourceSystem {
    config: ResourceSysConfig,
    registered_loaders: Vec<Option<ResourceLoader>>,
}

static mut RESOURCE_STATE: Option<ResourceSystem> = None;

impl ResourceSystem {
    pub fn initialize(config: ResourceSysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = RESOURCE_STATE {
                return Err(ResourceSysError::AlreadyInitialized);
            }
        }
        if config.max_loader_count < 1 {
            return Err(ResourceSysError::MaxLoaderCountZero);
        }

        let mut registered_loaders =
            Vec::<Option<ResourceLoader>>::with_capacity(config.max_loader_count);
        for _ in 0..config.max_loader_count {
            registered_loaders.push(Default::default());
        }
        // auto register image loader and material loader
        registered_loaders[0] = Some(ResourceLoader::new_image_loader());
        registered_loaders[0].as_mut().unwrap().id = 0;
        registered_loaders[1] = Some(ResourceLoader::new_material_loader());
        registered_loaders[1].as_mut().unwrap().id = 1;
        registered_loaders[2] = Some(ResourceLoader::new_binary_loader());
        registered_loaders[2].as_mut().unwrap().id = 2;
        unsafe {
            RESOURCE_STATE = Some(ResourceSystem {
                config: config,
                registered_loaders: registered_loaders,
            })
        }
        Ok(())
    }

    pub fn register_loader(loader: ResourceLoader) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RESOURCE_STATE {
                state
            } else {
                return Err(ResourceSysError::NotInitialized);
            }
        };
        for ld in state.registered_loaders.iter() {
            if let Some(l) = ld {
                if loader.res_type != ResourceType::Custom && loader.res_type == l.res_type {
                    return Err(ResourceSysError::LoaderAlreadyExists);
                } else if let Some(ref ld_cus_typ) = l.custom_type
                    && let Some(ref cus_type) = loader.custom_type
                    && cus_type == ld_cus_typ
                {
                    return Err(ResourceSysError::CustomLoaderAlreadyExists);
                }
            }
        }

        let new_id = state
            .registered_loaders
            .iter_mut()
            .enumerate()
            .find_map(|(i, ld)| {
                if let Some(l) = ld
                    && l.id == INVALID_ID
                {
                    Some(i)
                } else {
                    None
                }
            });

        if new_id.is_none() {
            return Err(ResourceSysError::RegisteredLoadersFull);
        }
        state.registered_loaders[new_id.unwrap()] = Some(loader);
        state.registered_loaders[new_id.unwrap()]
            .as_mut()
            .unwrap()
            .id = new_id.unwrap();
        Ok(())
    }

    pub fn load(name: &str, res_typ: ResourceType) -> Result<Resource> {
        let state = unsafe {
            if let Some(ref mut state) = RESOURCE_STATE {
                state
            } else {
                return Err(ResourceSysError::NotInitialized);
            }
        };
        let res = state.registered_loaders.iter().find_map(|ld| {
            if let Some(l) = ld
                && l.id != INVALID_ID
                && l.res_type == res_typ
            {
                let mut res = match (l.load)(name, &l.path_type) {
                    Ok(r) => r,
                    Err(_) => return None,
                };
                res.loader_id = l.id;
                Some(res)
            } else {
                None
            }
        });
        if res.is_none() {
            return Err(ResourceSysError::ResourceLoadError {
                name: name.to_string(),
            });
        }
        return Ok(res.unwrap());
    }

    pub fn unload(resouce: &mut Resource) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = RESOURCE_STATE {
                state
            } else {
                return Err(ResourceSysError::NotInitialized);
            }
        };
        if resouce.loader_id == INVALID_ID {
            return Err(ResourceSysError::ResourceIdInvalid);
        }
        if let Some(ref ld) = state.registered_loaders[resouce.loader_id] {
            return (ld.unload)(resouce);
        } else {
            return Err(ResourceSysError::ResourceUnloadError {
                name: resouce.name.clone(),
            });
        }
    }

    pub fn base_path() -> Result<String> {
        unsafe {
            if let Some(ref mut state) = RESOURCE_STATE {
                return Ok(state.config.asset_base_path.clone());
            } else {
                return Err(ResourceSysError::NotInitialized);
            }
        }
    }
}
