use super::{
    super::vulkan::shaders::vulkan_material_shader::VulkanMaterialShader,
    vulkan_buffer::VulkanBuffer,
    vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice,
    vulkan_renderpass::VulkanRenderPass,
    vulkan_swapchain::VulkanSwapchain,
    vulkan_sync_objects::{InFlightFrames, SyncObjects},
};
use crate::application::{
    basic::{
        math::{
            consts::INVALID_ID,
            matrix4::Matrix4,
            vec3::{Vec3, Vector3D},
            vec4::Vec4,
        },
        window::Window,
    },
    renderer::{
        renderer_types::GeometryRenderData,
        vulkan::{shaders::vulkan_ui_shader::VulkanUIshader, vulkan_renderpass::ClearFlag},
    },
    resources::resource_types::{Geometry, Material, MaterialType, Texture},
    systems::resource_system::ResourceSysError,
};

use ash::{
    Entry, Instance, LoadingError,
    khr::{
        surface::{self, Instance as SurfaceInstance},
        xlib_surface,
    },
    vk::{
        self, BorderColor, BufferUsageFlags, CommandPool, CommandPoolResetFlags, CompareOp,
        DeviceSize, Extent2D, Fence, Filter, Format, Framebuffer, FramebufferCreateInfo,
        ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, IndexType,
        MemoryMapFlags, MemoryPropertyFlags, Offset2D, PipelineStageFlags, Queue, Rect2D,
        SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, SubmitInfo, SurfaceKHR, Viewport,
    },
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
use std::ffi::{CString, NulError, c_void};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};
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
        source: ash::vk::Result,
    },
    #[error("{source}\nvulkan backend error: resource sys error")]
    ResourceError {
        #[from]
        source: ResourceSysError,
    },
}

const VULKAN_MAX_GEOMETRY_COUNT: usize = 100;

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

pub enum BuiltInRenderpass {
    World,
    UI,
}

pub struct VulkanContext<'a> {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
    frame_delta_time: f32,
    geometry_vertex_offset: u64,
    geometry_index_offset: u64,
    geometries: Vec<VulkanGeometryData>,
    object_index_buffer: VulkanBuffer,
    object_vertex_buffer: VulkanBuffer,
    material_shader: VulkanMaterialShader<'a>,
    ui_shader: VulkanUIshader<'a>,
    images_in_flight: Vec<Option<&'a SyncObjects>>,
    in_flight_frames: InFlightFrames,
    graphics_cmd_bufs: VulkanCommandBuffer,
    swapchain_framebuffers: Vec<Framebuffer>,
    world_framebuffers: Vec<Framebuffer>,
    main_renderpass: VulkanRenderPass,
    ui_renderpass: VulkanRenderPass,
    image_index: u32,
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

static mut VULKAN_STATE: Option<VulkanContext> = None;

impl<'a> VulkanContext<'a> {
    pub fn initialize(name: &str, window: &Window) -> Result<()> {
        unsafe {
            if let Some(ref _state) = VULKAN_STATE {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context was already initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        }

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
        for i in 0..swap.views.len() {
            let mut world_attachments = Vec::new();
            world_attachments.push(swap.views[i]);
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
            ui_attachments.push(swap.views[i]);

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
            images_in_flight.push(None);
        }

        let in_flight_frames = InFlightFrames::new(sync_objects);

        let material_shader = VulkanMaterialShader::create(
            &instance,
            &dev,
            &main_renderpass,
            window.width,
            window.height,
            swap.max_frames_in_flight as u32,
        )?;

        let ui_shader = VulkanUIshader::create(
            &instance,
            &dev,
            &ui_renderpass,
            window.width,
            window.height,
            swap.max_frames_in_flight as u32,
        )?;

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

            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_messenger =
                unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None)? };
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
                    frame_delta_time: 0.0,
                    geometry_vertex_offset: 0,
                    geometry_index_offset: 0,
                    geometries: geometries,
                    object_index_buffer: index_buffer,
                    object_vertex_buffer: vertex_buffer,
                    material_shader: material_shader,
                    images_in_flight: images_in_flight,
                    image_index: 0,
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
                    ui_shader: ui_shader,
                    dbg_messenger: debug_messenger,
                    dbg_util_loader: debug_utils_loader,
                })
            };
        }

        #[cfg(not(feature = "debug"))]
        {
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
                    frame_delta_time: 0.0,
                    geometry_vertex_offset: 0,
                    geometry_index_offset: 0,
                    geometries: geometries,
                    object_index_buffer: index_buffer,
                    object_vertex_buffer: vertex_buffer,
                    material_shader: material_shader,
                    images_in_flight: images_in_flight,
                    image_index: 0,
                    recreating_swapchain: false,
                    in_flight_frames,
                    graphics_cmd_bufs: graph_cmd_buf,
                    device: dev,
                    instance: instance,
                    surface: surface,
                    surface_loader: surface_loader,
                    swapchain: swap,
                    main_renderpass: main_renderpass,
                    ui_renderpass: ui_renderpass,
                    framebuffer_height: window.height,
                    framebuffer_width: window.width,
                    frame_buffer_size_generation: 0,
                    frame_buffer_last_generation: 0,
                    swapchain_framebuffers: swap_framebuffers,
                    world_framebuffers: world_framebuffers,
                    ui_shader: ui_shader,
                });
            }
        }
        println!("Vulkan Instance created");
        Ok(())
    }

    pub fn shutdown() -> Result<()> {
        unsafe {
            if let Some(ref _state) = VULKAN_STATE {
                VULKAN_STATE = None;
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context already destroyed",
                    file: file!(),
                    line: line!(),
                });
            }
        }

        Ok(())
    }
    pub fn on_resize(width: i32, height: i32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context already destroyed",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        state.framebuffer_height = height as u32;
        state.framebuffer_width = width as u32;
        state.frame_buffer_last_generation += 1;

        println!(
            "renderer backend resized w: {}, h: {}, gen: {}",
            state.framebuffer_width, state.framebuffer_height, state.frame_buffer_last_generation
        );

        Ok(())
    }
    pub fn begin_frame(delta: f32) -> Result<bool> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context already destroyed",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state.frame_delta_time = delta;
        if state.recreating_swapchain {
            match unsafe { state.device.device.device_wait_idle() } {
                Ok(_) => return Ok(false),
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not wait on device",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }

        if state.frame_buffer_last_generation != state.frame_buffer_size_generation {
            match unsafe { state.device.device.device_wait_idle() } {
                Ok(_) => match Self::recreate_swapchain() {
                    Ok(()) => return Ok(false),
                    Err(e) => return Err(e),
                },
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not wait on device",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }
        let sync = &state.in_flight_frames.sync_objs;
        let current_frame = state.in_flight_frames.current_frame;

        if !sync[current_frame].fence_wait(&state.device, u64::MAX)? {
            return Ok(false);
        }

        sync[current_frame].reset_fence(&state.device)?;

        state.image_index = match state.swapchain.acquire_next_image_index(
            u64::MAX,
            sync[current_frame].image_avail_semaphore,
            sync[current_frame].fence,
        ) {
            Ok((suboptimal, index)) => {
                if suboptimal {
                    match Self::recreate_swapchain() {
                        Ok(()) => return Ok(false),
                        Err(e) => return Err(e),
                    }
                } else {
                    index
                }
            }
            Err(e) => return Err(e),
        };

        if let Some(s) = state.images_in_flight[state.image_index as usize] {
            if s.fence != Fence::null() {
                if !s.fence_wait(&state.device, u64::MAX)? {
                    return Ok(false);
                }
            }
        }

        state.images_in_flight[state.image_index as usize] =
            Some(&state.in_flight_frames.sync_objs[current_frame as usize]);

        let command_buffer = &mut state.graphics_cmd_bufs;
        command_buffer.reset(&state.device, current_frame)?;
        command_buffer.begin(&state.device, false, false, false, current_frame as usize)?;

        let viewport = Viewport::default()
            .x(0.0)
            .y(state.framebuffer_height as f32)
            .height(-(state.framebuffer_height as f32))
            .width(state.framebuffer_width as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = Rect2D::default()
            .extent(
                Extent2D::default()
                    .height(state.framebuffer_height)
                    .width(state.framebuffer_width),
            )
            .offset(Offset2D::default());

        unsafe {
            state.device.device.cmd_set_viewport(
                command_buffer.command_buffer[current_frame as usize],
                0,
                std::slice::from_ref(&viewport),
            );
        };
        unsafe {
            state.device.device.cmd_set_scissor(
                command_buffer.command_buffer[current_frame as usize],
                0,
                std::slice::from_ref(&scissor),
            )
        };

        state.material_shader.use_shader(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        );

        Ok(true)
    }

    pub fn update_global_world_state(
        projection: Matrix4,
        view: Matrix4,
        _view_position: Vec3,
        _ambient_colour: Vec4,
        _mode: i32,
    ) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state.material_shader.use_shader(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        );

        state.material_shader.global_ubo.projection = projection;
        state.material_shader.global_ubo.view = view;

        state.material_shader.update_global_state(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
            state.frame_delta_time,
        )?;

        Ok(())
    }

    pub fn update_global_ui_state(projection: Matrix4, view: Matrix4, _mode: i32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        state.ui_shader.use_shader(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        );

        state.ui_shader.global_ubo.projection = projection;
        state.ui_shader.global_ubo.view = view;

        state.ui_shader.update_global_state(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
            state.frame_delta_time,
        )?;

        Ok(())
    }

    pub fn draw_geometry(data: &mut GeometryRenderData) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let mut geo = data.geometry.borrow_mut();

        match geo.material.as_ref().unwrap().material_type {
            MaterialType::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "material type was unknown",
                    file: file!(),
                    line: line!(),
                });
            }
            MaterialType::World => {
                state.material_shader.set_model(
                    &state.device,
                    &state.graphics_cmd_bufs,
                    state.in_flight_frames.current_frame as u32,
                    data.model,
                )?;
                state.material_shader.apply_material(
                    &state.device,
                    &state.graphics_cmd_bufs,
                    state.in_flight_frames.current_frame as u32,
                    geo.material.as_mut().unwrap(),
                )?;
            }
            MaterialType::UI => {
                state.ui_shader.set_model(
                    &state.device,
                    &state.graphics_cmd_bufs,
                    state.in_flight_frames.current_frame as u32,
                    data.model,
                )?;
                state.ui_shader.apply_material(
                    &state.device,
                    &state.graphics_cmd_bufs,
                    state.in_flight_frames.current_frame as u32,
                    geo.material.as_mut().unwrap(),
                )?;
            }
        }

        let buffer_data = &state.geometries[geo.internal_id];

        let offsets: [DeviceSize; 1] = [buffer_data.vertex_buffer_offset.into()];
        unsafe {
            state.device.device.cmd_bind_vertex_buffers(
                state.graphics_cmd_bufs.command_buffer
                    [state.in_flight_frames.current_frame as usize],
                0,
                &[state.object_vertex_buffer.buffer],
                &offsets,
            );
            if buffer_data.index_count > 0 {
                state.device.device.cmd_bind_index_buffer(
                    state.graphics_cmd_bufs.command_buffer
                        [state.in_flight_frames.current_frame as usize],
                    state.object_index_buffer.buffer,
                    buffer_data.index_buffer_offset.into(),
                    IndexType::UINT32,
                );

                state.device.device.cmd_draw_indexed(
                    state.graphics_cmd_bufs.command_buffer
                        [state.in_flight_frames.current_frame as usize],
                    buffer_data.index_count,
                    1,
                    0,
                    0,
                    0,
                );
            } else {
                state.device.device.cmd_draw(
                    state.graphics_cmd_bufs.command_buffer
                        [state.in_flight_frames.current_frame as usize],
                    buffer_data.vertex_count,
                    1,
                    0,
                    0,
                );
            }
        }

        Ok(())
    }

    pub fn create_texture(pixels: &[u8], texture: &mut Texture) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let image_size: DeviceSize = (texture.width as u64
            * texture.height as u64
            * texture.channel_count as u64) as DeviceSize;
        let image_format = Format::R8G8B8A8_UNORM;
        let usage = BufferUsageFlags::TRANSFER_SRC;
        let memory_property_flags =
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT;
        let staging = VulkanBuffer::create(
            &state.instance,
            &state.device,
            image_size,
            usage,
            memory_property_flags,
            true,
        )?;

        staging.load_data(
            &state.device,
            0,
            image_size,
            MemoryMapFlags::empty(),
            pixels,
        )?;

        texture.internal_data.image = super::vulkan_image::VulkanImage::create(
            &state.instance,
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
            &state.device,
        )?;

        let mut temp_command_buffer = VulkanCommandBuffer::allocate_and_begin_single_use(
            &state.device,
            state.device.graphics_command_pool,
        )?;

        texture.internal_data.image.transition_layout(
            &state.device,
            &temp_command_buffer,
            image_format,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            0,
        )?;

        texture.internal_data.image.copy_from_buffer(
            &state.device,
            &staging,
            &temp_command_buffer,
            0,
        );

        texture.internal_data.image.transition_layout(
            &state.device,
            &temp_command_buffer,
            image_format,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            0,
        )?;

        temp_command_buffer.end_single_use(
            &state.device,
            state.device.graphics_command_pool,
            state.device.graphics_queue,
        )?;

        staging.destroy(&state.device);

        let sampler_info = SamplerCreateInfo::default()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .address_mode_u(SamplerAddressMode::REPEAT)
            .address_mode_v(SamplerAddressMode::REPEAT)
            .address_mode_w(SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(CompareOp::ALWAYS)
            .mipmap_mode(SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        texture.internal_data.sampler = unsafe {
            match state.device.device.create_sampler(&sampler_info, None) {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create sampler",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        Ok(())
    }

    pub fn destroy_texture(texture: &Texture) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let _ = unsafe { state.device.device.device_wait_idle() };

        texture.internal_data.image.destroy(&state.device);
        unsafe {
            state
                .device
                .device
                .destroy_sampler(texture.internal_data.sampler, None)
        };
        Ok(())
    }

    pub fn create_material(material: &mut Material) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        match material.material_type {
            MaterialType::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "material type was unknown",
                    file: file!(),
                    line: line!(),
                });
            }
            MaterialType::World => state
                .material_shader
                .acquire_resources(&state.device, material)?,
            MaterialType::UI => state.ui_shader.acquire_resources(&state.device, material)?,
        }

        Ok(())
    }

    pub fn destroy_material(material: &Material) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        match material.material_type {
            MaterialType::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "material type was unknown",
                    file: file!(),
                    line: line!(),
                });
            }
            MaterialType::World => state
                .material_shader
                .release_resources(&state.device, material)?,
            MaterialType::UI => state.ui_shader.release_resources(&state.device, material)?,
        }

        Ok(())
    }

    pub fn create_geometry<T: Clone, U: Clone>(
        geometry: &mut Geometry<'a>,
        vertices: &[T],
        indices: &[U],
    ) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let is_reupload = geometry.internal_id != INVALID_ID;
        let mut old_range = VulkanGeometryData::default();
        let mut internal_data: Option<&mut VulkanGeometryData> = None;
        if is_reupload {
            internal_data = Some(&mut state.geometries[geometry.internal_id]);
            let int_data = internal_data.as_mut().unwrap();
            old_range.index_buffer_offset = int_data.index_buffer_offset;
            old_range.index_count = int_data.index_count;
            old_range.index_size = int_data.index_size;
            old_range.vertex_buffer_offset = int_data.vertex_buffer_offset;
            old_range.vertex_count = int_data.vertex_count;
            old_range.vertex_size = int_data.vertex_size;
        } else {
            for (i, geo) in state.geometries.iter_mut().enumerate() {
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

        let command_pool = state.device.graphics_command_pool;
        let queue = state.device.graphics_queue;
        let int_data = internal_data.unwrap();
        int_data.vertex_buffer_offset = state.geometry_vertex_offset as u32;
        int_data.vertex_count = vertices.len() as u32;
        int_data.vertex_size = vertices.len() as u32 * size_of::<T>() as u32;
        Self::upload_data_range(
            &state.instance,
            &state.device,
            command_pool,
            Fence::null(),
            queue,
            &state.object_vertex_buffer,
            state.geometry_vertex_offset,
            vertices,
        )?;

        state.geometry_vertex_offset += int_data.vertex_size as u64;

        if indices.len() > 0 {
            int_data.index_buffer_offset = state.geometry_index_offset as u32;
            int_data.index_count = indices.len() as u32;
            int_data.index_size = indices.len() as u32 * size_of::<U>() as u32;

            Self::upload_data_range(
                &state.instance,
                &state.device,
                command_pool,
                Fence::null(),
                queue,
                &state.object_index_buffer,
                state.geometry_index_offset,
                indices,
            )?;
            state.geometry_index_offset += int_data.index_size as u64;
        }
        if int_data.generation == INVALID_ID {
            int_data.generation = 0;
        } else {
            int_data.generation += 1;
        }
        // NEXT: create a free list of offsets that were freed
        Ok(())
    }

    pub fn destroy_geometry(geometry: &Geometry<'a>) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        if geometry.internal_id != INVALID_ID {
            state.geometries[geometry.internal_id] = VulkanGeometryData::default();
        }
        Ok(())
    }

    pub fn end_frame(_delta: f32) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let image_index = state.image_index;
        let command_buff = &mut state.graphics_cmd_bufs;

        command_buff.end(&state.device, state.in_flight_frames.current_frame as usize)?;

        if let Some(sync_obj) = state.images_in_flight[state.image_index as usize] {
            match sync_obj.fence_wait(&state.device, u64::MAX) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        state.images_in_flight[image_index as usize] =
            Some(&state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]);

        // should handle bool here
        state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
            .fence_wait(&state.device, u64::MAX)?;

        state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
            .reset_fence(&state.device)?;

        let submit_info = SubmitInfo::default()
            .command_buffers(std::slice::from_ref(
                &state.graphics_cmd_bufs.command_buffer
                    [state.in_flight_frames.current_frame as usize],
            ))
            .signal_semaphores(std::slice::from_ref(
                &state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame as usize]
                    .render_finished_semaphore,
            ))
            .wait_semaphores(std::slice::from_ref(
                &state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame as usize]
                    .image_avail_semaphore,
            ))
            .wait_dst_stage_mask(std::slice::from_ref(
                &PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ));

        unsafe {
            state.device.device.queue_submit(
                state.device.graphics_queue,
                std::slice::from_ref(&submit_info),
                state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame as usize]
                    .fence,
            )?;
        }

        state.swapchain.present(
            &state.device.graphics_queue,
            &state.device.present_queue,
            &state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
                .render_finished_semaphore,
            state.image_index,
        )?;

        state.in_flight_frames.current_frame = (state.in_flight_frames.current_frame + 1)
            % state.swapchain.max_frames_in_flight as usize;

        Ok(())
    }

    pub fn begin_renderpass(renderpass_type: BuiltInRenderpass) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };

        let (renderpass, framebuffer) = match renderpass_type {
            BuiltInRenderpass::World => (
                &state.main_renderpass,
                state.world_framebuffers[state.image_index as usize],
            ),
            BuiltInRenderpass::UI => (
                &state.ui_renderpass,
                state.swapchain_framebuffers[state.image_index as usize],
            ),
        };
        renderpass.begin(
            &state.device,
            &mut state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame,
            framebuffer,
        );
        match renderpass_type {
            BuiltInRenderpass::World => state.material_shader.use_shader(
                &state.device,
                &state.graphics_cmd_bufs,
                state.in_flight_frames.current_frame as u32,
            ),
            BuiltInRenderpass::UI => state.ui_shader.use_shader(
                &state.device,
                &state.graphics_cmd_bufs,
                state.in_flight_frames.current_frame as u32,
            ),
        }
        Ok(())
    }

    pub fn end_renderpass(renderpass_type: BuiltInRenderpass) -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let renderpass = match renderpass_type {
            BuiltInRenderpass::World => &state.main_renderpass,
            BuiltInRenderpass::UI => &state.ui_renderpass,
        };
        renderpass.end(
            &state.device,
            &mut state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame,
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

    pub fn recreate_swapchain() -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        println!("recreating swapchain");
        let _ = unsafe { state.device.device.device_wait_idle() };
        for i in 0..state.swapchain.image_count as usize {
            state.images_in_flight[i] = None;
        }

        state.in_flight_frames.destroy(&state.device);
        let mut sync_objs = Vec::new();
        for _ in 0..state.swapchain.max_frames_in_flight {
            sync_objs.push(SyncObjects::create(&state.device, true)?);
        }
        state.in_flight_frames = InFlightFrames::new(sync_objs);

        state.recreating_swapchain = true;
        //state.swapchain.destroy(&state.device);
        state.swapchain.recreate(
            &state.instance,
            &mut state.device,
            &state.surface,
            &state.surface_loader,
            state.framebuffer_width,
            state.framebuffer_height,
        )?;

        state
            .main_renderpass
            .render_area
            .set_w(state.framebuffer_width as f32);
        state
            .main_renderpass
            .render_area
            .set_h(state.framebuffer_height as f32);

        state.frame_buffer_last_generation = state.frame_buffer_size_generation;

        unsafe {
            state.device.device.reset_command_pool(
                state.device.graphics_command_pool,
                CommandPoolResetFlags::empty(),
            )?
        }

        for i in 0..state.swapchain.image_count as usize {
            unsafe {
                state
                    .device
                    .device
                    .destroy_framebuffer(state.swapchain_framebuffers[i], None);
                state
                    .device
                    .device
                    .destroy_framebuffer(state.world_framebuffers[i], None);
            }
        }

        Self::regenerate_framebuffers()?;

        state.recreating_swapchain = false;

        Ok(())
    }

    fn create_command_buffer(device: &VulkanDevice, frames: usize) -> Result<VulkanCommandBuffer> {
        VulkanCommandBuffer::allocate(device, true, device.graphics_command_pool, frames as u32)
    }

    fn regenerate_framebuffers() -> Result<()> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Vulkan Context not initialized",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let mut world_framebuffers = Vec::new();
        let mut swap_framebuffers = Vec::new();
        for i in 0..state.swapchain.views.len() {
            let mut world_attachments = Vec::new();
            world_attachments.push(state.swapchain.views[i]);
            world_attachments.push(state.swapchain.depth_attachment.view.unwrap());
            let framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(state.main_renderpass.renderpass)
                .attachments(&world_attachments)
                .height(state.framebuffer_height)
                .width(state.framebuffer_width)
                .layers(1);

            let framebuffer = unsafe {
                match state
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
            ui_attachments.push(state.swapchain.views[i]);

            let ui_framebuffer_create_info = FramebufferCreateInfo::default()
                .render_pass(state.ui_renderpass.renderpass)
                .attachments(&ui_attachments)
                .height(state.framebuffer_height)
                .width(state.framebuffer_width)
                .layers(1);

            let ui_framebuffer = unsafe {
                match state
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
        state.world_framebuffers = world_framebuffers;
        state.swapchain_framebuffers = swap_framebuffers;
        Ok(())
    }

    fn create_buffers(
        instance: &Instance,
        device: &VulkanDevice,
    ) -> Result<(VulkanBuffer, VulkanBuffer)> {
        let memory_property_flag = MemoryPropertyFlags::DEVICE_LOCAL;

        const VERTEX_BUFFER_SIZE: usize = size_of::<Vector3D>() * 1024;

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

        const INDEX_BUFFER_SIZE: usize = size_of::<u32>() * 1024;

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

impl<'a> Drop for VulkanContext<'a> {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                let _ = state.device.device.device_wait_idle();
                state.object_index_buffer.destroy(&state.device);
                state.object_vertex_buffer.destroy(&state.device);
                for frame in &state.swapchain_framebuffers {
                    state.device.device.destroy_framebuffer(*frame, None);
                }
                for frame in &state.world_framebuffers {
                    state.device.device.destroy_framebuffer(*frame, None);
                }
                state.ui_shader.destroy(&state.device);
                state.material_shader.destroy(&state.device);
                state.in_flight_frames.destroy(&state.device);
                state
                    .device
                    .device
                    .destroy_command_pool(state.device.graphics_command_pool, None);
                state.ui_renderpass.destroy(&state.device);
                state.main_renderpass.destroy(&state.device);
                state.swapchain.destroy(&state.device);
                state.device.device.destroy_device(None);
                state.surface_loader.destroy_surface(state.surface, None);
                state
                    .dbg_util_loader
                    .destroy_debug_utils_messenger(state.dbg_messenger, None);
                state.instance.destroy_instance(None);
            }
        }
        #[cfg(not(feature = "debug"))]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                let _ = state.device.device.device_wait_idle();
                state.object_index_buffer.destroy(&state.device);
                state.object_vertex_buffer.destroy(&state.device);
                for frame in &state.swapchain_framebuffers {
                    state.device.device.destroy_framebuffer(*frame, None);
                }
                for frame in &state.world_framebuffers {
                    state.device.device.destroy_framebuffer(*frame, None);
                }
                state.ui_shader.destroy(&state.device);
                state.material_shader.destroy(&state.device);
                state.in_flight_frames.destroy(&state.device);
                state
                    .device
                    .device
                    .destroy_command_pool(state.device.graphics_command_pool, None);
                state.ui_renderpass.destroy(&state.device);
                state.main_renderpass.destroy(&state.device);
                state.swapchain.destroy(&state.device);
                state.device.device.destroy_device(None);
                state.surface_loader.destroy_surface(state.surface, None);
                state.instance.destroy_instance(None);
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
