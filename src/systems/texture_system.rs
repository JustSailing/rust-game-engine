use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::application::{
    basic::math::consts::INVALID_ID,
    renderer::frontend_renderer::{Renderer, RendererError},
    resources::resource_types::{ResourceData, ResourceType, Texture, TextureData, TextureFlags},
    systems::resource_system::{ResourceSysError, ResourceSystem},
};

use image::ImageError;
use thiserror::Error;

#[derive(Clone, Copy)]
pub struct TextureSysConfig {
    pub max_count: usize,
}

#[derive(Clone, Copy, Debug)]
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
    #[error("texture system error: operation failed: {ty} {file} {line}")]
    OperationFailed {
        ty: String,
        file: &'static str,
        line: u32,
    },
}

type Result<T> = std::result::Result<T, TextureSysError>;

pub const DEFAULT_TEXTURE_NAME: &'static str = "default";
pub const DEFAULT_TEXTURE_SPECULAR_NAME: &'static str = "default_specular";
pub const DEFAULT_TEXTURE_NORMAL_NAME: &'static str = "default_normal";

pub struct TextureSystem {
    config: TextureSysConfig,
    default_texture: Rc<RefCell<Texture>>,
    default_specular_texture: Rc<RefCell<Texture>>,
    default_normal_texture: Rc<RefCell<Texture>>,
    registered_textures: Vec<Rc<RefCell<Texture>>>,
    registered_textures_hashmap: HashMap<String, TextureRef>,
    frontend_renderer: Option<Rc<RefCell<Renderer>>>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    //TODO: add free list of indices when we release textures
}

impl TextureSystem {
    pub fn initialize(
        config: TextureSysConfig,
        resource_system: Rc<RefCell<ResourceSystem>>,
    ) -> Result<Self> {
        if config.max_count == 0 {
            return Err(TextureSysError::TextureCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_array = Vec::<Rc<RefCell<Texture>>>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<String, TextureRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_array.push(Rc::new(RefCell::new(Texture::default())));
        }

        Ok(Self {
            config,
            default_texture: Rc::new(RefCell::new(Texture::default())),
            default_specular_texture: Rc::new(RefCell::new(Texture::default())),
            default_normal_texture: Rc::new(RefCell::new(Texture::default())),
            registered_textures: registered_array,
            registered_textures_hashmap: registered_hash_map,
            frontend_renderer: None,
            resource_system,
        })
    }

    pub fn set_renderer(&mut self, frontend_renderer: Rc<RefCell<Renderer>>) {
        self.frontend_renderer = Some(frontend_renderer);
    }

    pub fn create_default_textures(&mut self) -> Result<()> {
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
            .width(TEX_DIMENSION as u32)
            .height(TEX_DIMENSION as u32)
            .channel_count(CHANNELS)
            .generation(INVALID_ID)
            .name(DEFAULT_TEXTURE_NAME.to_string());

        let renderer = if let Some(ref renderer) = self.frontend_renderer {
            renderer
        } else {
            return Err(TextureSysError::ReleaseTextureDoesNotExist {
                file: file!(),
                line: line!(),
            });
        };

        renderer
            .borrow()
            .create_texture("default", &pixels, &mut texture)
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        self.default_texture.replace(texture);

        let spec_pixels = [0u8; 16 * 16 * 4];
        let mut specular_texture = Texture::default()
            .width(16)
            .height(16)
            .channel_count(4)
            .generation(INVALID_ID)
            .name(DEFAULT_TEXTURE_SPECULAR_NAME.to_string());

        renderer
            .borrow()
            .create_texture("default_specular", &spec_pixels, &mut specular_texture)
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        self.default_specular_texture.replace(specular_texture);

        let mut normal_pixels = [0u8; 16 * 16 * 4];
        for i in (0..16 * 16).step_by(4) {
            normal_pixels[i + 0] = 128;
            normal_pixels[i + 1] = 128;
            normal_pixels[i + 2] = 255;
            normal_pixels[i + 3] = 255;
        }

        let mut normal_texture = Texture::default()
            .width(16)
            .height(16)
            .channel_count(4)
            .generation(INVALID_ID)
            .name(DEFAULT_TEXTURE_NORMAL_NAME.to_string());

        renderer
            .borrow()
            .create_texture("default_normal", &normal_pixels, &mut normal_texture)
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        self.default_normal_texture.replace(normal_texture);

        Ok(())
    }

    fn destroy_default_textures(&self) -> Result<()> {
        let renderer = if let Some(ref renderer) = self.frontend_renderer {
            renderer
        } else {
            return Err(TextureSysError::ReleaseTextureDoesNotExist {
                file: file!(),
                line: line!(),
            });
        };
        renderer
            .borrow()
            .destroy_texture(&self.default_texture.borrow())
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        renderer
            .borrow()
            .destroy_texture(&self.default_specular_texture.borrow())
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        renderer
            .borrow()
            .destroy_texture(&self.default_normal_texture.borrow())
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    fn load_texture(&self, name: &str) -> Result<Texture> {
        let mut img_res = self
            .resource_system
            .borrow()
            .load(name, ResourceType::Image)
            .map_err(|e| TextureSysError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let data = match img_res.data {
            ResourceData::ImageResourceData(ref image_resource_data) => image_resource_data,
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
            ResourceData::ShaderResourceData(_) => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "ShaderResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MaterialResourceData(_) => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "MaterialResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MeshResourceData(_) => {
                return Err(TextureSysError::WrongResourceDataType {
                    ty: "MeshResourceData".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let total_size = data.width * data.height * data.channel_count as u32;
        let mut transparency = false;
        for i in 0..total_size as usize - 3 {
            if data.pixels[i + 3] < 255 {
                transparency = true;
                break;
            }
        }
        let mut texture = Texture::default()
            .transparency_flag(transparency)
            .width(data.width)
            .height(data.height)
            .channel_count(data.channel_count)
            .generation(INVALID_ID)
            .name(name.to_string());

        if let Some(ref renderer) = self.frontend_renderer {
            renderer
                .borrow()
                .create_texture(name, data.pixels.as_slice(), &mut texture)
                .map_err(|e| TextureSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }
        self.resource_system
            .borrow()
            .unload(&mut img_res)
            .map_err(|e| TextureSysError::ResourceSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        Ok(texture)
    }

    pub fn acquire(&mut self, name: &String, auto_release: bool) -> Result<Rc<RefCell<Texture>>> {
        if name == DEFAULT_TEXTURE_NAME {
            // NOTE: probably worry about normal and specular texuters here as well
            return Ok(Rc::clone(&self.default_texture));
        } else if name == DEFAULT_TEXTURE_SPECULAR_NAME {
            return Ok(Rc::clone(&self.default_specular_texture));
        } else if name == DEFAULT_TEXTURE_NORMAL_NAME {
            return Ok(Rc::clone(&self.default_normal_texture));
        }

        let mut id = INVALID_ID as u32;
        self.process_texture_reference(name, 1, auto_release, false, &mut id)?;
        Ok(Rc::clone(&self.registered_textures[id as usize]))
    }

    pub fn acquire_writable(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        channel_count: u8,
        has_transparency: bool,
    ) -> Result<Rc<RefCell<Texture>>> {
        let mut id = INVALID_ID as u32;

        self.process_texture_reference(name, 1, false, true, &mut id)?;

        let texture = Rc::clone(&self.registered_textures[id as usize]);
        texture.borrow_mut().width = width;
        texture.borrow_mut().height = height;
        texture.borrow_mut().channel_count = channel_count;
        texture.borrow_mut().generation = INVALID_ID;
        if has_transparency {
            texture
                .borrow_mut()
                .flags
                .insert(TextureFlags::Transparency)
        }
        texture.borrow_mut().flags.insert(TextureFlags::Writable);
        texture.borrow_mut().internal_data = TextureData::default();
        if let Some(ref renderer) = self.frontend_renderer {
            renderer
                .borrow()
                .create_writable_texture(&mut texture.borrow_mut())
                .map_err(|e| TextureSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
        }
        Ok(texture)
    }

    pub fn wrap_internal(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
        channel_count: u8,
        has_transparency: bool,
        is_writable: bool,
        register_texture: bool,
        internal_data: TextureData,
    ) -> Result<Rc<RefCell<Texture>>> {
        let mut id = INVALID_ID as u32;
        let texture = if register_texture {
            self.process_texture_reference(name, 1, false, true, &mut id)?;
            Rc::clone(&self.registered_textures[id as usize])
        } else {
            Rc::new(RefCell::new(Texture::default()))
        };

        texture.borrow_mut().id = id as usize;
        texture.borrow_mut().width = width;
        texture.borrow_mut().height = height;
        texture.borrow_mut().channel_count = channel_count;
        texture.borrow_mut().generation = INVALID_ID;
        if has_transparency {
            texture
                .borrow_mut()
                .flags
                .insert(TextureFlags::Transparency)
        }
        if is_writable {
            texture.borrow_mut().flags.insert(TextureFlags::Writable)
        }
        texture.borrow_mut().flags.insert(TextureFlags::Wrapped);
        texture.borrow_mut().internal_data = internal_data;
        Ok(texture)
    }

    pub fn set_internal(texture: &mut Texture, internal_data: TextureData) -> Result<()> {
        texture.internal_data = internal_data;
        texture.generation += 1;
        Ok(())
    }

    pub fn resize(
        &self,
        texture: &mut Texture,
        width: u32,
        height: u32,
        regenerate_internal_data: bool,
    ) -> Result<()> {
        if !texture.flags.contains(TextureFlags::Writable) {
            println!(
                "[WARN] texture system error: resize should not be called on textures that are not writable"
            );
            return Ok(());
        }
        texture.width = width;
        texture.height = height;
        if !texture.flags.contains(TextureFlags::Wrapped) && regenerate_internal_data {
            if let Some(ref renderer) = self.frontend_renderer {
                renderer
                    .borrow()
                    .resize_texture(texture, width, height)
                    .map_err(|e| TextureSysError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
            }
        }
        Ok(())
    }

    pub fn release(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_TEXTURE_NAME
            || name == DEFAULT_TEXTURE_SPECULAR_NAME
            || name == DEFAULT_TEXTURE_NORMAL_NAME
        {
            return Ok(());
        }

        let mut id = INVALID_ID as u32;
        self.process_texture_reference(name, -1, false, false, &mut id)?;
        Ok(())
    }

    fn process_texture_reference(
        &mut self,
        name: &str,
        loadable: i8,
        auto_release: bool,
        skip_load: bool,
        id: &mut u32,
    ) -> Result<()> {
        let free_handle = self
            .registered_textures
            .iter()
            .enumerate()
            .find_map(|(i, texture_rc)| {
                if texture_rc.borrow().id == INVALID_ID {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(INVALID_ID);

        if free_handle == INVALID_ID {
            return Err(TextureSysError::FailedToAcquireTexture {
                name: format!("texture system: max amount of textures reached {}", name),
                file: file!(),
                line: line!(),
            });
        }

        let mut tex_ref = self
            .registered_textures_hashmap
            .remove(name)
            .unwrap_or_else(|| {
                // If entry didn't exist, create the new TextureRef
                TextureRef {
                    reference_count: 0,
                    handle: INVALID_ID,
                    auto_release,
                }
            });

        // checks if valid release
        if tex_ref.reference_count == 0 && loadable < 0 {
            if tex_ref.handle != INVALID_ID {
                println!(
                    "[WARN] tried to release a texture with reference count already zero: {}",
                    name
                );
            } else {
                println!(
                    "[WARN] tried to release a texture that does not exist: {}",
                    name
                )
            }
            // NOTE: it's okay for now just to return with Ok value might change in the future
            return Ok(());
        }

        tex_ref.handle = free_handle;
        let load_needed = tex_ref.reference_count == 0;

        if loadable > 0 {
            tex_ref.reference_count += 1;
        } else {
            tex_ref.reference_count -= 1;
        }

        // release
        if loadable < 0 {
            if tex_ref.reference_count == 0 && tex_ref.auto_release {
                let texture = self.registered_textures[tex_ref.handle].borrow_mut();
                let renderer = if let Some(ref renderer) = self.frontend_renderer {
                    renderer
                } else {
                    return Err(TextureSysError::OperationFailed{ ty: "texture system: frontend_renderer not initialized in texture system value None".to_string(), file: file!(), line: line!()});
                };
                renderer.borrow().destroy_texture(&texture).map_err(|e| {
                    TextureSysError::RendererSysError {
                        source: e,
                        file: file!(),
                        line: line!(),
                    }
                })?;
            }
        } else {
            // new texture
            if skip_load {
                // nothing happens here
            } else {
                if load_needed {
                    self.registered_textures[tex_ref.handle].replace(self.load_texture(name)?);
                    self.registered_textures[tex_ref.handle].borrow_mut().id = tex_ref.handle;
                }
            }
        }
        *id = tex_ref.handle as u32;
        self.registered_textures_hashmap
            .insert(name.to_string(), tex_ref);

        Ok(())
    }

    pub fn get_default_texture(&self) -> Result<Rc<RefCell<Texture>>> {
        Ok(Rc::clone(&self.default_texture))
    }

    pub fn get_default_specular_texture(&self) -> Result<Rc<RefCell<Texture>>> {
        Ok(Rc::clone(&self.default_specular_texture))
    }

    pub fn get_default_normal_texture(&self) -> Result<Rc<RefCell<Texture>>> {
        Ok(Rc::clone(&self.default_normal_texture))
    }
}

impl Drop for TextureSystem {
    fn drop(&mut self) {
        let _ = self.destroy_default_textures();
        for texture in self.registered_textures.iter_mut() {
            if texture.borrow().id != INVALID_ID {
                self.registered_textures_hashmap
                    .iter()
                    .find_map(|(key, value)| {
                        if value.handle == texture.borrow().id && !value.auto_release {
                            println!("WARN: did not free texture name: {}", key);
                            Some(())
                        } else {
                            None
                        }
                    });

                let _ = if let Some(ref renderer) = self.frontend_renderer {
                    let _ = renderer.borrow().destroy_texture(&texture.borrow());
                };
            }
        }
    }
}
