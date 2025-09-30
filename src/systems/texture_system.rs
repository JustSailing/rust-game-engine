use std::{collections::HashMap, fmt};

use crate::application::{
    basic::math::consts::INVALID_ID, renderer::renderer_types::Renderer,
    resources::resource_types::Texture,
};

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

#[derive(Debug)]
pub enum Error {
    AlreadyInitialized,
    NotInitialized,
    AlreadyShutdown,
    TextureCountZero,
    ReleaseTextureDoesNotExist,
    FailedToAcquireTexture(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AlreadyInitialized => write!(
                f,
                "Texture System State Already Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::NotInitialized => write!(
                f,
                "Texture System State Not Initialized {}  {}",
                file!(),
                line!()
            ),
            Error::AlreadyShutdown => write!(
                f,
                "Texture System State Already Shutdown {}  {}",
                file!(),
                line!()
            ),
            Error::TextureCountZero => write!(
                f,
                "Texture System State Texture Count Zero {}  {}",
                file!(),
                line!()
            ),
            Error::FailedToAcquireTexture(e) => write!(f, "{e} {}  {}", file!(), line!()),
            Error::ReleaseTextureDoesNotExist => write!(
                f,
                "Tried to release texture that does not exist {}  {}",
                file!(),
                line!()
            ),
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

pub struct TextureSystem {
    config: TextureSysConfig,
    default_texture: Texture,
    registered_textures: Vec<Texture>,
    registered_textures_hashmap: HashMap<String, TextureRef>,
    free_index: usize,
    //TODO: add free list of indices when we release textures
}

static mut TEXTURE_STATE: Option<TextureSystem> = None;

impl<'a> TextureSystem {
    pub fn initialize(config: TextureSysConfig) -> Result<()> {
        unsafe {
            if let Some(ref _state) = TEXTURE_STATE {
                return Err(Error::AlreadyInitialized.into());
            }
        }
        if config.max_count == 0 {
            return Err(Error::TextureCountZero.into());
        }

        let mut registered_array = Vec::<Texture>::with_capacity(config.max_count);
        let registered_hash_map = HashMap::<String, TextureRef>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            let mut tf: Texture = unsafe { std::mem::zeroed() };
            tf.id = INVALID_ID;
            tf.generation = INVALID_ID;
            registered_array.push(tf);
        }

        unsafe {
            TEXTURE_STATE = Some(TextureSystem {
                config: config,
                default_texture: Self::create_default_texture()?,
                registered_textures: registered_array,
                registered_textures_hashmap: registered_hash_map,
                free_index: 0,
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

        let mut texture = Renderer::create_texture(
            DEFAULT_TEXTURE_NAME,
            TEX_DIMENSION as u32,
            TEX_DIMENSION as u32,
            CHANNELS as u32,
            &pixels,
            false,
        )?;
        texture.generation = INVALID_ID;

        Ok(texture)
    }

    fn destroy_default_texture() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                Renderer::destroy_texture(&state.default_texture)
            } else {
                Err(Error::NotInitialized.into())
            }
        }
    }

    fn load_texture(name: &str) -> Result<Texture> {
        let file_path = format!("assets/textures/{}.{}", name, "jpg");
        let data = image::open(file_path)?.to_rgba8();
        let width = data.width();
        let height = data.height();
        let v = data.into_raw();
        let channel_count: u32 = 4;
        let total_size = width * height * channel_count;
        let mut transparancy = false;
        for i in (3..total_size as usize).step_by(channel_count as usize) {
            if v[i] < 255 {
                transparancy = true;
                break;
            }
        }
        let mut texture = Renderer::create_texture(
            name,
            width as u32,
            height as u32,
            channel_count,
            v.as_slice(),
            transparancy,
        )?;
        texture.generation = INVALID_ID;

        Ok(texture)
    }

    pub fn acquire(name: &str, auto_release: bool) -> Result<&mut Texture> {
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
            }
        };
        if name == DEFAULT_TEXTURE_NAME {
            println!(
                "WARN: texture acquire was called with default texture name. Use get_default_texture for 'default'"
            );
            return Ok(&mut state.default_texture);
        }
        let tex_ref = match state.registered_textures_hashmap.get_mut(name) {
            Some(t) => t,
            None => {
                let tex_ref = TextureRef {
                    reference_count: 0,
                    handle: INVALID_ID,
                    auto_release,
                };
                state
                    .registered_textures_hashmap
                    .insert(String::from(name), tex_ref);
                state.registered_textures_hashmap.get_mut(name).unwrap()
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
                return Err(Error::FailedToAcquireTexture(String::from(
                    "failed to acquire texture. texture system cannot hold anymore textures",
                ))
                .into());
            }
            state.registered_textures[tex_ref.handle] = Self::load_texture(name)?;
            state.registered_textures[tex_ref.handle].id = tex_ref.handle;
        }
        Self::insert_hashmap(name, *tex_ref)?;
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
                return Err(Error::NotInitialized.into());
            }
        };

        let tex_ref = match state.registered_textures_hashmap.get_mut(name) {
            Some(t) => t,
            None => return Err(Error::ReleaseTextureDoesNotExist.into()),
        };
        if tex_ref.reference_count == 0 {
            println!("WARN tried to release a non-loaded texture.");
            return Ok(());
        }
        tex_ref.reference_count -= 1;
        if tex_ref.reference_count == 0 && tex_ref.auto_release {
            let t = &state.registered_textures[tex_ref.handle];
            Renderer::destroy_texture(t)?;
            unsafe { state.registered_textures[tex_ref.handle] = std::mem::zeroed() };
            state.registered_textures[tex_ref.handle].id = INVALID_ID;
            state.registered_textures[tex_ref.handle].generation = INVALID_ID;
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
                return Err(Error::NotInitialized.into());
            }
        }
    }

    fn insert_hashmap(name: &str, texture_ref: TextureRef) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                state
            } else {
                return Err(Error::NotInitialized.into());
            }
        };
        state
            .registered_textures_hashmap
            .insert(name.to_string(), texture_ref);
        Ok(())
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref mut state) = TEXTURE_STATE {
                for texture in state.registered_textures.iter() {
                    if texture.id != INVALID_ID {
                        Renderer::destroy_texture(texture)?;
                    }
                }
                Renderer::destroy_texture(&state.default_texture)?;
                TEXTURE_STATE = None;
                Ok(())
            } else {
                Err(Error::NotInitialized.into())
            }
        }
    }
}
