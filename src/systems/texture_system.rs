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
            config,
            default_texture: Rc::new(RefCell::new(Texture::default())),
            default_specular_texture: Rc::new(RefCell::new(Texture::default())),
            default_normal_texture: Rc::new(RefCell::new(Texture::default())),
            registered_textures: registered_array,
            registered_textures_hashmap: registered_hash_map,
            frontend_renderer,
            resource_system,
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
            .channel_count(CHANNELS)
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

        let mut normal_pixels = [0u8; 16 * 16 * 4];
        for i in (0..16 * 16).step_by(4) {
            normal_pixels[i + 0] = 128;
            normal_pixels[i + 1] = 128;
            normal_pixels[i + 2] = 255;
            normal_pixels[i + 3] = 255;
        }

        let mut normal_texture = Texture::default()
            .has_transparency(false)
            .width(16)
            .height(16)
            .channel_count(4)
            .generation(INVALID_ID);
        self.frontend_renderer
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
            })?;
        self.frontend_renderer
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
            .has_transparency(transparency)
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

    pub fn acquire(&mut self, name: String, auto_release: bool) -> Result<Rc<RefCell<Texture>>> {
        if name == DEFAULT_TEXTURE_NAME {
            // ... (default texture check remains the same) ...
            return Ok(Rc::clone(&self.default_texture));
        }

        // --- STEP 1: Find a free slot (PRE-CALCULATION) ---
        // This is done before map interaction to avoid initial borrow conflicts.
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

        // --- STEP 2: Handle Map Entry and Release Borrow ---
        let mut current_ref = self
            .registered_textures_hashmap
            .remove(&name) // 👈 REMOVE the existing entry (releasing the borrow)
            .unwrap_or_else(|| {
                // If entry didn't exist, create the new TextureRef
                TextureRef {
                    reference_count: 0,
                    handle: free_handle,
                    auto_release,
                }
            });

        // --- STEP 3: Handle System Full Error ---
        if current_ref.reference_count == 0 && current_ref.handle == INVALID_ID {
            // We removed it in Step 2, so no need to remove again.
            return Err(TextureSysError::FailedToAcquireTexture {
                name: format!(
                    "failed to acquire texture '{}'. texture system cannot hold anymore textures",
                    name
                ),
                file: file!(),
                line: line!(),
            });
        }

        // --- STEP 4: Update Reference Count and Load if necessary ---
        let load_needed = current_ref.reference_count == 0;

        current_ref.reference_count += 1;
        current_ref.auto_release = auto_release;

        // --- STEP 5: Re-insert entry to save state (CRITICAL BORROW RELEASE POINT) ---
        // The `current_ref` is now a temporary, owned struct.
        // We insert it back, which creates a *new* mutable borrow of the HashMap,
        // but the borrow on `current_ref` ends immediately.
        self.registered_textures_hashmap
            .insert(name.clone(), current_ref);

        // Now, the HashMap is stable and the previous mutable borrow is finished.
        // We can safely proceed to access other parts of `self`.

        if load_needed {
            // Load the texture (requires internal borrows of renderer/resource system)
            self.registered_textures[current_ref.handle].replace(self.load_texture(&name)?);
            self.registered_textures[current_ref.handle].borrow_mut().id = current_ref.handle;
        }

        // --- STEP 6: Return the reference ---
        Ok(Rc::clone(&self.registered_textures[current_ref.handle]))
    }

    pub fn release(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_TEXTURE_NAME
            || name == DEFAULT_TEXTURE_SPECULAR_NAME
            || name == DEFAULT_TEXTURE_NORMAL_NAME
        {
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
            // don't think I need the 2 lines below
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
                // FIXME: some how im getting allot of could not release textures
                // still better than validation error
                // not sure if I should error out
                // INFO: I think this is fixed for now
                // Removed deletion of material textures to the texture system Drop rather than the
                // materieal system or the geometry system
                println!("could not release texture by id {}", id);
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
            // don't think I need the 2 lines below
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

                let _ = self
                    .frontend_renderer
                    .borrow()
                    .destroy_texture(&texture.borrow());
            }
        }
    }
}
