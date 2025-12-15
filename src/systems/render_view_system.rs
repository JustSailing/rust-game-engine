use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    basic::math::consts::INVALID_ID,
    renderer::{
        frontend_renderer::{Renderer, RendererError},
        renderer_types::{
            PacketData, RenderView, RenderViewConfig, RenderViewKnownType, RenderViewPacket,
        },
        views::render_views::{SkyboxRenderView, UIRenderView, WorldRenderView},
    },
    systems::{
        camera_system::CameraSystem, geometry_system::GeometrySystem,
        material_system::MaterialSystem, shader_system::ShaderSystem,
        texture_system::TextureSystem,
    },
};
use thiserror::Error;

#[derive(Default, Clone, Copy)]
pub struct RenderViewSysConfig {
    max_count: usize,
}

impl RenderViewSysConfig {
    pub fn max_count(mut self, max_count: usize) -> Self {
        self.max_count = max_count;
        self
    }
}

#[derive(Error, Debug)]
pub enum RenderViewSysError {
    #[error("render view system error: renderpass in config is zero")]
    ConfigHasNoRenderpasses,
    #[error("render view system error: render view already exists")]
    RenderViewAlreadyExists,
    #[error("render view system error: render view does not exist")]
    RenderViewDoesNotExist,
    #[error("render view system error: max render views reached")]
    MaxRenderViewsReached,
    #[error("render view system error: max render views reached")]
    RenderViewKnownTypeUnknown,
    #[error("texture system error: frontend renderer error: {0}")]
    RendererSysErr(Box<RendererError>),
}

impl From<RendererError> for RenderViewSysError {
    fn from(err: RendererError) -> Self {
        RenderViewSysError::RendererSysErr(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, RenderViewSysError>;

struct RenderViewRef {
    handle: RenderViewHandle,
    reference_count: usize,
}

impl Default for RenderViewRef {
    fn default() -> Self {
        Self {
            handle: INVALID_ID,
            reference_count: 0,
        }
    }
}

pub type RenderViewHandle = usize;

impl RenderViewRef {
    fn handle(mut self, handle: usize) -> Self {
        self.handle = handle;
        self
    }

    fn reference_count(mut self, reference_count: usize) -> Self {
        self.reference_count = reference_count;
        self
    }
}

pub struct RenderViewSystem<'a> {
    max_view_count: usize,
    registered_view_hashmap: HashMap<String, RenderViewRef>,
    registered_views: Vec<Box<dyn RenderView>>,
    renderer_system: Rc<RefCell<Renderer<'a>>>,
    shader_system: Rc<RefCell<ShaderSystem<'a>>>,
    camera_system: Rc<RefCell<CameraSystem>>,
    material_system: Rc<RefCell<MaterialSystem<'a>>>,
    geometry_system: Rc<RefCell<GeometrySystem<'a>>>,
    texture_system: Rc<RefCell<TextureSystem<'a>>>,
}

impl<'a> RenderViewSystem<'a> {
    pub fn initialize(
        config: &RenderViewSysConfig,
        renderer_system: Rc<RefCell<Renderer<'a>>>,
        shader_system: Rc<RefCell<ShaderSystem<'a>>>,
        camera_system: Rc<RefCell<CameraSystem>>,
        material_system: Rc<RefCell<MaterialSystem<'a>>>,
        geometry_system: Rc<RefCell<GeometrySystem<'a>>>,
        texture_system: Rc<RefCell<TextureSystem<'a>>>,
    ) -> Result<Self> {
        let registered_view_hashmap = HashMap::<String, RenderViewRef>::new();
        let mut registered_views = Vec::<Box<dyn RenderView>>::with_capacity(config.max_count);
        for _ in 0..config.max_count {
            registered_views.push(Box::new(WorldRenderView::default()));
        }
        Ok(Self {
            max_view_count: config.max_count,
            registered_views: registered_views,
            registered_view_hashmap: registered_view_hashmap,
            renderer_system,
            camera_system,
            shader_system,
            material_system,
            geometry_system,
            texture_system,
        })
    }

    pub fn create_view(&mut self, config: &RenderViewConfig) -> Result<RenderViewHandle> {
        if config.passes.len() < 1 {
            return Err(RenderViewSysError::ConfigHasNoRenderpasses);
        }

        let view_ref = self
            .registered_view_hashmap
            .remove(&config.name)
            .unwrap_or_default();
        if view_ref.handle != INVALID_ID {
            return Err(RenderViewSysError::RenderViewAlreadyExists);
        }

        let id = self
            .registered_views
            .iter()
            .enumerate()
            .find_map(|(i, v)| {
                if v.get_id() == INVALID_ID {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| INVALID_ID);

        if id == INVALID_ID {
            return Err(RenderViewSysError::MaxRenderViewsReached);
        }

        match config.known_type {
            RenderViewKnownType::UI => {
                let mut view: UIRenderView = UIRenderView::default()
                    .id(id)
                    .known_type(config.known_type)
                    .custom_shader_name(&config.custom_shader_name);
                for pass in config.passes.iter() {
                    view.passes.push(
                        self.renderer_system
                            .borrow()
                            .get_renderpass_handle(&pass.name)?,
                    );
                }
                view.create(&self.shader_system, &self.camera_system)?;
                let _ = std::mem::replace(&mut self.registered_views[id], Box::new(view));
                self.registered_view_hashmap.insert(
                    config.custom_shader_name.clone(),
                    RenderViewRef::default().handle(id),
                );
            }
            RenderViewKnownType::World => {
                let mut view = WorldRenderView::default()
                    .id(id)
                    .known_type(config.known_type)
                    .custom_shader_name(&config.custom_shader_name);
                for pass in config.passes.iter() {
                    view.passes.push(
                        self.renderer_system
                            .borrow()
                            .get_renderpass_handle(&pass.name)?,
                    );
                }
                view.create(&self.shader_system, &self.camera_system)?;
                let _ = std::mem::replace(&mut self.registered_views[id], Box::new(view));
                self.registered_view_hashmap.insert(
                    config.custom_shader_name.clone(),
                    RenderViewRef::default().handle(id),
                );
            }
            RenderViewKnownType::Skybox => {
                let mut view = SkyboxRenderView::default()
                    .id(id)
                    .known_type(config.known_type)
                    .custom_shader_name(&config.custom_shader_name);
                for pass in config.passes.iter() {
                    view.passes.push(
                        self.renderer_system
                            .borrow()
                            .get_renderpass_handle(&pass.name)?,
                    );
                }
                view.create(&self.shader_system, &self.camera_system)?;
                let _ = std::mem::replace(&mut self.registered_views[id], Box::new(view));
                self.registered_view_hashmap.insert(
                    config.custom_shader_name.clone(),
                    RenderViewRef::default().handle(id),
                );
            }
            _ => return Err(RenderViewSysError::RenderViewKnownTypeUnknown),
        }

        Ok(id)
    }

    pub fn on_window_resize(&mut self, width: u16, height: u16) -> Result<()> {
        for view in self.registered_views.iter_mut() {
            if view.get_id() != INVALID_ID {
                view.resize(width, height, &self.renderer_system)?;
            }
        }
        Ok(())
    }

    pub fn get_render_view_handle(&self, name: &str) -> Result<RenderViewHandle> {
        let handle = self.registered_view_hashmap.get(name);
        if handle.is_none() {
            return Err(RenderViewSysError::RenderViewDoesNotExist);
        }
        Ok(handle.unwrap().handle)
    }

    pub fn get_render_view_by_name(&self, name: &str) -> Result<&dyn RenderView> {
        let handle = self.get_render_view_handle(name)?;
        Ok(self.registered_views[handle].as_ref())
    }

    pub fn get_mut_render_view_by_name(&mut self, name: &str) -> Result<&mut dyn RenderView> {
        let handle = self.get_render_view_handle(name)?;
        Ok(self.registered_views[handle].as_mut())
    }

    pub fn get_render_view_by_id(&self, id: usize) -> Result<&dyn RenderView> {
        Ok(self.registered_views[id].as_ref())
    }

    pub fn get_mut_render_view_by_id(&mut self, id: usize) -> Result<&mut dyn RenderView> {
        Ok(self.registered_views[id].as_mut())
    }
    pub fn build_packet(
        &self,
        render_view_handle: RenderViewHandle,
        data_packet: &mut PacketData,
    ) -> Result<RenderViewPacket> {
        Ok(self.registered_views[render_view_handle].build_packet(
            data_packet,
            &self.camera_system,
            &self.geometry_system,
            &self.material_system,
            &self.texture_system,
        )?)
    }

    pub fn on_render(
        &self,
        render_view_handle: RenderViewHandle,
        frame_number: u64,
        render_view_packet: &mut RenderViewPacket,
    ) -> Result<()> {
        self.registered_views[render_view_handle].render(
            &self.shader_system,
            &self.material_system,
            &self.renderer_system,
            &self.geometry_system,
            frame_number,
            render_view_packet,
        )?;
        Ok(())
    }
}
