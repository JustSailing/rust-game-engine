use crate::application::{
    basic::{
        math::{consts::INVALID_ID, vec3::Vector3D, vec4::Vec4},
        window::Window,
    },
    renderer::{
        renderer_types::GeometryRenderData,
        vulkan::{
            vulkan_buffer::VulkanBuffer,
            vulkan_command_buffer::VulkanCommandBuffer,
            vulkan_device::VulkanDevice,
            vulkan_image::VulkanImage,
            vulkan_pipeline::VulkanPipeline,
            vulkan_renderpass::{ClearFlag, VulkanRenderPass},
            vulkan_swapchain::VulkanSwapchain,
            vulkan_sync_objects::{InFlightFrames, SyncObjects},
        },
    },
    resources::resource_types::{
        Geometry, ResourceData, ResourceType, ShaderAttributeType, ShaderScope, ShaderStage,
        ShaderUniformType, Texture, TextureMap,
    },
    systems::{
        geometry_system::DEFAULT_GEOMETRY_NAME,
        resource_system::{ResourceSysError, ResourceSystem},
        shader_system::{Shader, ShaderInternalData},
        texture_system::TextureSystem,
    },
};

use ash::{
    Entry, Instance, LoadingError,
    khr::{
        surface::{self, Instance as SurfaceInstance},
        xlib_surface,
    },
    vk::{
        self, BorderColor, BufferUsageFlags, CommandPool, CommandPoolResetFlags, CompareOp,
        DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags,
        DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
        DescriptorType, DeviceSize, Extent2D, Fence, Format, Framebuffer, FramebufferCreateInfo,
        ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        IndexType, MemoryMapFlags, MemoryPropertyFlags, Offset2D, PipelineBindPoint,
        PipelineShaderStageCreateInfo, PipelineStageFlags, Queue, Rect2D, Sampler,
        SamplerCreateInfo, SamplerMipmapMode, ShaderModule, ShaderModuleCreateInfo,
        ShaderStageFlags, SubmitInfo, SurfaceKHR, VertexInputAttributeDescription, Viewport,
        WriteDescriptorSet,
    },
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CString, NulError, c_void},
    ptr,
    rc::Rc,
};
use thiserror::Error;
type Result<T> = std::result::Result<T, VulkanBackendError>;

#[derive(Error, Debug)]
pub enum VulkanBackendError {
    #[error("vulkan backend error: {issue} {file} {line}")]
    OperationFailed {
        issue: &'static str,
        file: &'static str,
        line: u32,
    },
    #[error("{source}\nvulkan backend error: loading error")]
    AshLoadingError {
        #[from]
        source: LoadingError,
    },
    #[error("{source}\nvulkan backend error: CString error")]
    CStringError {
        #[from]
        source: NulError,
    },
    #[error("{source}\nvulkan backend error: ash result error")]
    AshResultError {
        #[from]
        source: vk::Result,
    },
    #[error("{source}\nvulkan backend error: resource sys error")]
    ResourceError {
        #[from]
        source: ResourceSysError,
    },
}

const VULKAN_MAX_GEOMETRY_COUNT: usize = 4096;
const VULKAN_MAX_MATERIAL_COUNT: usize = 1024;
const VULKAN_SHADER_MAX_STAGES: usize = 8;
const VULKAN_SHADER_MAX_GLOBAL_TEXTURES: usize = 31;
const VULKAN_SHADER_MAX_INSTANCE_TEXTURES: usize = 31;
const VULKAN_SHADER_MAX_ATTRIBUTES: usize = 16;
const VULKAN_SHADER_MAX_UNIFORMS: usize = 128;
const VULKAN_SHADER_MAX_BINDINGS: usize = 2;
const VULKAN_SHADER_MAX_PUSH_CONST_RANGE: usize = 32;
const DESC_SET_INDEX_GLOBAL: usize = 0;
const DESC_SET_INDEX_INSTANCE: usize = 1;
const BINDING_INDEX_UBO: usize = 0;
const BINDING_INDEX_SAMPLER: usize = 1;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct VulkanShaderStage<'a> {
    shader_mod: ShaderModule,
    shader_stage_create_info: PipelineShaderStageCreateInfo<'a>,
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct VulkanDescriptorSetConfig<'a> {
    binding_count: u8,
    bindings: [DescriptorSetLayoutBinding<'a>; VULKAN_SHADER_MAX_BINDINGS],
}

#[derive(Default, Clone)]
pub struct VulkanShaderStageConfig {
    stage: ShaderStageFlags,
    file_name: String,
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct VulkanShaderConfig<'a> {
    stage_count: u8,
    stages: [VulkanShaderStageConfig; VULKAN_SHADER_MAX_STAGES],
    pool_sizes: [DescriptorPoolSize; 2],
    max_descriptor_set_count: u16,
    descriptor_set_count: u8,
    descriptor_set_configs: [VulkanDescriptorSetConfig<'a>; 2],
    attributes: [VertexInputAttributeDescription; VULKAN_SHADER_MAX_ATTRIBUTES],
}

#[derive(Clone)]
#[repr(C)]
pub struct VulkanDescriptorState {
    generations: [u8; 3],
    ids: [u32; 3],
}

impl Default for VulkanDescriptorState {
    fn default() -> Self {
        Self {
            generations: [INVALID_ID as u8; 3],
            ids: [INVALID_ID as u32; 3],
        }
    }
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct VulkanShaderDescriptorSetState {
    descriptor_sets: [DescriptorSet; 3],
    descriptor_states: [VulkanDescriptorState; VULKAN_SHADER_MAX_BINDINGS],
}

#[derive(Clone)]
#[repr(C)]
pub struct VulkanShaderInstanceState {
    id: u32,
    offset: u64,
    descriptor_set_state: VulkanShaderDescriptorSetState,
    instance_texture_maps: Vec<Rc<RefCell<TextureMap>>>,
}

impl Default for VulkanShaderInstanceState {
    fn default() -> Self {
        Self {
            id: INVALID_ID as u32,
            offset: Default::default(),
            descriptor_set_state: Default::default(),
            instance_texture_maps: Default::default(),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct VulkanShader<'a> {
    pub mapped_uniform_block: *mut c_void,
    vulkan_shader_config: VulkanShaderConfig<'a>,
    renderpass: BuiltInRenderpass,
    stages: [VulkanShaderStage<'a>; VULKAN_SHADER_MAX_STAGES],
    descriptor_pool: DescriptorPool,
    descriptor_set_layouts: [DescriptorSetLayout; 2],
    global_descriptor_sets: Vec<DescriptorSet>,
    pub uniform_buffer: VulkanBuffer,
    pipeline: VulkanPipeline,
    instance_count: u32,
    instance_states: [VulkanShaderInstanceState; VULKAN_MAX_MATERIAL_COUNT],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct VulkanGeometryData {
    id: usize,
    generation: usize,
    vertex_count: u32,
    vertex_size: u32,
    vertex_buffer_offset: u32,
    index_count: u32,
    index_size: u32,
    index_buffer_offset: u32,
}

impl Default for VulkanGeometryData {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            generation: INVALID_ID,
            vertex_count: Default::default(),
            vertex_size: Default::default(),
            vertex_buffer_offset: Default::default(),
            index_count: Default::default(),
            index_size: Default::default(),
            index_buffer_offset: Default::default(),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub enum BuiltInRenderpass {
    World,
    UI,
}

#[repr(C)]
pub struct VulkanContext {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_instance_loader: debug_utils::Instance,
    #[cfg(feature = "debug")]
    debug_utils_loader: debug_utils::Device,
    default_texture: Rc<RefCell<Texture>>,
    image_index: u32,
    resource_system: Rc<RefCell<ResourceSystem>>,
    texture_system: Rc<RefCell<TextureSystem>>,
    frame_delta_time: f32,
    geometry_vertex_offset: u64,
    geometry_index_offset: u64,
    geometries: Vec<VulkanGeometryData>,
    object_index_buffer: VulkanBuffer,
    object_vertex_buffer: VulkanBuffer,
    images_in_flight: Vec<Rc<RefCell<Option<SyncObjects>>>>,
    in_flight_frames: InFlightFrames,
    graphics_cmd_bufs: VulkanCommandBuffer,
    swapchain_framebuffers: Vec<Framebuffer>,
    world_framebuffers: Vec<Framebuffer>,
    main_renderpass: VulkanRenderPass,
    ui_renderpass: VulkanRenderPass,
    recreating_swapchain: bool,
    swapchain: VulkanSwapchain,
    frame_buffer_size_generation: u32,
    frame_buffer_last_generation: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    device: VulkanDevice,
    surface_loader: SurfaceInstance,
    surface: SurfaceKHR,
    instance: Instance,
}

impl<'a> VulkanContext {
    pub fn initialize(
        name: &str,
        window: &Window,
        resource_system: Rc<RefCell<ResourceSystem>>,
        texture_system: Rc<RefCell<TextureSystem>>,
    ) -> Result<Self> {
        let entry = unsafe { Entry::load()? };

        let app_name = CString::new(name)?;

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::make_api_version(0, 1, 3, 0))
            .application_name(app_name.as_c_str())
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"IronOxide")
            .engine_version(0);

        #[allow(unused_mut)] //debug purpose
        let mut layers_names_raw: [*const i8; 1] = [c"".as_ptr()];
        let mut extensions = Vec::new();
        extensions.push(surface::NAME.as_ptr());
        extensions.push(xlib_surface::NAME.as_ptr());
        #[cfg(feature = "debug")]
        {
            extensions.push(debug_utils::NAME.as_ptr());
            let layer_names = c"VK_LAYER_KHRONOS_validation";
            layers_names_raw[0] = layer_names.as_ptr();

            let exts = unsafe { entry.enumerate_instance_extension_properties(None)? };

            for ex in exts {
                println!("{:?} ", ex.extension_name_as_c_str())
            }

            let layers = unsafe { entry.enumerate_instance_layer_properties()? };

            for l in layers {
                println!("{:?} ", l.layer_name_as_c_str())
            }
        }

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);
        if cfg!(feature = "debug") {
            create_info = create_info.enabled_layer_names(&layers_names_raw);
        }
        let instance = unsafe { entry.create_instance(&create_info, None)? };

        // create xlib surface
        let xlib_surface_info = vk::XlibSurfaceCreateInfoKHR::default()
            .dpy(window.display.raw as *mut c_void)
            .window(window.window_id);
        let xlib_surface_loader = xlib_surface::Instance::new(&entry, &instance);
        let surface = unsafe { xlib_surface_loader.create_xlib_surface(&xlib_surface_info, None)? };
        let surface_loader = surface::Instance::new(&entry, &instance);

        //create device
        let mut dev = VulkanDevice::new(&instance, &surface, &surface_loader)?;

        // create swapchain
        let swap = VulkanSwapchain::create(
            &instance,
            &mut dev,
            &surface,
            &surface_loader,
            window.width,
            window.height,
            &texture_system,
        )?;

        // create renderpass
        let main_renderpass = VulkanRenderPass::create(
            Vec4::new(0.0, 0.0, window.width as f32, window.height as f32),
            Vec4::new(0.0, 0.0, 0.2, 1.0),
            1.0,
            0,
            &dev,
            swap.image_format.format,
            dev.depth_format,
            ClearFlag::ColourBuffer.value()
                | ClearFlag::DepthBuffer.value()
                | ClearFlag::StencilBuffer.value(),
            false,
            true,
        )?;

        let ui_renderpass = VulkanRenderPass::create(
            Vec4::new(0.0, 0.0, window.width as f32, window.height as f32),
            Vec4::new(0.0, 0.0, 0.0, 0.0),
            1.0,
            0,
            &dev,
            swap.image_format.format,
            dev.depth_format,
            0,
            true,
            false,
        )?;

        let mut swap_framebuffers = Vec::new();
        let mut world_framebuffers = Vec::new();
        for i in 0..swap.render_textures.len() {
            let mut world_attachments = Vec::new();
            world_attachments.push(
                swap.render_textures[i]
                    .borrow()
                    .internal_data
                    .image
                    .view
                    .unwrap(),
            );
            world_attachments.push(swap.depth_attachment.view.unwrap());
            let world_framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(main_renderpass.renderpass)
                .attachments(&world_attachments)
                .height(window.height)
                .width(window.width)
                .layers(1);

            let world_framebuffer = unsafe {
                match dev
                    .device
                    .create_framebuffer(&world_framebuffer_create_info, None)
                {
                    Ok(f) => f,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create framebuffer",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            world_framebuffers.push(world_framebuffer);

            let mut ui_attachments = Vec::new();
            ui_attachments.push(
                swap.render_textures[i]
                    .borrow()
                    .internal_data
                    .image
                    .view
                    .unwrap(),
            );

            let ui_framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(ui_renderpass.renderpass)
                .attachments(&ui_attachments)
                .height(window.height)
                .width(window.width)
                .layers(1);

            let ui_framebuffer = unsafe {
                match dev
                    .device
                    .create_framebuffer(&ui_framebuffer_create_info, None)
                {
                    Ok(f) => f,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create framebuffer",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            swap_framebuffers.push(ui_framebuffer);
        }
        // creating command buffer
        let graph_cmd_buf = Self::create_command_buffer(&dev, swap.max_frames_in_flight as usize)?;

        // creating sync objects
        let mut sync_objects = Vec::with_capacity(swap.max_frames_in_flight as usize);
        let mut images_in_flight = Vec::with_capacity(swap.max_frames_in_flight as usize);
        for _ in 0..swap.max_frames_in_flight {
            let obj = SyncObjects::create(&dev, true)?;
            sync_objects.push(obj);
        }

        for _ in 0..swap.image_count as usize {
            images_in_flight.push(Rc::new(RefCell::new(None)));
        }

        let in_flight_frames = InFlightFrames::new(sync_objects);

        let (vertex_buffer, index_buffer) = Self::create_buffers(&instance, &dev)?;

        let mut geometries = Vec::<VulkanGeometryData>::with_capacity(VULKAN_MAX_GEOMETRY_COUNT);
        for _ in 0..VULKAN_MAX_GEOMETRY_COUNT {
            geometries.push(VulkanGeometryData::default());
        }

        #[cfg(feature = "debug")]
        {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_instance_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_messenger =
                unsafe { debug_instance_loader.create_debug_utils_messenger(&debug_info, None)? };
            println!("Vulkan Instance created");
            let debug_utils_loader = debug_utils::Device::new(&instance, &dev.device);
            return Ok(VulkanContext {
                frame_delta_time: 0.0,
                geometry_vertex_offset: 0,
                geometry_index_offset: 0,
                geometries: geometries,
                object_index_buffer: index_buffer,
                object_vertex_buffer: vertex_buffer,
                image_index: 0,
                images_in_flight: images_in_flight,
                recreating_swapchain: false,
                in_flight_frames: in_flight_frames,
                graphics_cmd_bufs: graph_cmd_buf,
                device: dev,
                surface_loader: surface_loader,
                surface: surface,
                instance: instance,
                swapchain: swap,
                main_renderpass: main_renderpass,
                ui_renderpass: ui_renderpass,
                framebuffer_height: window.height,
                framebuffer_width: window.width,
                frame_buffer_size_generation: 0,
                frame_buffer_last_generation: 0,
                swapchain_framebuffers: swap_framebuffers,
                world_framebuffers: world_framebuffers,
                resource_system: resource_system,
                texture_system: texture_system,
                default_texture: Rc::new(RefCell::new(Texture::default())),
                dbg_messenger: debug_messenger,
                dbg_instance_loader: debug_instance_loader,
                debug_utils_loader: debug_utils_loader,
            });
        }

        #[cfg(not(feature = "debug"))]
        {
            println!("Vulkan Instance created");
            Ok(Self {
                default_texture: Rc::new(RefCell::new(Texture::default())),
                frame_delta_time: 0.0,
                geometry_vertex_offset: 0,
                geometry_index_offset: 0,
                geometries,
                object_index_buffer: index_buffer,
                object_vertex_buffer: vertex_buffer,
                image_index: 0,
                images_in_flight,
                recreating_swapchain: false,
                in_flight_frames,
                graphics_cmd_bufs: graph_cmd_buf,
                device: dev,
                instance,
                surface,
                surface_loader,
                swapchain: swap,
                main_renderpass,
                ui_renderpass,
                framebuffer_height: window.height,
                framebuffer_width: window.width,
                frame_buffer_size_generation: 0,
                frame_buffer_last_generation: 0,
                swapchain_framebuffers: swap_framebuffers,
                world_framebuffers,
                resource_system,
                texture_system,
            })
        }
    }

    pub fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    pub fn on_resize(&mut self, width: i32, height: i32) -> Result<()> {
        self.framebuffer_height = height as u32;
        self.framebuffer_width = width as u32;
        self.frame_buffer_last_generation += 1;

        println!(
            "renderer backend resized w: {}, h: {}, gen: {}",
            self.framebuffer_width, self.framebuffer_height, self.frame_buffer_last_generation
        );

        Ok(())
    }

    pub fn begin_frame(&mut self, delta: f32) -> Result<bool> {
        self.frame_delta_time = delta;
        if self.recreating_swapchain {
            return match unsafe { self.device.device.device_wait_idle() } {
                Ok(_) => Ok(false),
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue: "could not wait on device",
                    file: file!(),
                    line: line!(),
                }),
            };
        }

        if self.frame_buffer_last_generation != self.frame_buffer_size_generation {
            return match unsafe { self.device.device.device_wait_idle() } {
                Ok(_) => match self.recreate_swapchain() {
                    Ok(()) => Ok(false),
                    Err(e) => Err(e),
                },
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue: "could not wait on device",
                    file: file!(),
                    line: line!(),
                }),
            };
        }
        let sync = &self.in_flight_frames.sync_objs;
        let current_frame = self.in_flight_frames.current_frame;

        if !sync[current_frame].fence_wait(&self.device, u64::MAX)? {
            return Ok(false);
        }

        sync[current_frame].reset_fence(&self.device)?;

        self.image_index = match self.swapchain.acquire_next_image_index(
            u64::MAX,
            sync[current_frame].image_avail_semaphore,
            sync[current_frame].fence,
        ) {
            Ok((suboptimal, index)) => {
                if suboptimal {
                    return match self.recreate_swapchain() {
                        Ok(()) => Ok(false),
                        Err(e) => Err(e),
                    };
                } else {
                    index
                }
            }
            Err(e) => return Err(e),
        };

        // not sure if i need this also
        if let Some(s) = *self.images_in_flight[self.image_index as usize].borrow() {
            if s.fence != Fence::null() {
                if !s.fence_wait(&self.device, u64::MAX)? {
                    return Ok(false);
                }
            }
        }

        let command_buffer = &mut self.graphics_cmd_bufs;
        command_buffer.reset(&self.device, current_frame)?;
        command_buffer.begin(&self.device, false, false, false, current_frame)?;

        let viewport = Viewport::default()
            .x(0.0)
            .y(self.framebuffer_height as f32)
            .height(-(self.framebuffer_height as f32))
            .width(self.framebuffer_width as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = Rect2D::default()
            .extent(
                Extent2D::default()
                    .height(self.framebuffer_height)
                    .width(self.framebuffer_width),
            )
            .offset(Offset2D::default());

        unsafe {
            self.device.device.cmd_set_viewport(
                command_buffer.command_buffer[current_frame],
                0,
                std::slice::from_ref(&viewport),
            );
        };
        unsafe {
            self.device.device.cmd_set_scissor(
                command_buffer.command_buffer[current_frame],
                0,
                std::slice::from_ref(&scissor),
            )
        };

        self.main_renderpass
            .render_area
            .set_w(self.framebuffer_width as f32);
        self.main_renderpass
            .render_area
            .set_h(self.framebuffer_height as f32);
        self.ui_renderpass
            .render_area
            .set_w(self.framebuffer_width as f32);
        self.ui_renderpass
            .render_area
            .set_h(self.framebuffer_height as f32);

        Ok(true)
    }

    pub fn end_frame(&mut self, _delta: f32) -> Result<()> {
        let current_frame = self.in_flight_frames.current_frame;
        let command_buff = &mut self.graphics_cmd_bufs;

        command_buff.end(&self.device, current_frame)?;

        if let Some(sync_obj) = *self.images_in_flight[self.image_index as usize].borrow_mut() {
            match sync_obj.fence_wait(&self.device, u64::MAX) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        self.images_in_flight[self.image_index as usize].replace(Some(
            self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame],
        ));

        // should handle bool here
        self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame]
            .fence_wait(&self.device, u64::MAX)?;

        self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame]
            .reset_fence(&self.device)?;

        let submit_info = SubmitInfo::default()
            .command_buffers(std::slice::from_ref(
                &self.graphics_cmd_bufs.command_buffer[self.in_flight_frames.current_frame],
            ))
            .signal_semaphores(std::slice::from_ref(
                &self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame]
                    .render_finished_semaphore,
            ))
            .wait_semaphores(std::slice::from_ref(
                &self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame]
                    .image_avail_semaphore,
            ))
            .wait_dst_stage_mask(std::slice::from_ref(
                &PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ));

        unsafe {
            self.device.device.queue_submit(
                self.device.graphics_queue,
                std::slice::from_ref(&submit_info),
                self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame].fence,
            )?;
        }

        if !self.swapchain.present(
            &self.device.graphics_queue,
            &self.device.present_queue,
            &self.in_flight_frames.sync_objs[self.in_flight_frames.current_frame]
                .render_finished_semaphore,
            self.image_index,
        )? {
            self.recreate_swapchain()?;
        }

        self.in_flight_frames.current_frame = (self.in_flight_frames.current_frame + 1)
            % self.swapchain.max_frames_in_flight as usize;

        Ok(())
    }

    pub fn begin_renderpass(&mut self, renderpass_type: BuiltInRenderpass) -> Result<()> {
        let (renderpass, framebuffer) = match renderpass_type {
            BuiltInRenderpass::World => (
                &self.main_renderpass,
                self.world_framebuffers[self.image_index as usize],
            ),
            BuiltInRenderpass::UI => (
                &self.ui_renderpass,
                self.swapchain_framebuffers[self.image_index as usize],
            ),
        };
        renderpass.begin(
            &self.device,
            &mut self.graphics_cmd_bufs,
            self.in_flight_frames.current_frame,
            framebuffer,
        );

        Ok(())
    }

    pub fn end_renderpass(&mut self, renderpass_type: BuiltInRenderpass) -> Result<()> {
        let renderpass = match renderpass_type {
            BuiltInRenderpass::World => &self.main_renderpass,
            BuiltInRenderpass::UI => &self.ui_renderpass,
        };
        renderpass.end(
            &self.device,
            &mut self.graphics_cmd_bufs,
            self.in_flight_frames.current_frame,
        );
        Ok(())
    }

    pub fn find_memory_index(
        instance: &Instance,
        dev: &VulkanDevice,
        type_filter: u32,
        property_flags: MemoryPropertyFlags,
    ) -> i32 {
        let memory_props =
            unsafe { instance.get_physical_device_memory_properties(dev.physical_device) };
        for i in 0..memory_props.memory_type_count {
            let suitable = (type_filter & (1 << i)) != 0;
            let memory_type = memory_props.memory_types[i as usize];

            if suitable && memory_type.property_flags.contains(property_flags) {
                return i as i32;
            }
        }
        -1
    }

    pub fn recreate_swapchain(&mut self) -> Result<()> {
        println!("recreating swapchain");
        let _ = unsafe { self.device.device.device_wait_idle() };
        for i in 0..self.swapchain.image_count as usize {
            self.images_in_flight[i].replace(None);
        }

        self.in_flight_frames.destroy(&self.device);
        let mut sync_objs = Vec::new();
        for _ in 0..self.swapchain.max_frames_in_flight {
            sync_objs.push(SyncObjects::create(&self.device, true)?);
        }
        self.in_flight_frames = InFlightFrames::new(sync_objs);

        self.recreating_swapchain = true;
        //self.swapchain.destroy(&self.device);
        self.swapchain.recreate(
            &self.instance,
            &mut self.device,
            &self.surface,
            &self.surface_loader,
            self.framebuffer_width,
            self.framebuffer_height,
            &self.texture_system,
        )?;

        self.main_renderpass
            .render_area
            .set_w(self.framebuffer_width as f32);
        self.main_renderpass
            .render_area
            .set_h(self.framebuffer_height as f32);

        self.ui_renderpass
            .render_area
            .set_w(self.framebuffer_width as f32);
        self.ui_renderpass
            .render_area
            .set_h(self.framebuffer_height as f32);

        self.frame_buffer_last_generation = self.frame_buffer_size_generation;

        // I think you can clear all command buffers without having to destroy and recreate
        unsafe {
            self.device.device.reset_command_pool(
                self.device.graphics_command_pool,
                CommandPoolResetFlags::empty(),
            )?
        }

        for i in 0..self.swapchain.image_count as usize {
            unsafe {
                self.device
                    .device
                    .destroy_framebuffer(self.swapchain_framebuffers[i], None);
                self.device
                    .device
                    .destroy_framebuffer(self.world_framebuffers[i], None);
            }
        }

        self.regenerate_framebuffers()?;

        self.recreating_swapchain = false;

        Ok(())
    }

    fn create_command_buffer(device: &VulkanDevice, frames: usize) -> Result<VulkanCommandBuffer> {
        VulkanCommandBuffer::allocate(device, true, device.graphics_command_pool, frames as u32)
    }

    fn regenerate_framebuffers(&mut self) -> Result<()> {
        let mut world_framebuffers = Vec::new();
        let mut swap_framebuffers = Vec::new();
        for i in 0..self.swapchain.render_textures.len() {
            let mut world_attachments = Vec::new();
            world_attachments.push(
                self.swapchain.render_textures[i]
                    .borrow()
                    .internal_data
                    .image
                    .view
                    .unwrap(),
            );
            world_attachments.push(self.swapchain.depth_attachment.view.unwrap());
            let framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(self.main_renderpass.renderpass)
                .attachments(&world_attachments)
                .height(self.framebuffer_height)
                .width(self.framebuffer_width)
                .layers(1);

            let framebuffer = unsafe {
                match self
                    .device
                    .device
                    .create_framebuffer(&framebuffer_create_info, None)
                {
                    Ok(f) => f,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create framebuffer",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            world_framebuffers.push(framebuffer);

            let mut ui_attachments = Vec::new();
            ui_attachments.push(
                self.swapchain.render_textures[i]
                    .borrow()
                    .internal_data
                    .image
                    .view
                    .unwrap(),
            );

            let ui_framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(self.ui_renderpass.renderpass)
                .attachments(&ui_attachments)
                .height(self.framebuffer_height)
                .width(self.framebuffer_width)
                .layers(1);

            let ui_framebuffer = unsafe {
                match self
                    .device
                    .device
                    .create_framebuffer(&ui_framebuffer_create_info, None)
                {
                    Ok(f) => f,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create framebuffer",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            swap_framebuffers.push(ui_framebuffer);
        }
        self.world_framebuffers = world_framebuffers;
        self.swapchain_framebuffers = swap_framebuffers;
        Ok(())
    }

    fn create_buffers(
        instance: &Instance,
        device: &VulkanDevice,
    ) -> Result<(VulkanBuffer, VulkanBuffer)> {
        let memory_property_flag = MemoryPropertyFlags::DEVICE_LOCAL;

        const VERTEX_BUFFER_SIZE: usize = size_of::<Vector3D>() * 1024 * 1024;

        let vertex_buffer = VulkanBuffer::create(
            instance,
            device,
            VERTEX_BUFFER_SIZE as u64,
            BufferUsageFlags::VERTEX_BUFFER
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
            memory_property_flag,
            true,
        )?;

        const INDEX_BUFFER_SIZE: usize = size_of::<u32>() * 1024 * 1024;

        let index_buffer = VulkanBuffer::create(
            instance,
            device,
            INDEX_BUFFER_SIZE as u64,
            BufferUsageFlags::INDEX_BUFFER
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
            memory_property_flag,
            true,
        )?;
        Ok((vertex_buffer, index_buffer))
    }

    pub fn create_texture(&self, name: &str, pixels: &[u8], texture: &mut Texture) -> Result<()> {
        let image_size: DeviceSize = (texture.width as u64
            * texture.height as u64
            * texture.channel_count as u64) as DeviceSize;
        let image_format = Format::R8G8B8A8_UNORM;

        texture.internal_data.image = VulkanImage::create(
            &self.instance,
            ImageType::TYPE_2D,
            texture.width,
            texture.height,
            image_format,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::SAMPLED
                | ImageUsageFlags::COLOR_ATTACHMENT,
            MemoryPropertyFlags::DEVICE_LOCAL,
            true,
            ImageAspectFlags::COLOR,
            &self.device,
        )?;

        self.write_data_texture(texture, 0, image_size as u64, pixels)?;

        let _ = name;
        // TODO: should move parts of this into where the sampler is created
        #[cfg(feature = "debug")]
        {
            let image_name: CString = CString::new(name).unwrap();
            let image_view_name_info: vk::DebugUtilsObjectNameInfoEXT<'_> =
                vk::DebugUtilsObjectNameInfoEXT::default()
                    .object_name(&image_name)
                    .object_handle(texture.internal_data.image.view.unwrap());

            let image_name_info: vk::DebugUtilsObjectNameInfoEXT<'_> =
                vk::DebugUtilsObjectNameInfoEXT::default()
                    .object_name(&image_name)
                    .object_handle(texture.internal_data.image.image);

            //         let sampler_name_info: vk::DebugUtilsObjectNameInfoEXT<'_> =
            //             vk::DebugUtilsObjectNameInfoEXT::default()
            //                 .object_name(&image_name)
            //                 .object_handle(texture.internal_data.sampler);
            unsafe {
                self.debug_utils_loader
                    .set_debug_utils_object_name(&image_name_info)
                    .map_err(|_| VulkanBackendError::OperationFailed {
                        issue: "failed to name image object for debugging",
                        file: file!(),
                        line: line!(),
                    })?;
                //             self.debug_utils_loader
                //                 .set_debug_utils_object_name(&sampler_name_info)
                //                 .map_err(|_| VulkanBackendError::OperationFailed {
                //                     issue: "failed to name sampler object for debugging",
                //                     file: file!(),
                //                     line: line!(),
                //                 })?;
                self.debug_utils_loader
                    .set_debug_utils_object_name(&image_view_name_info)
                    .map_err(|_| VulkanBackendError::OperationFailed {
                        issue: "failed to name image view object for debugging",
                        file: file!(),
                        line: line!(),
                    })?;
            }
        }
        Ok(())
    }

    pub fn write_data_texture(
        &self,
        texture: &mut Texture,
        _offset: u32,
        size: u64,
        pixels: &[u8],
    ) -> Result<()> {
        let image_format = Format::R8G8B8A8_UNORM;
        let usage = BufferUsageFlags::TRANSFER_SRC;
        let memory_property_flags =
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT;
        let staging = VulkanBuffer::create(
            &self.instance,
            &self.device,
            size,
            usage,
            memory_property_flags,
            true,
        )?;

        staging.load_data(&self.device, 0, size, MemoryMapFlags::empty(), pixels)?;

        let mut temp_command_buffer = VulkanCommandBuffer::allocate_and_begin_single_use(
            &self.device,
            self.device.graphics_command_pool,
        )?;

        texture.internal_data.image.transition_layout(
            &self.device,
            &temp_command_buffer,
            image_format,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            0,
        )?;

        texture.internal_data.image.copy_from_buffer(
            &self.device,
            &staging,
            &temp_command_buffer,
            0,
        );

        texture.internal_data.image.transition_layout(
            &self.device,
            &temp_command_buffer,
            image_format,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            0,
        )?;

        temp_command_buffer.end_single_use(
            &self.device,
            self.device.graphics_command_pool,
            self.device.graphics_queue,
        )?;

        staging.destroy(&self.device);
        if texture.generation == INVALID_ID {
            texture.generation = 0
        } else {
            texture.generation += 1
        };
        Ok(())
    }

    pub fn create_writable_texture(&self, texture: &mut Texture) -> Result<()> {
        let image_format = Format::R8G8B8A8_UNORM;

        texture.internal_data.image = VulkanImage::create(
            &self.instance,
            ImageType::TYPE_2D,
            texture.width,
            texture.height,
            image_format,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::SAMPLED
                | ImageUsageFlags::COLOR_ATTACHMENT,
            MemoryPropertyFlags::DEVICE_LOCAL,
            true,
            ImageAspectFlags::COLOR,
            &self.device,
        )?;
        if texture.generation == INVALID_ID {
            texture.generation = 0
        } else {
            texture.generation += 1
        };
        Ok(())
    }

    pub fn resize_texture(
        &self,
        texture: &mut Texture,
        new_width: u32,
        new_height: u32,
    ) -> Result<()> {
        texture.internal_data.image.destroy(&self.device);
        texture.internal_data.image = VulkanImage::create(
            &self.instance,
            ImageType::TYPE_2D,
            new_width,
            new_height,
            Format::R8G8B8A8_UNORM,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::SAMPLED
                | ImageUsageFlags::COLOR_ATTACHMENT,
            MemoryPropertyFlags::DEVICE_LOCAL,
            true,
            ImageAspectFlags::COLOR,
            &self.device,
        )?;
        Ok(())
    }

    pub fn destroy_texture(&self, texture: &Texture) -> Result<()> {
        let _ = unsafe { self.device.device.device_wait_idle() };

        texture.internal_data.image.destroy(&self.device);
        Ok(())
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        &mut self,
        geometry: &mut Geometry,
        vertices: &[T],
        indices: &[U],
    ) -> Result<()> {
        let is_reupload = geometry.internal_id != INVALID_ID;
        let mut old_range = VulkanGeometryData::default();
        let mut internal_data: Option<&mut VulkanGeometryData> = None;
        if is_reupload {
            internal_data = Some(&mut self.geometries[geometry.internal_id]);
            let int_data = internal_data.as_mut().unwrap();
            old_range.index_buffer_offset = int_data.index_buffer_offset;
            old_range.index_count = int_data.index_count;
            old_range.index_size = int_data.index_size;
            old_range.vertex_buffer_offset = int_data.vertex_buffer_offset;
            old_range.vertex_count = int_data.vertex_count;
            old_range.vertex_size = int_data.vertex_size;
        } else {
            for (i, geo) in self.geometries.iter_mut().enumerate() {
                if geo.id == INVALID_ID {
                    geometry.internal_id = i;
                    geo.id = i;
                    internal_data = Some(geo);
                    break;
                }
            }
        }
        if internal_data.is_none() {
            return Err(VulkanBackendError::OperationFailed {
                issue: "Vulkan State Geometries is Full",
                file: file!(),
                line: line!(),
            });
        }

        let command_pool = self.device.graphics_command_pool;
        let queue = self.device.graphics_queue;
        let int_data = internal_data.unwrap();
        int_data.vertex_buffer_offset = self.geometry_vertex_offset as u32;
        int_data.vertex_count = vertices.len() as u32;
        int_data.vertex_size = vertices.len() as u32 * size_of::<T>() as u32;
        Self::upload_data_range(
            &self.instance,
            &self.device,
            command_pool,
            Fence::null(),
            queue,
            &self.object_vertex_buffer,
            self.geometry_vertex_offset,
            vertices,
        )?;

        self.geometry_vertex_offset += int_data.vertex_size as u64;

        if indices.len() > 0 {
            int_data.index_buffer_offset = self.geometry_index_offset as u32;
            int_data.index_count = indices.len() as u32;
            int_data.index_size = indices.len() as u32 * size_of::<U>() as u32;

            Self::upload_data_range(
                &self.instance,
                &self.device,
                command_pool,
                Fence::null(),
                queue,
                &self.object_index_buffer,
                self.geometry_index_offset,
                indices,
            )?;
            self.geometry_index_offset += int_data.index_size as u64;
        }
        if int_data.generation == INVALID_ID {
            int_data.generation = 0;
        } else {
            int_data.generation += 1;
        }
        // NEXT: create a free list of offsets that were freed
        Ok(())
    }

    pub fn destroy_geometry(&mut self, geometry: &Geometry) -> Result<()> {
        if geometry.internal_id != INVALID_ID || geometry.name == DEFAULT_GEOMETRY_NAME {
            self.geometries[geometry.internal_id] = VulkanGeometryData::default();
        }
        Ok(())
    }

    pub fn draw_geometry(&mut self, data: &mut GeometryRenderData) -> Result<()> {
        //let geo = data.geometry.borrow_mut();
        let buffer_data = &self.geometries[data.geometry.borrow().internal_id];
        let command_buffer =
            self.graphics_cmd_bufs.command_buffer[self.in_flight_frames.current_frame];

        let offsets: [DeviceSize; 1] = [buffer_data.vertex_buffer_offset.into()];
        unsafe {
            self.device.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.object_vertex_buffer.buffer],
                &offsets,
            );
            if buffer_data.index_count > 0 {
                self.device.device.cmd_bind_index_buffer(
                    command_buffer,
                    self.object_index_buffer.buffer,
                    buffer_data.index_buffer_offset.into(),
                    IndexType::UINT32,
                );

                self.device.device.cmd_draw_indexed(
                    command_buffer,
                    buffer_data.index_count,
                    1,
                    0,
                    0,
                    0,
                );
            } else {
                self.device
                    .device
                    .cmd_draw(command_buffer, buffer_data.vertex_count, 1, 0, 0);
            }
        }
        Ok(())
    }

    pub fn shader_create(
        &self,
        shader: &mut Shader<'a>,
        renderpass_id: BuiltInRenderpass,
        stage_count: u8,
        stage_filenames: &Vec<String>,
        stages: &Vec<ShaderStage>,
    ) -> Result<()> {
        let max_descriptor_allocate_count: u32 = 1024;

        let mut vulkan_shader_config_stage_count: usize = 0;

        let mut vulkan_shader_config_stages: [VulkanShaderStageConfig; VULKAN_SHADER_MAX_STAGES] =
            [(); VULKAN_SHADER_MAX_STAGES].map(|_| VulkanShaderStageConfig::default());

        for i in 0..stage_count {
            if vulkan_shader_config_stage_count + 1 > VULKAN_SHADER_MAX_STAGES {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "shader reached maximum shader stages",
                    file: file!(),
                    line: line!(),
                });
            }
            let mut stage_flag = ShaderStageFlags::empty();
            match stages[i as usize] {
                ShaderStage::Vertex => stage_flag = ShaderStageFlags::VERTEX,
                ShaderStage::Fragment => stage_flag = ShaderStageFlags::FRAGMENT,
                _ => println!("stage flag unsupported"),
            }

            vulkan_shader_config_stages[vulkan_shader_config_stage_count].stage = stage_flag;
            vulkan_shader_config_stages[vulkan_shader_config_stage_count].file_name =
                stage_filenames[i as usize].clone();
            vulkan_shader_config_stage_count += 1;
        }

        let pool_sizes: [DescriptorPoolSize; 2] = [
            DescriptorPoolSize::default()
                .ty(DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(4096),
            DescriptorPoolSize::default()
                .ty(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(4096),
        ];

        let mut global_descriptor_set_config: VulkanDescriptorSetConfig =
            VulkanDescriptorSetConfig {
                binding_count: 0,
                bindings: [DescriptorSetLayoutBinding::default(); VULKAN_SHADER_MAX_BINDINGS],
            };
        global_descriptor_set_config.bindings[BINDING_INDEX_UBO].binding = BINDING_INDEX_UBO as u32;
        global_descriptor_set_config.bindings[BINDING_INDEX_UBO].descriptor_count = 1;
        global_descriptor_set_config.bindings[BINDING_INDEX_UBO].descriptor_type =
            DescriptorType::UNIFORM_BUFFER;
        global_descriptor_set_config.bindings[BINDING_INDEX_UBO].stage_flags =
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT;
        global_descriptor_set_config.binding_count += 1;

        let mut descriptor_count = 0;
        let mut vulkan_shader_config_descriptor_sets: [VulkanDescriptorSetConfig; 2] =
            [(); 2].map(|_| VulkanDescriptorSetConfig::default());

        vulkan_shader_config_descriptor_sets[DESC_SET_INDEX_GLOBAL] = global_descriptor_set_config;
        descriptor_count += 1;

        if shader.use_instances {
            let mut instance_descriptor_set_config: VulkanDescriptorSetConfig =
                VulkanDescriptorSetConfig {
                    binding_count: 0,
                    bindings: [DescriptorSetLayoutBinding::default(); VULKAN_SHADER_MAX_BINDINGS],
                };
            instance_descriptor_set_config.bindings[BINDING_INDEX_UBO].binding =
                BINDING_INDEX_UBO as u32;
            instance_descriptor_set_config.bindings[BINDING_INDEX_UBO].descriptor_count = 1;
            instance_descriptor_set_config.bindings[BINDING_INDEX_UBO].descriptor_type =
                DescriptorType::UNIFORM_BUFFER;
            instance_descriptor_set_config.bindings[BINDING_INDEX_UBO].stage_flags =
                ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT;
            instance_descriptor_set_config.binding_count += 1;

            vulkan_shader_config_descriptor_sets[DESC_SET_INDEX_INSTANCE] =
                instance_descriptor_set_config;
            descriptor_count += 1;
        }

        let vulkan_shader_config: VulkanShaderConfig = VulkanShaderConfig {
            stage_count: vulkan_shader_config_stage_count as u8,
            stages: vulkan_shader_config_stages,
            pool_sizes,
            max_descriptor_set_count: max_descriptor_allocate_count as u16,
            descriptor_set_count: descriptor_count,
            descriptor_set_configs: vulkan_shader_config_descriptor_sets,
            attributes: [(); VULKAN_SHADER_MAX_ATTRIBUTES]
                .map(|_| VertexInputAttributeDescription::default()),
        };
        let vk_shader = self.shader_initialize(shader, vulkan_shader_config, renderpass_id)?;
        shader.internal_data = ShaderInternalData::Vulkan(vk_shader);
        Ok(())
    }

    pub fn shader_initialize(
        &self,
        shader: &mut Shader,
        mut vulkan_shader_config: VulkanShaderConfig<'a>,
        renderpass_id: BuiltInRenderpass,
    ) -> Result<VulkanShader<'a>> {
        let renderpass = match renderpass_id {
            BuiltInRenderpass::World => &self.main_renderpass,
            BuiltInRenderpass::UI => &self.ui_renderpass,
        };

        let mut vulkan_shader_stages = [VulkanShaderStage {
            shader_mod: ShaderModule::null(),
            shader_stage_create_info: PipelineShaderStageCreateInfo::default(),
        }; VULKAN_SHADER_MAX_STAGES];

        for i in 0..vulkan_shader_config.stage_count as usize {
            vulkan_shader_stages[i] = self.create_shader_module(&vulkan_shader_config, i)?;
        }

        let attr_formats = HashMap::from([
            (ShaderAttributeType::Float32 as u32, Format::R32_SFLOAT),
            (ShaderAttributeType::Float32_2 as u32, Format::R32G32_SFLOAT),
            (
                ShaderAttributeType::Float32_3 as u32,
                Format::R32G32B32_SFLOAT,
            ),
            (
                ShaderAttributeType::Float32_4 as u32,
                Format::R32G32B32A32_SFLOAT,
            ),
            (ShaderAttributeType::Int8 as u32, Format::R8_SINT),
            (ShaderAttributeType::Uint8 as u32, Format::R8_UINT),
            (ShaderAttributeType::Int16 as u32, Format::R16_SINT),
            (ShaderAttributeType::Uint16 as u32, Format::R16_UINT),
            (ShaderAttributeType::Int32 as u32, Format::R32_SINT),
            (ShaderAttributeType::Uint32 as u32, Format::R32_UINT),
        ]);

        let attribute_count = shader.attributes.len();
        let mut offset = 0;
        for i in 0..attribute_count {
            let attribute = VertexInputAttributeDescription::default()
                .binding(0)
                .location(i as u32)
                .offset(offset as u32)
                .format(
                    *attr_formats
                        .get(&(shader.attributes[i].attribute_type as u32))
                        .unwrap(),
                );
            vulkan_shader_config.attributes[i] = attribute;
            offset += shader.attributes[i].size;
        }

        let uniform_count = shader.uniforms.len();
        for i in 0..uniform_count {
            if shader.uniforms[i].uniform_type == ShaderUniformType::Sampler {
                let set_index = if shader.uniforms[i].shader_scope == ShaderScope::Global {
                    DESC_SET_INDEX_GLOBAL
                } else {
                    DESC_SET_INDEX_INSTANCE
                };
                let set_config = &mut vulkan_shader_config.descriptor_set_configs[set_index];
                if set_config.binding_count < 2 {
                    set_config.bindings[BINDING_INDEX_SAMPLER].binding =
                        BINDING_INDEX_SAMPLER as u32;
                    set_config.bindings[BINDING_INDEX_SAMPLER].descriptor_count = 1;
                    set_config.bindings[BINDING_INDEX_SAMPLER].descriptor_type =
                        DescriptorType::COMBINED_IMAGE_SAMPLER;
                    set_config.bindings[BINDING_INDEX_SAMPLER].stage_flags =
                        ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT;
                    set_config.binding_count += 1;
                } else {
                    set_config.bindings[BINDING_INDEX_SAMPLER].descriptor_count += 1;
                }
            }
        }

        let pool_info = DescriptorPoolCreateInfo::default()
            .max_sets(vulkan_shader_config.max_descriptor_set_count as u32)
            .pool_sizes(&vulkan_shader_config.pool_sizes)
            .flags(DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

        let descriptor_pool = unsafe {
            self.device
                .device
                .create_descriptor_pool(&pool_info, None)?
        };

        let mut descriptor_set_layout: [DescriptorSetLayout; 2] =
            [DescriptorSetLayout::default(); 2];

        for (i, set_layout) in descriptor_set_layout.iter_mut().enumerate() {
            let mut layout_info = DescriptorSetLayoutCreateInfo::default();
            layout_info.p_bindings =
                &vulkan_shader_config.descriptor_set_configs[i].bindings as *const _;
            layout_info.binding_count =
                vulkan_shader_config.descriptor_set_configs[i].binding_count as u32;
            *set_layout = unsafe {
                self.device
                    .device
                    .create_descriptor_set_layout(&layout_info, None)?
            };
        }

        let view_port = Viewport::default()
            .height(-(self.framebuffer_height as f32))
            .width(self.framebuffer_width as f32)
            .x(0.0)
            .y(self.framebuffer_height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = Rect2D::default()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(Extent2D {
                width: self.framebuffer_width,
                height: self.framebuffer_height,
            });

        let mut pipeline_create_infos =
            [PipelineShaderStageCreateInfo::default(); VULKAN_SHADER_MAX_STAGES];

        for i in 0..vulkan_shader_config.stage_count as usize {
            pipeline_create_infos[i] = vulkan_shader_stages[i].shader_stage_create_info;
        }

        let pipeline = VulkanPipeline::create(
            &self.device,
            renderpass,
            shader.attribute_stride as u32,
            attribute_count as u32,
            &vulkan_shader_config.attributes,
            2,
            &descriptor_set_layout,
            vulkan_shader_config.stage_count as u32,
            &pipeline_create_infos,
            view_port,
            scissor,
            false,
            true,
            // &shader.push_constant_ranges,
            // shader.push_constant_range_count,
        )?;
        let required_ubo_alignment = self
            .device
            .properties
            .limits
            .min_uniform_buffer_offset_alignment;
        //
        shader.global_ubo_stride = (shader.global_ubo_size + (required_ubo_alignment as usize - 1))
            & !(required_ubo_alignment as usize - 1);
        shader.ubo_stride = (shader.ubo_size + (required_ubo_alignment as usize - 1))
            & !(required_ubo_alignment as usize - 1);
        let total_buffer_size =
            shader.global_ubo_stride + (shader.ubo_stride * VULKAN_MAX_MATERIAL_COUNT);

        let uniform_buffer = VulkanBuffer::create(
            &self.instance,
            &self.device,
            total_buffer_size as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::HOST_VISIBLE
                | MemoryPropertyFlags::HOST_COHERENT
                | MemoryPropertyFlags::DEVICE_LOCAL,
            true,
        )?;

        let mapped_memory =
            uniform_buffer.lock_memory(&self.device, 0, vk::WHOLE_SIZE, MemoryMapFlags::empty())?;

        let global_layouts = [
            descriptor_set_layout[DESC_SET_INDEX_GLOBAL],
            descriptor_set_layout[DESC_SET_INDEX_GLOBAL],
            descriptor_set_layout[DESC_SET_INDEX_GLOBAL],
        ];

        let alloc_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&global_layouts);

        let global_descriptor_sets =
            unsafe { self.device.device.allocate_descriptor_sets(&alloc_info)? };
        let vulkan_shader = VulkanShader {
            mapped_uniform_block: mapped_memory,
            vulkan_shader_config,
            renderpass: renderpass_id,
            stages: vulkan_shader_stages,
            descriptor_pool,
            descriptor_set_layouts: descriptor_set_layout,
            global_descriptor_sets,
            uniform_buffer,
            pipeline,
            instance_count: 0,
            instance_states: [(); VULKAN_MAX_MATERIAL_COUNT]
                .map(|_| VulkanShaderInstanceState::default()),
        };
        Ok(vulkan_shader)
    }

    fn create_shader_module(
        &self,
        vulkan_shader_config: &VulkanShaderConfig,
        index: usize,
    ) -> Result<VulkanShaderStage<'a>> {
        let bin_res = self.resource_system.borrow().load(
            &vulkan_shader_config.stages[index].file_name,
            ResourceType::Binary,
        )?;
        let data = match bin_res.data {
            ResourceData::BinaryResourceData(ref items) => items,
            ResourceData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type. given: Unknown, expected: Binary",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ImageResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type. given: Image, expected: Binary",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MaterialResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type. given: Material, expected: Binary",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::ShaderResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type. given: Shader, expected: Binary",
                    file: file!(),
                    line: line!(),
                });
            }
            ResourceData::MeshResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type. given: Mesh, expected: Binary",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let (_, code, _) = unsafe { data.align_to::<u32>() };

        let module_create_info = ShaderModuleCreateInfo::default().code(code);
        let shader_module = unsafe {
            match self
                .device
                .device
                .create_shader_module(&module_create_info, None)
            {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create shader module".into(),
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        let pipeline_shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(vulkan_shader_config.stages[index].stage)
            .module(shader_module)
            .name(c"main");

        Ok(VulkanShaderStage {
            shader_mod: shader_module,
            shader_stage_create_info: pipeline_shader_stage_create_info,
        })
    }

    pub fn shader_destroy(&self, shader: &mut Shader) -> Result<()> {
        let s = match shader.internal_data {
            ShaderInternalData::Vulkan(ref mut vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        unsafe {
            let _ = self.device.device.device_wait_idle();
        }
        for i in 0..s.vulkan_shader_config.descriptor_set_count as usize {
            unsafe {
                self.device
                    .device
                    .destroy_descriptor_set_layout(s.descriptor_set_layouts[i], None);
            }
        }

        for set_layout in s.descriptor_set_layouts.iter_mut() {
            *set_layout = DescriptorSetLayout::default();
        }

        unsafe {
            self.device
                .device
                .destroy_descriptor_pool(s.descriptor_pool, None);
        }

        s.uniform_buffer.unlock_memory(&self.device);
        s.mapped_uniform_block = ptr::null_mut();
        s.uniform_buffer.destroy(&self.device);

        s.pipeline.destroy(&self.device);

        for i in 0..s.vulkan_shader_config.stage_count as usize {
            unsafe {
                self.device
                    .device
                    .destroy_shader_module(s.stages[i].shader_mod, None);
            }
        }

        for stage in s.stages.iter_mut() {
            stage.shader_mod = ShaderModule::null();
            stage.shader_stage_create_info = PipelineShaderStageCreateInfo::default();
        }

        s.vulkan_shader_config = VulkanShaderConfig::default();
        shader.internal_data = ShaderInternalData::Unknown;

        Ok(())
    }

    pub fn shader_use(&self, shader: &Shader) -> Result<()> {
        let s = match shader.internal_data {
            ShaderInternalData::Vulkan(ref vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        s.pipeline.bind(
            &self.device,
            &self.graphics_cmd_bufs,
            self.in_flight_frames.current_frame,
            PipelineBindPoint::GRAPHICS,
        );

        Ok(())
    }
    pub fn shader_bind_globals(&self, shader: &mut Shader) -> Result<()> {
        shader.bound_ubo_offset = shader.global_ubo_offset;
        Ok(())
    }
    pub fn shader_bind_instance(&self, shader: &mut Shader, instance_id: u32) -> Result<()> {
        shader.bound_instance_id = instance_id as usize;
        //let object_state = &internal_data.instance_states[instance_id as usize];
        shader.bound_ubo_offset = (shader.global_ubo_stride as u64
            + (shader.ubo_stride as u64 * shader.bound_instance_id as u64))
            as usize;
        Ok(())
    }

    pub fn shader_apply_globals(&self, shader: &Shader) -> Result<()> {
        let internal_data = match shader.internal_data {
            ShaderInternalData::Vulkan(ref vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let current_frame = self.in_flight_frames.current_frame;
        let global_descriptor = internal_data.global_descriptor_sets[current_frame];

        let buffer_info = DescriptorBufferInfo::default()
            .buffer(internal_data.uniform_buffer.buffer)
            .offset(shader.global_ubo_offset as u64)
            .range(shader.global_ubo_stride as u64);

        let ubo_write = WriteDescriptorSet::default()
            .buffer_info(std::slice::from_ref(&buffer_info))
            .descriptor_count(1)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .dst_set(global_descriptor);

        let mut descriptor_writes = [WriteDescriptorSet::default(); 2];
        descriptor_writes[0] = ubo_write;
        let mut global_set_binding_count =
            internal_data.vulkan_shader_config.descriptor_set_configs[DESC_SET_INDEX_GLOBAL]
                .binding_count;

        if global_set_binding_count > 1 {
            global_set_binding_count = 0;
            println!("warning: global image samplers are not yet supported")
        }

        unsafe {
            self.device
                .device
                .update_descriptor_sets(&descriptor_writes[0..1], &[]);
        }

        unsafe {
            self.device.device.cmd_bind_descriptor_sets(
                self.graphics_cmd_bufs.command_buffer[self.in_flight_frames.current_frame],
                PipelineBindPoint::GRAPHICS,
                internal_data.pipeline.layout,
                0,
                std::slice::from_ref(&global_descriptor),
                &[],
            );
        }
        Ok(())
    }
    pub fn shader_apply_instance(&self, shader: &mut Shader) -> Result<()> {
        let internal_data = match shader.internal_data {
            ShaderInternalData::Vulkan(ref mut vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let object_state = &mut internal_data.instance_states[shader.bound_instance_id];
        let object_descriptor_set =
            object_state.descriptor_set_state.descriptor_sets[self.in_flight_frames.current_frame];

        let mut descriptor_writes = [WriteDescriptorSet::default(); 2];

        let mut descriptor_count = 0;
        let mut descriptor_index = 0;

        let instance_ubo_generation = &mut object_state.descriptor_set_state.descriptor_states
            [descriptor_index as usize]
            .generations[self.in_flight_frames.current_frame];

        let buffer_info = DescriptorBufferInfo::default()
            .buffer(internal_data.uniform_buffer.buffer)
            .offset(shader.bound_ubo_offset as u64)
            .range(shader.ubo_stride as u64);

        if *instance_ubo_generation == INVALID_ID as u8 {
            let ubo_descriptor = WriteDescriptorSet::default()
                .buffer_info(std::slice::from_ref(&buffer_info))
                .dst_set(object_descriptor_set)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .dst_binding(descriptor_index);
            descriptor_writes[descriptor_count] = ubo_descriptor;
            descriptor_count += 1;
            *instance_ubo_generation = 1;
        }
        descriptor_index += 1;

        if internal_data.vulkan_shader_config.descriptor_set_configs[DESC_SET_INDEX_INSTANCE]
            .binding_count
            > 1
        {
            let total_sampler_count = internal_data.vulkan_shader_config.descriptor_set_configs
                [DESC_SET_INDEX_INSTANCE]
                .bindings[BINDING_INDEX_SAMPLER]
                .descriptor_count;
            let mut update_sampler_count = 0;
            let mut image_infos =
                [DescriptorImageInfo::default(); VULKAN_SHADER_MAX_GLOBAL_TEXTURES];
            for i in 0..total_sampler_count {
                image_infos[i as usize].image_view = if internal_data.instance_states
                    [shader.bound_instance_id]
                    .instance_texture_maps[i as usize]
                    .borrow()
                    .texture
                    .borrow()
                    .internal_data
                    .image
                    .view
                    .is_none()
                {
                    ImageView::null()
                } else {
                    internal_data.instance_states[shader.bound_instance_id].instance_texture_maps
                        [i as usize]
                        .borrow()
                        .texture
                        .borrow()
                        .internal_data
                        .image
                        .view
                        .unwrap()
                };
                image_infos[i as usize].image_layout = ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                image_infos[i as usize].sampler = internal_data.instance_states
                    [shader.bound_instance_id]
                    .instance_texture_maps[i as usize]
                    .borrow()
                    .internal_data;

                update_sampler_count += 1;
            }

            let mut sampler_descriptor = WriteDescriptorSet::default();
            sampler_descriptor.p_image_info = image_infos.as_ptr();
            sampler_descriptor.descriptor_count = update_sampler_count;
            sampler_descriptor.dst_binding = descriptor_index;
            sampler_descriptor.descriptor_type = DescriptorType::COMBINED_IMAGE_SAMPLER;
            sampler_descriptor.dst_set = object_descriptor_set;
            descriptor_writes[descriptor_count] = sampler_descriptor;
            descriptor_count += 1;
        }

        if descriptor_count == 1 {
            unsafe {
                self.device.device.update_descriptor_sets(
                    std::slice::from_ref(descriptor_writes.first().unwrap()),
                    &[],
                );
            }
        } else if descriptor_count == 2 {
            unsafe {
                self.device
                    .device
                    .update_descriptor_sets(&descriptor_writes, &[]);
            }
        }

        unsafe {
            self.device.device.cmd_bind_descriptor_sets(
                self.graphics_cmd_bufs.command_buffer[self.in_flight_frames.current_frame],
                PipelineBindPoint::GRAPHICS,
                internal_data.pipeline.layout,
                1,
                std::slice::from_ref(&object_descriptor_set),
                &[],
            );
        }

        Ok(())
    }

    pub fn set_default_texture(&mut self, default_texture: Rc<RefCell<Texture>>) -> Result<()> {
        self.default_texture = default_texture;
        Ok(())
    }

    pub fn texture_map_acquire_resources(&self, map: &mut TextureMap) -> Result<()> {
        let mut sampler_create_info = SamplerCreateInfo::default();

        sampler_create_info.min_filter = map.filter_minify.into();
        sampler_create_info.mag_filter = map.filter_magnify.into();

        sampler_create_info.address_mode_u = map.repeat_u.into();
        sampler_create_info.address_mode_v = map.repeat_v.into();
        sampler_create_info.address_mode_w = map.repeat_w.into();

        sampler_create_info.anisotropy_enable = vk::TRUE;
        sampler_create_info.max_anisotropy = 16.0;
        sampler_create_info.border_color = BorderColor::INT_OPAQUE_BLACK;
        sampler_create_info.unnormalized_coordinates = vk::FALSE;
        sampler_create_info.compare_enable = vk::FALSE;
        sampler_create_info.compare_op = CompareOp::ALWAYS;
        sampler_create_info.mipmap_mode = SamplerMipmapMode::LINEAR;
        sampler_create_info.mip_lod_bias = 0.0;
        sampler_create_info.min_lod = 0.0;
        sampler_create_info.max_lod = 0.0;

        map.internal_data = unsafe {
            self.device
                .device
                .create_sampler(&sampler_create_info, None)?
        };

        Ok(())
    }

    pub fn texture_map_release_resources(&self, map: &mut TextureMap) -> Result<()> {
        unsafe {
            self.device.device.destroy_sampler(map.internal_data, None);
        }
        map.internal_data = Sampler::null();
        Ok(())
    }

    pub fn shader_acquire_instance_resources(
        &self,
        shader: &mut Shader,
        maps: &Vec<&TextureMap>,
    ) -> Result<u32> {
        let internal_data = match shader.internal_data {
            ShaderInternalData::Vulkan(ref mut vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let mut instance_id: u32 = INVALID_ID as u32;
        for (i, state) in internal_data.instance_states.iter_mut().enumerate() {
            if state.id == INVALID_ID as u32 {
                state.id = i as u32;
                instance_id = i as u32;
                break;
            }
        }

        if instance_id == INVALID_ID as u32 {
            return Err(VulkanBackendError::OperationFailed {
                issue: "failed to acquire resources for vulkan shader",
                file: file!(),
                line: line!(),
            });
        }

        let instance_state = &mut internal_data.instance_states[instance_id as usize];

        let instance_texture_map_count = internal_data.vulkan_shader_config.descriptor_set_configs
            [DESC_SET_INDEX_INSTANCE]
            .bindings[BINDING_INDEX_SAMPLER]
            .descriptor_count;

        instance_state
            .instance_texture_maps
            .reserve(instance_texture_map_count as usize);
        // NOTE: not sure if to use instance_texture_map_count or map.len()
        for i in 0..instance_texture_map_count {
            instance_state
                .instance_texture_maps
                .push(Rc::new(RefCell::new((*maps[i as usize]).clone())));
            instance_state.instance_texture_maps[i as usize]
                .borrow_mut()
                .texture = Rc::clone(&self.default_texture);
        }

        let set_state = &mut instance_state.descriptor_set_state;

        let binding_count = internal_data.vulkan_shader_config.descriptor_set_configs
            [DESC_SET_INDEX_INSTANCE]
            .binding_count;

        for i in 0..binding_count as usize {
            set_state.descriptor_states[i]
                .generations
                .iter_mut()
                .map(|generation| *generation = INVALID_ID as u8)
                .count();
            set_state.descriptor_states[i]
                .ids
                .iter_mut()
                .map(|id| *id = INVALID_ID as u32)
                .count();
        }

        let layouts = [
            internal_data.descriptor_set_layouts[DESC_SET_INDEX_INSTANCE],
            internal_data.descriptor_set_layouts[DESC_SET_INDEX_INSTANCE],
            internal_data.descriptor_set_layouts[DESC_SET_INDEX_INSTANCE],
        ];

        let alloc_info = DescriptorSetAllocateInfo::default()
            .set_layouts(&layouts)
            .descriptor_pool(internal_data.descriptor_pool);

        instance_state.descriptor_set_state.descriptor_sets = unsafe {
            match self.device.device.allocate_descriptor_sets(&alloc_info) {
                Ok(ds) => ds.as_slice().try_into().unwrap(),
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not allocate descriptor sets",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        Ok(instance_id)
    }
    pub fn shader_release_instance_resources(
        &self,
        shader: &mut Shader,
        instance_id: u32,
    ) -> Result<()> {
        let internal_data = match shader.internal_data {
            ShaderInternalData::Vulkan(ref mut vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let instance_state = &mut internal_data.instance_states[instance_id as usize];

        let _ = unsafe {
            let _ = self.device.device.device_wait_idle();
            self.device
                .device
                .free_descriptor_sets(
                    internal_data.descriptor_pool,
                    &instance_state.descriptor_set_state.descriptor_sets,
                )
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not free descriptor sets",
                    file: file!(),
                    line: line!(),
                })?;
        };
        instance_state.descriptor_set_state = VulkanShaderDescriptorSetState::default();
        instance_state.instance_texture_maps.clear();
        instance_state.offset = INVALID_ID as u64;
        instance_state.id = INVALID_ID as u32;
        Ok(())
    }
    pub fn set_uniform(
        &self,
        shader: &mut Shader,
        uniform_index: usize,
        value: *const c_void,
    ) -> Result<()> {
        let internal_data = match shader.internal_data {
            ShaderInternalData::Vulkan(ref mut vulkan_shader) => vulkan_shader,
            ShaderInternalData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "unknown shader internal data",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let uniform = shader.uniforms[uniform_index];
        if uniform.uniform_type == ShaderUniformType::Sampler {
            if uniform.shader_scope == ShaderScope::Global {
                let tex_map_ptr = value as *const TextureMap;
                let tex_map: TextureMap = unsafe { (*tex_map_ptr).clone() };
                shader.global_texture_maps[uniform.location as usize] =
                    Rc::new(RefCell::new(tex_map));
            } else {
                let tex_map_ptr = value as *const TextureMap;
                let tex_map: TextureMap = unsafe { (*tex_map_ptr).clone() };
                internal_data.instance_states[shader.bound_instance_id].instance_texture_maps
                    [uniform.location as usize] = Rc::new(RefCell::new(tex_map));
            }
        } else {
            if uniform.shader_scope == ShaderScope::Local {
                unsafe {
                    self.device.device.cmd_push_constants(
                        self.graphics_cmd_bufs.command_buffer[self.in_flight_frames.current_frame],
                        internal_data.pipeline.layout,
                        ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                        uniform.offset as u32,
                        std::slice::from_raw_parts(value as *const _ as *const u8, uniform.size),
                    );
                }
            } else {
                if uniform.shader_scope == ShaderScope::Global {
                    let mut addr = internal_data.mapped_uniform_block;
                    unsafe {
                        addr = addr.add(shader.global_ubo_offset + uniform.offset);
                        ptr::copy_nonoverlapping(value as *const _, addr.cast(), uniform.size);
                    }
                } else if uniform.shader_scope == ShaderScope::Instance {
                    let mut addr = internal_data.mapped_uniform_block;
                    unsafe {
                        addr = addr.add(shader.bound_ubo_offset + uniform.offset);
                        ptr::copy_nonoverlapping(value as *const _, addr.cast(), uniform.size);
                    }
                }
            }
        }
        Ok(())
    }

    fn upload_data_range<T>(
        instance: &Instance,
        device: &VulkanDevice,
        pool: CommandPool,
        fence: Fence,
        queue: Queue,
        buffer: &VulkanBuffer,
        offset: u64,
        data: &[T],
    ) -> Result<()> {
        let memory_flags = MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT;
        let staging_buffer = VulkanBuffer::create(
            instance,
            device,
            size_of_val(data) as u64,
            BufferUsageFlags::TRANSFER_SRC,
            memory_flags,
            true,
        )?;

        staging_buffer.load_data(
            device,
            0,
            size_of_val(data) as u64,
            MemoryMapFlags::empty(),
            data,
        )?;

        VulkanBuffer::copy_to(
            device,
            pool,
            fence,
            queue,
            staging_buffer.buffer,
            0,
            buffer.buffer,
            offset,
            size_of_val(data) as u64,
        )?;

        staging_buffer.destroy(device);

        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        {
            unsafe {
                let _ = self.device.device.device_wait_idle();
                self.object_index_buffer.destroy(&self.device);
                self.object_vertex_buffer.destroy(&self.device);
                for frame in &self.swapchain_framebuffers {
                    self.device.device.destroy_framebuffer(*frame, None);
                }
                for frame in &self.world_framebuffers {
                    self.device.device.destroy_framebuffer(*frame, None);
                }
                self.in_flight_frames.destroy(&self.device);
                self.device
                    .device
                    .destroy_command_pool(self.device.graphics_command_pool, None);
                self.ui_renderpass.destroy(&self.device);
                self.main_renderpass.destroy(&self.device);
                self.swapchain.destroy(&self.device);
                self.device.device.destroy_device(None);
                self.surface_loader.destroy_surface(self.surface, None);
                self.dbg_instance_loader
                    .destroy_debug_utils_messenger(self.dbg_messenger, None);
                self.instance.destroy_instance(None);
            }
        }
        #[cfg(not(feature = "debug"))]
        {
            unsafe {
                let _ = self.device.device.device_wait_idle();
                self.object_index_buffer.destroy(&self.device);
                self.object_vertex_buffer.destroy(&self.device);
                for frame in &self.swapchain_framebuffers {
                    self.device.device.destroy_framebuffer(*frame, None);
                }
                for frame in &self.world_framebuffers {
                    self.device.device.destroy_framebuffer(*frame, None);
                }
                self.in_flight_frames.destroy(&self.device);
                self.device
                    .device
                    .destroy_command_pool(self.device.graphics_command_pool, None);
                self.ui_renderpass.destroy(&self.device);
                self.main_renderpass.destroy(&self.device);
                self.swapchain.destroy(&self.device);
                self.device.device.destroy_device(None);
                self.surface_loader.destroy_surface(self.surface, None);
                self.instance.destroy_instance(None);
            }
        }
    }
}

#[cfg(feature = "debug")]
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    unsafe {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );
    }
    vk::FALSE
}
