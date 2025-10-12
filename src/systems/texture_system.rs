use std::collections::HashMap;

use crate::application::{
    basic::math::consts::INVALID_ID,
    renderer::renderer_types::{RendererError, Renderer},
    resources::resource_types::{ResourceData, ResourceType, Texture},
    systems::resource_system::{ResourceSysError, ResourceSystem},
};

use image::ImageError;
use thiserror::Error;

#[derive(Clone, Copy)]
pub struct TextureSysConfig {
    pub max_count: usize,
}

const DEFAULT_TEXTURE_NAME: &'static str = "default";

#[derive(Clone, Copy)]
pub struct TextureRef {
    reference_count: usize,
    handle: usize,
    auto_release: bool,
}

impl TextureRef {
    pub fn auto_release(mut self, auto_release: bool) -> TextureRef {
        self.auto_release = auto_release;
        self
    }
}

impl Default for TextureRef {
    fn default() -> Self {
        Self {
            reference_count: 0,
            handle: INVALID_ID,
            auto_release: false,
        }
    }
}

#[derive(Error, Debug)]
pub enum TextureSysError {
    #[error("texture system error: system already initialized {file} {line}")]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("texture system error: system not initialized {file} {line}")]
    NotInitialized { file: &'static str, line: u32 },
    #[error("texture system error: system already shutdown {file} {line}")]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("texture system error: config given with max count less than 1 {file} {line}")]
    TextureCountZero { file: &'static str, line: u32 },
    #[error("texture system error: releasing a texture that does not exist {file} {line}")]
    ReleaseTextureDoesNotExist { file: &'static str, line: u32 },
    #[error("texture system error: failed to acquire texture: {} {} {}", name,  file!(), line!())]
    FailedToAcquireTexture {
        name: String,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\ntexture system error: frontend renderer error : {file} {line}")]
    RendererSysError {
        source: RendererError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\ntexture system error: image error from image crate {file} {line}")]
    ImageError {
        source: ImageError,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\ntexture system error: resource system error {file} {line}")]
    ResourceSysError {
        source: ResourceSysError,
        file: &'static str,
        line: u32,
    },
    #[error("texture system error: wrong resource data type {ty} {file} {line}")]
    WrongResourceDataType {
        ty: String,
        file: &'static str,
        line: u32,
    },
}

type Result<T> = std::result::Result<T, TextureSysError>;

pub struct TextureSystem {
    config: TextureSysConfig,
    default_texture: Texture,
    registered_textures: Vec<Texture>,
    registered_textures_hashmap: HashMap<String, TextureRef>,
    //TODO: add free list of indices when we release textures
}

static mut TEXTURE_STATE: Option<TextureSystem> = None;

impl<'a> TextureSystem {
    pub fn initialize(config: TextureSysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = TEXTURE_STATE {
                return Err(TextureSysError::AlreadyInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
        if config.max_count == 0 {
            return Err(TextureSysError::TextureCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Texture>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<String, TextureRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Texture::default());
        }

        unsafe {
            TEXTURE_STATE = Some(TextureSystem {
                config: config,
                default_texture: Self::create_default_texture()?,
                registered_textures: registered_array,
                registered_textures_hashmap: registered_hash_map,
            });
        }

        Ok(())
    }

    fn create_default_texture() -> Result<Texture> {
        const TEX_DIMENSION: u8 = 255;
        const CHANNELS: u8 = 4;
        const PIXEL_COUNT: usize = TEX_DIMENSION as usize * TEX_DIMENSION as usize;
        let mut pixels = [255u8; PIXEL_COUNT * CHANNELS as usize];
        let width = CHANNELS;
        let height: usize = pixels.len() / CHANNELS as usize;
        for row in (0..height).step_by(4) {
            for col in (0..width as usize).step_by(4) {
                let index = (row * width as usize) + col;

                pixels[index + 0] = 0;
                pixels[index + 1] = 0;
            }
        }
        let mut texture = Texture::default()
            .has_transparency(false)
            .width(TEX_DIMENSION as u32)
            .height(TEX_DIMENSION as u32)
            .channel_count(CHANNELS as u8)
            .generation(INVALID_ID);
        Renderer::create_texture(&pixels, &mut texture).map_err(|e| {
            TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;

        Ok(texture)
    }

    fn destroy_default_texture() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                Renderer::destroy_texture(&state.default_texture).map_err(|e| {
                    TextureSysError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })
            } else {
                Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                })
            }
        }
    }

    fn load_texture(name: &str) -> Result<Texture> {
        let mut img_res = ResourceSystem::load(name, ResourceType::Image).map_err(|e| {
            TextureSysError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        let data = match img_res.data {
            ResourceData::Unknown => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "Unknown".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::BinaryResourceData(_) => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "BinaryResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ImageResourceData(ref image_resource_data) => image_resource_data,
            ResourceData::MaterialResourceData(_) => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "MaterialResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let total_size = data.width * data.height * data.channel_count as u32;
        let mut transparancy = false;
        for i in 0..total_size as usize - 3 {
            if data.pixels[i + 3] < 255 {
                transparancy = true;
                break;
            }
        }
        let mut texture = Texture::default()
            .has_transparency(transparancy)
            .width(data.width)
            .height(data.height)
            .channel_count(data.channel_count)
            .generation(INVALID_ID);
        Renderer::create_texture(data.pixels.as_slice(), &mut texture).map_err(|e| {
            TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        ResourceSystem::unload(&mut img_res).map_err(|e| TextureSysError::ResourceSysError {
            source: e,
            file: file!(),
            line: line!(),
        })?;
        Ok(texture)
    }

    pub fn acquire(name: String, auto_release: bool) -> Result<&'a mut Texture> {
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if name == DEFAULT_TEXTURE_NAME {
            println!(
                "WARN: texture acquire was called with default texture name. Use get_default_texture for 'default'"
            );
            return Ok(&mut state.default_texture);
        }
        let tex_ref = match state.registered_textures_hashmap.get_mut(&name) {
            Some(t) => t,
            None => {
                let tex_ref = TextureRef::default().auto_release(auto_release);
                let n = String::from(name.clone());
                state.registered_textures_hashmap.insert(n.clone(), tex_ref);
                state.registered_textures_hashmap.get_mut(&n).unwrap()
            }
        };
        if tex_ref.reference_count == 0 {
            tex_ref.auto_release = auto_release;
        }
        tex_ref.reference_count += 1;
        if tex_ref.handle == INVALID_ID {
            for tuple in state.registered_textures.iter().enumerate() {
                if tuple.1.id == INVALID_ID {
                    tex_ref.handle = tuple.0;
                    break;
                }
            }
            if tex_ref.handle == INVALID_ID {
                return Err(TextureSysError::FailedToAcquireTexture {
                    name: "failed to acquire texture. texture system cannot hold anymore textures"
                        .to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            let n = String::from(&name);
            state.registered_textures[tex_ref.handle] = Self::load_texture(&n)?;
            state.registered_textures[tex_ref.handle].id = tex_ref.handle;
        }
        Self::insert_hashmap(&name, *tex_ref)?;
        Ok(&mut state.registered_textures[tex_ref.handle])
    }

    pub fn release(name: &str) -> Result<()> {
        if name == DEFAULT_TEXTURE_NAME {
            return Ok(());
        }
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let tex_ref = match state.registered_textures_hashmap.get_mut(name) {
            Some(t) => t,
            None => {
                return Err(TextureSysError::ReleaseTextureDoesNotExist {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if tex_ref.reference_count == 0 {
            println!("WARN tried to release a non-loaded texture.");
            return Ok(());
        }
        tex_ref.reference_count -= 1;
        if tex_ref.reference_count == 0 && tex_ref.auto_release {
            let t = &state.registered_textures[tex_ref.handle];
            Renderer::destroy_texture(t).map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
            state.registered_textures[tex_ref.handle] = Texture::default();
            // don't think i need the 2 lines below
            tex_ref.handle = INVALID_ID;
            tex_ref.auto_release = false;

            state.registered_textures_hashmap.remove(name);
        }

        Ok(())
    }

    pub fn get_default_texture() -> Result<&'a mut Texture> {
        unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                Ok(&mut state.default_texture)
            } else {
                return Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        }
    }

    fn insert_hashmap(name: &str, texture_ref: TextureRef) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state
            .registered_textures_hashmap
            .insert(name.to_string(), texture_ref);
        Ok(())
    }

    pub fn shutdown() -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(TextureSysError::NotInitialized {
                    file: file!(),
                    line: line!(),
                });
            }
        };
        Self::destroy_default_texture()?;
        for texture in state.registered_textures.iter() {
            if texture.id != INVALID_ID {
                state
                    .registered_textures_hashmap
                    .iter()
                    .find_map(|(key, value)| {
                        if value.handle == texture.id && !value.auto_release {
                            println!("WARN: did not free texture name: {}", key);
                            Some(())
                        } else {
                            None
                        }
                    });

                Renderer::destroy_texture(texture).map_err(|e| TextureSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            }
        }
        unsafe {
            TEXTURE_STATE = None;
        }
        Ok(())
    }
}
