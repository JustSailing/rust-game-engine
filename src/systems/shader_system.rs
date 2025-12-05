use std::{cell::RefCell, collections::HashMap, ffi::c_void, rc::Rc, u8};

use crate::{
    basic::math::consts::INVALID_ID,
    renderer::{
        frontend_renderer::{Renderer, RendererError},
        vulkan::vulkan_backend::VulkanShader,
    },
    resources::resource_types::{
        ShaderAttributeType, ShaderConfig, ShaderScope, ShaderUniformConfig, ShaderUniformType,
        TextureMap,
    },
    systems::texture_system::{TextureSysError, TextureSystem, DEFAULT_TEXTURE_NAME},
};

use thiserror::Error;

type Result<T> = std::result::Result<T, ShaderSysError>;

#[derive(Debug, Default, Clone, Copy)]
pub struct ShaderSysConfig {
    pub max_shader_count: u16,
    pub max_uniform_count: u16,
    pub max_global_textures: u16,
    pub max_instance_textures: u16,
}

impl ShaderSysConfig {
    pub fn max_shader_count(mut self, max_shader_count: u16) -> Self {
        self.max_shader_count = max_shader_count;
        self
    }

    pub fn max_uniform_count(mut self, max_uniform_count: u16) -> Self {
        self.max_uniform_count = max_uniform_count;
        self
    }

    pub fn max_global_textures(mut self, max_global_textures: u16) -> Self {
        self.max_global_textures = max_global_textures;
        self
    }

    pub fn max_instance_textures(mut self, max_instance_textures: u16) -> Self {
        self.max_instance_textures = max_instance_textures;
        self
    }
}

pub enum ShaderState {
    NotCreated,
    Uninitialized,
    Initialized,
}

impl Default for ShaderState {
    fn default() -> Self {
        Self::NotCreated
    }
}

pub struct ShaderRef {
    handle: usize,
    reference_count: usize,
    auto_release: bool,
}

impl Default for ShaderRef {
    fn default() -> Self {
        Self {
            handle: INVALID_ID,
            reference_count: Default::default(),
            auto_release: Default::default(),
        }
    }
}

impl ShaderRef {
    fn handle(mut self, handle: usize) -> Self {
        self.handle = handle;
        self
    }

    fn reference_count(mut self, reference_count: usize) -> Self {
        self.reference_count = reference_count;
        self
    }

    fn auto_release(mut self, auto_release: bool) -> Self {
        self.auto_release = auto_release;
        self
    }
}

#[derive(Clone, Copy)]
pub struct ShaderUniform {
    pub offset: usize,
    pub location: u16,
    pub index: u16,
    pub set_index: u8,
    pub shader_scope: ShaderScope,
    pub uniform_type: ShaderUniformType,
    pub size: usize,
}

impl Default for ShaderUniform {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            location: Default::default(),
            index: Default::default(),
            set_index: Default::default(),
            shader_scope: ShaderScope::Unknown,
            uniform_type: ShaderUniformType::Unknown,
            size: Default::default(),
        }
    }
}

pub struct ShaderAttribute {
    pub name: String,
    pub attribute_type: ShaderAttributeType,
    pub size: usize,
}

impl Default for ShaderAttribute {
    fn default() -> Self {
        Self {
            name: Default::default(),
            attribute_type: ShaderAttributeType::Unknown,
            size: Default::default(),
        }
    }
}
#[repr(C)]
pub enum ShaderInternalData<'a> {
    Vulkan(VulkanShader<'a>),
    Unknown,
}

impl<'a> Default for ShaderInternalData<'a> {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct Range {
    pub offset: usize,
    pub size: usize,
}

#[repr(C)]
pub struct Shader<'a> {
    pub id: usize,
    name: String,
    pub use_instances: bool,
    use_locals: bool,
    pub required_ubo_alignment: usize,
    pub global_ubo_size: usize,
    pub global_ubo_stride: usize,
    pub global_ubo_offset: usize,
    pub ubo_size: usize,
    pub ubo_stride: usize,
    pub global_texture_maps: Vec<TextureMap>,
    instance_texture_count: usize,
    pub bound_instance_id: usize,
    pub bound_ubo_offset: usize,
    bound_scope: ShaderScope,
    uniform_lookup: HashMap<String, ShaderUniform>,
    pub uniforms: Vec<ShaderUniform>,
    pub attributes: Vec<ShaderAttribute>,
    state: ShaderState,
    push_constant_size: usize,
    push_constant_stride: usize,
    push_constant_offset: usize,
    pub push_constant_range_count: usize,
    pub push_constant_ranges: [Range; 32],
    pub attribute_stride: usize,
    pub internal_data: ShaderInternalData<'a>,
}

impl<'a> Default for Shader<'a> {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            name: Default::default(),
            use_instances: Default::default(),
            use_locals: Default::default(),
            required_ubo_alignment: Default::default(),
            global_ubo_size: Default::default(),
            global_ubo_stride: Default::default(),
            global_ubo_offset: Default::default(),
            ubo_size: Default::default(),
            ubo_stride: Default::default(),
            push_constant_size: Default::default(),
            push_constant_stride: Default::default(),
            global_texture_maps: Default::default(),
            instance_texture_count: Default::default(),
            bound_instance_id: Default::default(),
            bound_ubo_offset: Default::default(),
            uniform_lookup: Default::default(),
            uniforms: Default::default(),
            attributes: Default::default(),
            state: Default::default(),
            push_constant_ranges: Default::default(),
            attribute_stride: Default::default(),
            internal_data: Default::default(),
            push_constant_range_count: Default::default(),
            bound_scope: ShaderScope::Unknown,
            push_constant_offset: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ShaderSysError {
    #[error("shader system error: system already initialized {file} {line}")]
    AlreadyInitialized { file: &'static str, line: u32 },
    #[error("shader system error: system not initialized {file} {line}")]
    NotInitialized { file: &'static str, line: u32 },
    #[error("shader system error: system already shutdown {file} {line}")]
    AlreadyShutdown { file: &'static str, line: u32 },
    #[error("shader system error: config given with max count less than 1 {file} {line}")]
    ShaderCountZero { file: &'static str, line: u32 },
    #[error(
        "shader system error: the amount of shaders registered is at the maximum {file} {line}"
    )]
    ShaderSystemFull { file: &'static str, line: u32 },
    #[error("shader system error: adding sampler error: {reason} {file} {line}")]
    ShaderAddSamplerError {
        reason: String,
        file: &'static str,
        line: u32,
    },
    #[error("shader system error: the max amount of global textures was reached {file} {line}")]
    MaxGlobalTexturesReached { file: &'static str, line: u32 },
    #[error("shader system error: the max amount of instance textures was reached {file} {line}")]
    MaxInstanceTexturesReached { file: &'static str, line: u32 },
    #[error("shader system error: the max amount of uniform count was reached {file} {line}")]
    MaxUniformCountReached { file: &'static str, line: u32 },
    #[error("shader system error: uniform type unknown {file} {line}")]
    ShaderUniformTypeUnknown { file: &'static str, line: u32 },
    #[error(
        "shader system error: uniform use of shader scope local was not allowed {file} {line}"
    )]
    ShaderUniformUseLocal { file: &'static str, line: u32 },
    #[error("shader system error: given shader id that was invalid {file} {line}")]
    ShaderInvalidId { file: &'static str, line: u32 },
    #[error("shader system error: uniform set called with no shader in use {file} {line}")]
    NoShaderInUse { file: &'static str, line: u32 },
    #[error("shader system error: shader given is invalid or not registered {name} {file} {line}")]
    ShaderInvalid {
        name: String,
        file: &'static str,
        line: u32,
    },
    #[error("shader system error: frontend renderer error: {0}")]
    RendererSysError(Box<RendererError>),
    #[error("shader system error: texture_system error: {0}")]
    TextureSysError(#[from] TextureSysError),
}
impl From<RendererError> for ShaderSysError {
    fn from(err: RendererError) -> Self {
        ShaderSysError::RendererSysError(Box::new(err))
    }
}

pub struct ShaderSystem<'a> {
    config: ShaderSysConfig,
    lookup: HashMap<String, ShaderRef>,
    current_shader_id: usize,
    registered_shaders: Vec<Shader<'a>>,
    frontend_renderer: Rc<RefCell<Renderer<'a>>>,
    texture_system: Rc<RefCell<TextureSystem<'a>>>,
}

impl<'a> ShaderSystem<'a> {
    pub fn initialize(
        config: ShaderSysConfig,
        frontend_renderer: Rc<RefCell<Renderer<'a>>>,
        texture_system: Rc<RefCell<TextureSystem<'a>>>,
    ) -> Result<Self> {
        if config.max_shader_count < 512 || config.max_shader_count == 0 {
            return Err(ShaderSysError::ShaderCountZero {
                file: file!(),
                line: line!(),
            });
        }

        let mut registered_shaders = Vec::with_capacity(config.max_shader_count as usize);
        for _ in 0..config.max_shader_count as usize {
            registered_shaders.push(Shader::default());
        }

        let lookup = HashMap::<String, ShaderRef>::with_capacity(config.max_shader_count as usize);
        Ok(Self {
            config,
            lookup,
            current_shader_id: INVALID_ID,
            registered_shaders,
            frontend_renderer,
            texture_system,
        })
    }

    pub fn shutdown(&mut self) -> Result<()> {
        let size = self.registered_shaders.len();
        for i in 0..size {
            let shader_id = self.registered_shaders[i].id;
            if shader_id == INVALID_ID {
                continue;
            }
            self.destroy_shader(i)?;
        }
        Ok(())
    }

    pub fn create(&mut self, shader_config: &ShaderConfig) -> Result<usize> {
        let id = self
            .registered_shaders
            .iter()
            .enumerate()
            .find_map(|(i, shader)| {
                if shader.id == INVALID_ID {
                    Some(i)
                } else {
                    None
                }
            });

        if id.is_none() {
            return Err(ShaderSysError::ShaderSystemFull {
                file: file!(),
                line: line!(),
            });
        }

        self.registered_shaders[id.unwrap()].id = id.unwrap();
        self.registered_shaders[id.unwrap()].name = shader_config.name.clone();
        self.registered_shaders[id.unwrap()].state = ShaderState::NotCreated;
        self.registered_shaders[id.unwrap()].use_instances = shader_config.use_instances;
        self.registered_shaders[id.unwrap()].use_locals = shader_config.use_locals;
        self.registered_shaders[id.unwrap()].push_constant_range_count = 0;
        self.registered_shaders[id.unwrap()].bound_instance_id = INVALID_ID;
        self.registered_shaders[id.unwrap()].push_constant_size = 0;
        self.registered_shaders[id.unwrap()].attribute_stride = 0;

        self.registered_shaders[id.unwrap()].attributes = shader_config
            .attributes
            .iter()
            .map(|attr_config| {
                let attrib = ShaderAttribute {
                    name: attr_config.name.clone(),
                    attribute_type: attr_config.attribute_type.clone(),
                    size: attr_config.size,
                };
                self.registered_shaders[id.unwrap()].attribute_stride += attr_config.size;
                attrib
            })
            .collect::<Vec<ShaderAttribute>>();

        for config in shader_config.uniforms.iter() {
            match config.uniform_type {
                ShaderUniformType::Sampler => {
                    self.add_sampler(config, &shader_config, id.unwrap())?
                }
                ShaderUniformType::Unknown => {
                    return Err(ShaderSysError::ShaderUniformTypeUnknown {
                        file: file!(),
                        line: line!(),
                    });
                }
                _ => self.add_uniform(
                    id.unwrap(),
                    &config.name,
                    config.size as u32,
                    config.uniform_type,
                    config.scope,
                    0,
                    false,
                )?,
            };
        }

        self.frontend_renderer.borrow_mut().create_shader(
            &mut self.registered_shaders[id.unwrap()],
            &shader_config.renderpass_name,
            shader_config.stages.len() as u8,
            &shader_config.stage_filenames,
            &shader_config.stages,
        )?;

        self.lookup.insert(
            shader_config.name.clone(),
            ShaderRef {
                handle: self.registered_shaders[id.unwrap()].id,
                reference_count: 1,
                auto_release: true,
            },
        );
        Ok(id.unwrap())
    }

    pub fn get_mut_shader_by_name(&mut self, name: &str) -> Result<&mut Shader<'a>> {
        let id = self.get_shader_id(name)?;
        Ok(&mut self.registered_shaders[id])
    }

    pub fn get_shader_by_name(&self, name: &str) -> Result<&Shader<'_>> {
        let id = self.get_shader_id(name)?;
        Ok(&self.registered_shaders[id])
    }

    pub fn get_shader_id(&self, name: &str) -> Result<usize> {
        match self.lookup.get(name) {
            Some(r) => {
                if r.handle == INVALID_ID {
                    Err(ShaderSysError::ShaderInvalid {
                        name: name.to_string(),
                        file: file!(),
                        line: line!(),
                    })
                } else {
                    Ok(r.handle)
                }
            }
            None => Err(ShaderSysError::ShaderInvalid {
                name: name.to_string(),
                file: file!(),
                line: line!(),
            }),
        }
    }

    pub fn get_mut_shader_by_id(&mut self, id: usize) -> Result<&mut Shader<'a>> {
        if self.registered_shaders[id].id == INVALID_ID {
            return Err(ShaderSysError::ShaderInvalidId {
                file: file!(),
                line: line!(),
            });
        }
        Ok(&mut self.registered_shaders[id])
    }

    pub fn get_shader_by_id(&self, id: usize) -> Result<&Shader<'a>> {
        if self.registered_shaders[id].id == INVALID_ID {
            return Err(ShaderSysError::ShaderInvalidId {
                file: file!(),
                line: line!(),
            });
        }
        Ok(&self.registered_shaders[id])
    }

    pub fn destroy_shader(&mut self, shader_handle: usize) -> Result<()> {
        self.frontend_renderer
            .borrow()
            .destroy_shader(&mut self.registered_shaders[shader_handle])?;
        self.registered_shaders[shader_handle] = Shader::default();
        Ok(())
    }

    pub fn use_shader(&mut self, name: &str) -> Result<()> {
        let id = self.get_shader_id(name)?;
        self.use_by_id(id)
    }

    pub fn use_by_id(&mut self, id: usize) -> Result<()> {
        if self.current_shader_id != id {
            self.current_shader_id = id;
            let shader = self.get_shader_by_id(id)?;
            self.frontend_renderer.borrow().use_shader(shader)?;
            self.frontend_renderer
                .borrow_mut()
                .bind_globals_for_shader(&mut self.registered_shaders[id])?;
        }
        Ok(())
    }

    pub fn get_uniform_index(&self, shader: &Shader, name: &str) -> Result<u16> {
        let uniform = shader.uniform_lookup.get(name);
        let u = match uniform {
            Some(ref u) => u,
            None => {
                return Err(ShaderSysError::ShaderInvalid {
                    name: name.to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        };
        Ok(u.index)
    }

    pub fn set_uniform(&mut self, name: &str, value: *const c_void) -> Result<()> {
        if self.current_shader_id == INVALID_ID {
            return Err(ShaderSysError::NoShaderInUse {
                file: file!(),
                line: line!(),
            });
        }
        let shader = &self.registered_shaders[self.current_shader_id];
        let index = self.get_uniform_index(shader, name)?;
        self.set_uniform_by_index(index, value)
    }

    pub fn set_uniform_by_index(&mut self, index: u16, value: *const c_void) -> Result<()> {
        let mut shader = &mut self.registered_shaders[self.current_shader_id];
        let uniform = shader.uniforms[index as usize];

        if shader.bound_scope != uniform.shader_scope {
            if uniform.shader_scope == ShaderScope::Global {
                self.frontend_renderer
                    .borrow()
                    .bind_globals_for_shader(&mut shader)?;
            } else if uniform.shader_scope == ShaderScope::Instance {
                self.frontend_renderer
                    .borrow()
                    .bind_instance_for_shader(&mut shader)?;
            }
            shader.bound_scope = uniform.shader_scope;
        }

        self.frontend_renderer.borrow().set_uniform_for_shader(
            &mut shader,
            index as usize,
            value,
        )?;
        Ok(())
    }

    pub fn set_sampler_by_index(&mut self, index: u16, value: *const c_void) -> Result<()> {
        self.set_uniform_by_index(index, value)
    }

    pub fn apply_globals(&mut self) -> Result<()> {
        self.frontend_renderer
            .borrow()
            .apply_globals_for_shader(&mut self.registered_shaders[self.current_shader_id])?;
        Ok(())
    }

    pub fn apply_instance(&mut self) -> Result<()> {
        self.frontend_renderer
            .borrow()
            .apply_instance_for_shader(&mut self.registered_shaders[self.current_shader_id])?;
        Ok(())
    }

    pub fn bind_instance(&mut self, instance_id: usize) -> Result<()> {
        self.registered_shaders[self.current_shader_id].bound_instance_id = instance_id;

        self.frontend_renderer
            .borrow()
            .bind_instance_for_shader(&mut self.registered_shaders[self.current_shader_id])?;
        Ok(())
    }

    fn add_sampler(
        &mut self,
        config: &ShaderUniformConfig,
        shader_config: &ShaderConfig,
        shader_id: usize,
    ) -> Result<()> {
        if config.scope == ShaderScope::Instance && !shader_config.use_instances {
            return Err(ShaderSysError::ShaderAddSamplerError {
                reason: "cannot use instance sampler for a shader that does not use instances"
                    .to_string(),
                file: file!(),
                line: line!(),
            });
        }

        if config.scope == ShaderScope::Local {
            return Err(ShaderSysError::ShaderAddSamplerError {
                reason: "cannot add sampler at local scope".to_string(),
                file: file!(),
                line: line!(),
            });
        }

        if self
            .lookup
            .get(&config.name)
            .is_some_and(|shader_ref| shader_ref.handle != INVALID_ID)
        {
            let reason = format!(
                "uniform by name {} already exists for shader {}",
                config.name, shader_config.name
            );
            return Err(ShaderSysError::ShaderAddSamplerError {
                reason,
                file: file!(),
                line: line!(),
            });
        }

        let mut location = 0;
        if config.scope == ShaderScope::Global {
            let global_texture_count = self.registered_shaders[shader_id].global_texture_maps.len();
            if global_texture_count + 1 > self.config.max_global_textures as usize {
                return Err(ShaderSysError::MaxGlobalTexturesReached {
                    file: file!(),
                    line: line!(),
                });
            }
            location = global_texture_count;
            // NOTE: create default texture maps here
            let mut texture_map = TextureMap::default();
            self.frontend_renderer
                .borrow_mut()
                .acquire_texture_map_resources(&mut texture_map)?;
            texture_map.texture_handle = self
                .texture_system
                .borrow_mut()
                .acquire(DEFAULT_TEXTURE_NAME, true)?;

            self.registered_shaders[shader_id]
                .global_texture_maps
                .push(texture_map);
        } else {
            if self.registered_shaders[shader_id].instance_texture_count + 1
                > self.config.max_instance_textures as usize
            {
                return Err(ShaderSysError::MaxInstanceTexturesReached {
                    file: file!(),
                    line: line!(),
                });
            }
            location = self.registered_shaders[shader_id].instance_texture_count;
            self.registered_shaders[shader_id].instance_texture_count += 1;
        }

        self.add_uniform(
            shader_id,
            &config.name,
            0,
            config.uniform_type,
            config.scope,
            location as u32,
            true,
        )
    }

    fn add_uniform(
        &mut self,
        shader_id: usize,
        uniform_name: &str,
        size: u32,
        uniform_type: ShaderUniformType,
        scope: ShaderScope,
        set_location: u32,
        is_sampler: bool,
    ) -> Result<()> {
        let uniform_count = self.registered_shaders[shader_id].uniforms.len();
        if uniform_count + 1 > self.config.max_uniform_count as usize {
            return Err(ShaderSysError::MaxUniformCountReached {
                file: file!(),
                line: line!(),
            });
        }

        let mut entry = ShaderUniform::default();
        entry.index = uniform_count as u16;
        entry.shader_scope = scope;
        entry.uniform_type = uniform_type;
        let is_global = ShaderScope::Global == scope;
        if is_sampler {
            entry.location = set_location as u16;
        } else {
            entry.location = entry.index;
        }
        if scope != ShaderScope::Local {
            entry.set_index = scope as u8;
            entry.offset = if is_sampler {
                0
            } else {
                if is_global {
                    self.registered_shaders[shader_id].global_ubo_size
                } else {
                    self.registered_shaders[shader_id].ubo_size
                }
            };
            entry.size = if is_sampler { 0 } else { size as usize }
        } else {
            if entry.shader_scope == ShaderScope::Local
                && !self.registered_shaders[shader_id].use_locals
            {
                return Err(ShaderSysError::ShaderUniformUseLocal {
                    file: file!(),
                    line: line!(),
                });
            }
            entry.set_index = u8::MAX;
            let r = Range {
                offset: self.registered_shaders[shader_id].push_constant_size,
                size: size as usize,
            };
            entry.offset = r.offset;
            entry.size = r.size;
            let push_range_count = self.registered_shaders[shader_id].push_constant_range_count;
            self.registered_shaders[shader_id].push_constant_ranges[push_range_count] = r;
            self.registered_shaders[shader_id].push_constant_range_count += 1;
            self.registered_shaders[shader_id].push_constant_size += r.size;
        }

        self.registered_shaders[shader_id]
            .uniform_lookup
            .insert(uniform_name.to_string(), entry);
        self.registered_shaders[shader_id].uniforms.push(entry);
        if !is_sampler {
            if entry.shader_scope == ShaderScope::Global {
                self.registered_shaders[shader_id].global_ubo_size += entry.size;
            } else if entry.shader_scope == ShaderScope::Instance {
                self.registered_shaders[shader_id].ubo_size += entry.size;
            }
        }
        Ok(())
    }
}

impl<'a> Drop for ShaderSystem<'a> {
    fn drop(&mut self) {
        let size = self.registered_shaders.len();
        for i in 0..size {
            let shader = &self.registered_shaders[i];
            if shader.id == INVALID_ID {
                continue;
            }
            let _ = self.destroy_shader(shader.id);
            self.registered_shaders[i] = Shader::default();
        }
    }
}
