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
        Renderer::create_texture(&pixels, &mut texture)?;

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
        for i in 0..total_size as usize - 3 {
            if v[i + 3] < 255 {
                transparancy = true;
                break;
            }
        }
        let mut texture = Texture::default()
            .has_transparency(transparancy)
            .width(width)
            .height(height)
            .channel_count(channel_count as u8)
            .generation(INVALID_ID);
        Renderer::create_texture(v.as_slice(), &mut texture)?;

        Ok(texture)
    }

    pub fn acquire(name: String, auto_release: bool) -> Result<&'a mut Texture> {
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
                return Err(Error::FailedToAcquireTexture(String::from(
                    "failed to acquire texture. texture system cannot hold anymore textures",
                ))
                .into());
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
                Self::destroy_default_texture()?;
                for texture in state.registered_textures.iter() {
                    if texture.id != INVALID_ID {
                        println!("WARN: did not free texture id: {}", texture.id);
                        Renderer::destroy_texture(texture)?;
                    }
                }
                TEXTURE_STATE = None;
                Ok(())
            } else {
                Err(Error::NotInitialized.into())
            }
        }
    }
}
