use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::application::{
    basic::math::consts::INVALID_ID,
    renderer::renderer_types::{Renderer, RendererError},
    resources::resource_types::{ResourceData, ResourceType, Texture},
    systems::resource_system::{ResourceSysError, ResourceSystem},
};

use image::ImageError;
use thiserror::Error;

#[derive(Clone, Copy)]
pub struct TextureSysConfig {
    pub max_count: usize,
}

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

pub const DEFAULT_TEXTURE_NAME: &'static str = "default";
pub const DEFAULT_TEXTURE_SPECULAR_NAME: &'static str = "default_specular";

pub struct TextureSystem {
    config: TextureSysConfig,
    default_texture: Rc<RefCell<Texture>>,
    default_specular_texture: Rc<RefCell<Texture>>,
    registered_textures: Vec<Rc<RefCell<Texture>>>,
    registered_textures_hashmap: HashMap<String, TextureRef>,
    frontend_renderer: Rc<RefCell<Renderer>>,
    resource_system: Rc<RefCell<ResourceSystem>>,
    //TODO: add free list of indices when we release textures
}

impl TextureSystem {
    pub fn initialize(
        config: TextureSysConfig,
        frontend_renderer: Rc<RefCell<Renderer>>,
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
            config: config,
            default_texture: Rc::new(RefCell::new(Texture::default())),
            default_specular_texture: Rc::new(RefCell::new(Texture::default())),
            registered_textures: registered_array,
            registered_textures_hashmap: registered_hash_map,
            frontend_renderer: frontend_renderer,
            resource_system: resource_system,
        })
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
            .has_transparency(false)
            .width(TEX_DIMENSION as u32)
            .height(TEX_DIMENSION as u32)
            .channel_count(CHANNELS as u8)
            .generation(INVALID_ID);
        self.frontend_renderer
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
            .has_transparency(false)
            .width(16)
            .height(16)
            .channel_count(4)
            .generation(INVALID_ID);
        self.frontend_renderer
            .borrow()
            .create_texture("default_specular", &spec_pixels, &mut specular_texture)
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        self.default_specular_texture.replace(specular_texture);
        Ok(())
    }

    fn destroy_default_textures(&self) -> Result<()> {
        self.frontend_renderer
            .borrow()
            .destroy_texture(&self.default_texture.borrow())
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        self.frontend_renderer
            .borrow()
            .destroy_texture(&self.default_specular_texture.borrow())
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })
    }

    fn load_texture(&self, name: &str, path_type: &str) -> Result<Texture> {
        let mut img_res = self
            .resource_system
            .borrow()
            .load(name, path_type, ResourceType::Image)
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

        self.frontend_renderer
            .borrow()
            .create_texture(name, data.pixels.as_slice(), &mut texture)
            .map_err(|e| TextureSysError::RendererSysError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
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

    pub fn acquire(
        &mut self,
        name: String,
        path_type: &str,
        auto_release: bool,
    ) -> Result<Rc<RefCell<Texture>>> {
        if name == DEFAULT_TEXTURE_NAME {
            println!(
                "WARN: texture acquire was called with default texture name. Use get_default_texture for 'default'"
            );
            return Ok(Rc::clone(&self.default_texture));
        }
        let mut texture_ref: TextureRef = TextureRef::default();
        {
            let tex_ref = match self.registered_textures_hashmap.get_mut(&name) {
                Some(t) => t,
                None => {
                    let tex_ref = TextureRef::default().auto_release(auto_release);
                    let n = String::from(name.clone());
                    self.registered_textures_hashmap.insert(n.clone(), tex_ref);
                    self.registered_textures_hashmap.get_mut(&n).unwrap()
                }
            };
            if tex_ref.reference_count == 0 {
                tex_ref.auto_release = auto_release;
            }
            tex_ref.reference_count += 1;
            if tex_ref.handle == INVALID_ID {
                for tuple in self.registered_textures.iter().enumerate() {
                    if tuple.1.borrow().id == INVALID_ID {
                        tex_ref.handle = tuple.0;
                        break;
                    }
                }
                if tex_ref.handle == INVALID_ID {
                    return Err(TextureSysError::FailedToAcquireTexture {
                        name:
                            "failed to acquire texture. texture system cannot hold anymore textures"
                                .to_string(),
                        file: file!(),
                        line: line!(),
                    });
                }
            }
            texture_ref = *tex_ref;
        }
        self.register_texture(&name, path_type, &texture_ref)?;
        Ok(Rc::clone(&self.registered_textures[texture_ref.handle]))
    }

    pub fn register_texture(
        &mut self,
        name: &String,
        path_type: &str,
        tex_ref: &TextureRef,
    ) -> Result<()> {
        self.registered_textures[tex_ref.handle].replace(self.load_texture(&name, path_type)?);
        self.registered_textures[tex_ref.handle].borrow_mut().id = tex_ref.handle;
        self.registered_textures_hashmap
            .insert(name.clone(), *tex_ref);
        Ok(())
    }

    pub fn release(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_TEXTURE_NAME || name == DEFAULT_TEXTURE_SPECULAR_NAME {
            return Ok(());
        }

        let mut tex_ref = match self.registered_textures_hashmap.get_mut(name) {
            Some(t) => *t,
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
            let t = &self.registered_textures[tex_ref.handle];
            self.frontend_renderer
                .borrow()
                .destroy_texture(&t.borrow())
                .map_err(|e| TextureSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.registered_textures[tex_ref.handle].borrow_mut().id = INVALID_ID;
            // don't think i need the 2 lines below
            tex_ref.handle = INVALID_ID;
            tex_ref.auto_release = false;
        }

        if tex_ref.reference_count == 0 {
            self.registered_textures_hashmap.remove(name);
        } else {
            self.registered_textures_hashmap
                .insert(name.to_string(), tex_ref);
        }

        Ok(())
    }

    pub fn release_by_id(&mut self, id: usize) -> Result<()> {
        if id == INVALID_ID {
            // warn here only texture with invalid id would be the default texture
            return Ok(());
        }

        let mut name = String::default();
        let mut tex_ref = TextureRef::default();
        {
            let mut option = self
                .registered_textures_hashmap
                .iter()
                .find_map(|(name, tf)| {
                    if tf.handle == id {
                        return Some((name, tf));
                    } else {
                        None
                    }
                });
            if option.is_none() {
                // not sure if i should error out
                return Ok(());
            }
            if let Some(ref mut opt) = option {
                name = opt.0.clone();
                tex_ref = *opt.1;
            } else {
                return Ok(());
            };
        }

        if tex_ref.reference_count == 0 {
            println!("WARN tried to release a non-loaded texture.");
            return Ok(());
        }
        tex_ref.reference_count -= 1;
        if tex_ref.reference_count == 0 && tex_ref.auto_release {
            self.frontend_renderer
                .borrow()
                .destroy_texture(&self.registered_textures[tex_ref.handle].borrow())
                .map_err(|e| TextureSysError::RendererSysError {
                    source: e,
                    file: file!(),
                    line: line!(),
                })?;
            self.registered_textures[tex_ref.handle].borrow_mut().id = INVALID_ID;
            // don't think i need the 2 lines below
            tex_ref.handle = INVALID_ID;
            tex_ref.auto_release = false;
        }
        if tex_ref.reference_count == 0 {
            self.registered_textures_hashmap.remove(&name);
        } else {
            self.registered_textures_hashmap.insert(name, tex_ref);
        }

        Ok(())
    }

    pub fn get_default_texture(&self) -> Result<Rc<RefCell<Texture>>> {
        Ok(Rc::clone(&self.default_texture))
    }

    pub fn get_default_specular_texture(&self) -> Result<Rc<RefCell<Texture>>> {
        Ok(Rc::clone(&self.default_specular_texture))
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

                let _ = self
                    .frontend_renderer
                    .borrow()
                    .destroy_texture(&texture.borrow());
            }
        }
    }
}
