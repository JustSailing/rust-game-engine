use ash::{
    Entry, Instance,
    khr::{
        surface::{self, Instance as SurfaceInstance},
        xlib_surface,
    },
    vk::{
        self, BufferUsageFlags, CommandPool, DeviceSize, Extent2D, Fence, IndexType,
        MemoryMapFlags, MemoryPropertyFlags, Offset2D, PipelineStageFlags, Queue, Rect2D,
        SubmitInfo, SurfaceKHR, Viewport,
    },
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
use std::ffi::{CString, c_void};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};

use super::{
    super::vulkan::shaders::vulkan_object_shader::VulkanObjectShader,
    vulkan_buffer::VulkanBuffer,
    vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice,
    vulkan_framebuffer::VulkanFramebuffer,
    vulkan_renderpass::VulkanRenderPass,
    vulkan_swapchain::VulkanSwapchain,
    vulkan_sync_objects::{InFlightFrames, SyncObjects},
};
use crate::application::basic::{
    math::{
        matrix4::Matrix4,
        vec3::{Vec3, Vector3D},
        vec4::Vec4,
    },
    window::Window,
};

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext<'a> {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
    geometry_vertex_offset: u64,
    geometry_index_offset: u64,
    object_index_buffer: VulkanBuffer,
    object_vertex_buffer: VulkanBuffer,
    object_shader: VulkanObjectShader<'a>,
    images_in_flight: Vec<Option<&'a SyncObjects>>,
    in_flight_frames: InFlightFrames,
    graphics_cmd_bufs: VulkanCommandBuffer,
    swapchain_framebuffers: Vec<VulkanFramebuffer>,
    main_renderpass: VulkanRenderPass,
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

impl<'a> VulkanContext<'a> {
    pub fn initialize(name: &str, window: &Window) -> Result<(), VulkanError> {
        unsafe {
            if let Some(ref _state) = VULKAN_STATE {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context was already initialized",
                ));
            }
        }

        let entry = unsafe {
            match Entry::load() {
                Ok(e) => e,
                Err(_) => return Err(VulkanError::OperationFailed("Could not load entry")),
            }
        };

        let app_name = match CString::new(name) {
            Ok(e) => e,
            //this error is temporary
            // TODO: change
            Err(_) => {
                return Err(VulkanError::OperationFailed(
                    "Could not turn application name to CStr",
                ));
            }
        };
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

            let exts = unsafe {
                match entry.enumerate_instance_extension_properties(None) {
                    Ok(l) => l,
                    Err(_) => {
                        return Err(VulkanError::OperationFailed(
                            "Could not enumerate extension properties",
                        ));
                    }
                }
            };

            for ex in exts {
                println!("{:?} ", ex.extension_name_as_c_str())
            }

            let layers = unsafe {
                match entry.enumerate_instance_layer_properties() {
                    Ok(l) => l,
                    Err(_) => {
                        return Err(VulkanError::OperationFailed(
                            "Could not enumerate layer properties",
                        ));
                    }
                }
            };

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
        let instance = unsafe {
            match entry.create_instance(&create_info, None) {
                Ok(instance) => instance,
                Err(e) => {
                    println!("{:?}", e);
                    return Err(VulkanError::OperationFailed(
                        "Could not create instance [fn] VulkanContext::initialize",
                    ));
                }
            }
        };

        // create xlib surface
        let xlib_surface_info = vk::XlibSurfaceCreateInfoKHR::default()
            .dpy(window.display.raw as *mut c_void)
            .window(window.window_id);
        let xlib_surface_loader = xlib_surface::Instance::new(&entry, &instance);
        let surface = unsafe {
            match xlib_surface_loader.create_xlib_surface(&xlib_surface_info, None) {
                Ok(res) => res,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "Could not create Xlib surface",
                    ));
                }
            }
        };
        let surface_loader = surface::Instance::new(&entry, &instance);

        //create device
        let mut dev = match VulkanDevice::new(&instance, &surface, &surface_loader) {
            Ok(dev) => dev,
            Err(e) => return Err(e),
        };

        // create swapchain
        let swap = match VulkanSwapchain::create(
            &instance,
            &mut dev,
            &surface,
            &surface_loader,
            window.width,
            window.height,
        ) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        // create renderpass
        let rend_pass = match VulkanRenderPass::create(
            0.0,
            0.0,
            window.width as f32,
            window.height as f32,
            0.0,
            0.0,
            0.2,
            1.0,
            1.0,
            0,
            &dev,
            swap.image_format.format,
            dev.depth_format,
        ) {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        let mut swap_framebuffers = Vec::new();
        for i in 0..swap.views.len() {
            let mut swap_attachments = Vec::new();
            swap_attachments.push(swap.views[i]);
            swap_attachments.push(swap.depth_attachment.view.unwrap());
            let buf = match VulkanFramebuffer::create(
                &dev,
                &rend_pass,
                window.height,
                window.width,
                &swap_attachments,
            ) {
                Ok(f) => f,
                Err(e) => return Err(e),
            };
            swap_framebuffers.push(buf);
        }
        // creating command buffer
        let graph_cmd_buf =
            match Self::create_command_buffer(&dev, swap.max_frames_in_flight as usize) {
                Ok(g) => g,
                Err(e) => return Err(e),
            };

        // creating sync objects
        let mut sync_objects = Vec::with_capacity(swap.max_frames_in_flight as usize);
        let mut images_in_flight = Vec::with_capacity(swap.max_frames_in_flight as usize);
        for _ in 0..swap.max_frames_in_flight {
            let obj = match SyncObjects::create(&dev, true) {
                Ok(o) => o,
                Err(e) => return Err(e),
            };
            sync_objects.push(obj);
        }

        for _ in 0..swap.image_count as usize {
            images_in_flight.push(None);
        }

        let in_flight_frames = InFlightFrames::new(sync_objects);

        let object_shader = match VulkanObjectShader::create(
            &instance,
            &dev,
            &rend_pass,
            window.width,
            window.height,
            swap.max_frames_in_flight as u32,
        ) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };

        let (vertex_buffer, index_buffer) = match Self::create_buffers(&instance, &dev) {
            Ok((v, i)) => (v, i),
            Err(e) => return Err(e),
        };
        const FACTOR: f32 = 10.0;
        const VERT_COUNT: usize = 4;
        let verts: [Vector3D; VERT_COUNT] = [
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, -0.5 * FACTOR, 0.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, 0.5 * FACTOR, 0.0),
            },
            Vector3D {
                position: Vec3::new(-0.5 * FACTOR, 0.5 * FACTOR, 0.0),
            },
            Vector3D {
                position: Vec3::new(0.5 * FACTOR, -0.5 * FACTOR, 0.0),
            },
        ];

        match Self::upload_data_range(
            &instance,
            &dev,
            dev.graphics_command_pool,
            Fence::null(),
            dev.graphics_queue,
            &vertex_buffer,
            0,
            &verts,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        };

        const INDEX_COUNT: usize = 6;
        let indices: [u32; INDEX_COUNT] = [0, 1, 2, 0, 3, 1];

        match Self::upload_data_range(
            &instance,
            &dev,
            dev.graphics_command_pool,
            Fence::null(),
            dev.graphics_queue,
            &index_buffer,
            0,
            &indices,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        };

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
            let debug_messenger = unsafe {
                debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap()
            };
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
                    geometry_vertex_offset: 0,
                    geometry_index_offset: 0,
                    object_index_buffer: index_buffer,
                    object_vertex_buffer: vertex_buffer,
                    object_shader: object_shader,
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
                    main_renderpass: rend_pass,
                    framebuffer_height: window.height,
                    framebuffer_width: window.width,
                    frame_buffer_size_generation: 0,
                    frame_buffer_last_generation: 0,
                    swapchain_framebuffers: swap_framebuffers,
                    dbg_messenger: debug_messenger,
                    dbg_util_loader: debug_utils_loader,
                })
            };
        }

        #[cfg(not(feature = "debug"))]
        {
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
                    geometry_vertex_offset: 0,
                    geometry_index_offset: 0,
                    object_index_buffer: index_buffer,
                    object_vertex_buffer: vertex_buffer,
                    object_shader: object_shader,
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
                    main_renderpass: rend_pass,
                    framebuffer_height: window.height,
                    framebuffer_width: window.width,
                    frame_buffer_size_generation: 0,
                    frame_buffer_last_generation: 0,
                    swapchain_framebuffers: swap_framebuffers,
                });
            }
        }
        println!("Vulkan Instance created");
        Ok(())
    }

    pub fn shutdown() -> Result<(), VulkanError> {
        unsafe {
            if let Some(ref _state) = VULKAN_STATE {
                VULKAN_STATE = None;
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context already destroyed",
                ));
            }
        }

        Ok(())
    }
    pub fn on_resize(width: i32, height: i32) -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context already destroyed",
                ));
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
    pub fn begin_frame(_delta: f32) -> Result<bool, VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context already destroyed",
                ));
            }
        };
        if state.recreating_swapchain {
            match unsafe { state.device.device.device_wait_idle() } {
                Ok(_) => return Ok(false),
                Err(_) => return Err(VulkanError::OperationFailed("could not wait on device")),
            }
        }

        if state.frame_buffer_last_generation != state.frame_buffer_size_generation {
            match unsafe { state.device.device.device_wait_idle() } {
                Ok(_) =>
                //{}
                {
                    match Self::recreate_swapchain() {
                        Ok(()) => return Ok(false),
                        Err(e) => return Err(e),
                    }
                }
                Err(_) => return Err(VulkanError::OperationFailed("could not wait on device")),
            }
        }
        let sync = &state.in_flight_frames.sync_objs;
        let current_frame = state.in_flight_frames.current_frame;

        match sync[current_frame].fence_wait(&state.device, u64::MAX) {
            Ok(b) => {
                if !b {
                    return Ok(false);
                }
            }
            Err(e) => return Err(e),
        }

        match sync[current_frame].reset_fence(&state.device) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        state.image_index = match state.swapchain.acquire_next_image_index(
            u64::MAX,
            sync[current_frame].image_avail_semaphore,
            sync[current_frame].fence,
        ) {
            Ok((suboptimal, index)) => {
                if suboptimal {
                    return Ok(false);
                } else {
                    index
                }
            }
            Err(e) => return Err(e),
        };

        if let Some(s) = state.images_in_flight[state.image_index as usize] {
            //if s.fence != Fence::null() {
            match s.fence_wait(&state.device, u64::MAX) {
                Ok(b) => {
                    if !b {
                        return Ok(false);
                    }
                }
                Err(e) => return Err(e),
            }
            //}
        }

        state.images_in_flight[state.image_index as usize] =
            Some(&state.in_flight_frames.sync_objs[current_frame as usize]);

        let command_buffer = &mut state.graphics_cmd_bufs;
        match command_buffer.begin(&state.device, false, false, false, current_frame as usize) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

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

        state.main_renderpass.w = state.framebuffer_width as f32;
        state.main_renderpass.h = state.framebuffer_height as f32;
        state.main_renderpass.begin(
            &state.device,
            command_buffer,
            state.in_flight_frames.current_frame as usize,
            state.swapchain_framebuffers[state.image_index as usize].framebuffer,
        );

        state.object_shader.use_shader(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        );

        Ok(true)
    }

    pub fn update_global_state(
        projection: Matrix4,
        view: Matrix4,
        _view_position: Vec3,
        _ambient_colour: Vec4,
        _mode: i32,
    ) -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context not initialized",
                ));
            }
        };
        state.object_shader.use_shader(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        );

        state.object_shader.global_ubo.projection = projection;
        state.object_shader.global_ubo.view = view;

        match state.object_shader.update_global_state(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }

    pub fn update_object(model: Matrix4) -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context not initialized",
                ));
            }
        };

        state.object_shader.update_object(
            &state.device,
            &state.graphics_cmd_bufs,
            state.in_flight_frames.current_frame as u32,
            model,
        );

        let offsets: [DeviceSize; 1] = [0];
        unsafe {
            state.device.device.cmd_bind_vertex_buffers(
                state.graphics_cmd_bufs.command_buffer
                    [state.in_flight_frames.current_frame as usize],
                0,
                &[state.object_vertex_buffer.buffer],
                &offsets,
            );

            state.device.device.cmd_bind_index_buffer(
                state.graphics_cmd_bufs.command_buffer
                    [state.in_flight_frames.current_frame as usize],
                state.object_index_buffer.buffer,
                0,
                IndexType::UINT32,
            );

            state.device.device.cmd_draw_indexed(
                state.graphics_cmd_bufs.command_buffer
                    [state.in_flight_frames.current_frame as usize],
                6,
                1,
                0,
                0,
                0,
            );
        }

        Ok(())
    }

    pub fn end_frame(_delta: f32) -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context not initialized",
                ));
            }
        };
        let image_index = state.image_index;
        let mut command_buff = &mut state.graphics_cmd_bufs;
        state.main_renderpass.end(
            &state.device,
            &mut command_buff,
            state.in_flight_frames.current_frame as usize,
        );

        match command_buff.end(&state.device, state.in_flight_frames.current_frame as usize) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if let Some(sync_obj) = state.images_in_flight[state.image_index as usize] {
            match sync_obj.fence_wait(&state.device, u64::MAX) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        state.images_in_flight[image_index as usize] =
            Some(&state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]);

        match state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
            .fence_wait(&state.device, u64::MAX)
        {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        match state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
            .reset_fence(&state.device)
        {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let submit_info = SubmitInfo::default()
            .command_buffers(std::slice::from_ref(
                &state.graphics_cmd_bufs.command_buffer[state.in_flight_frames.current_frame as usize],
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
            match state.device.device.queue_submit(
                state.device.graphics_queue,
                std::slice::from_ref(&submit_info),
                state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame as usize]
                    .fence,
            ) {
                Ok(_) => {}
                Err(_) => return Err(VulkanError::OperationFailed("could not submit to queue")),
            }
        }

        match state.swapchain.present(
            &state.device.graphics_queue,
            &state.device.present_queue,
            &state.in_flight_frames.sync_objs[state.in_flight_frames.current_frame]
                .render_finished_semaphore,
            state.image_index,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        state.in_flight_frames.current_frame = (state.in_flight_frames.current_frame + 1)
            % state.swapchain.max_frames_in_flight as usize;

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

    pub fn recreate_swapchain() -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context not initialized",
                ));
            }
        };
        println!("recreating swapchain");
        let _ = unsafe { state.device.device.device_wait_idle() };
        for i in 0..state.swapchain.image_count as usize {
            state.images_in_flight[i] = None;
        }
        state.recreating_swapchain = true;
        state.swapchain.destroy(&state.device);
        state.swapchain = match VulkanSwapchain::create(
            &state.instance,
            &mut state.device,
            &state.surface,
            &state.surface_loader,
            state.framebuffer_width,
            state.framebuffer_height,
        ) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        state.main_renderpass.w = state.framebuffer_width as f32;
        state.main_renderpass.h = state.framebuffer_height as f32;

        state.frame_buffer_last_generation = state.frame_buffer_size_generation;

        state
            .graphics_cmd_bufs
            .free(&state.device, state.device.graphics_command_pool);

        for i in 0..state.swapchain.image_count as usize {
            state.swapchain_framebuffers[i].destroy(&state.device);
        }

        state.swapchain_framebuffers = match Self::regenerate_framebuffers(
            &state.device,
            &state.main_renderpass,
            &state.swapchain,
            state.framebuffer_height,
            state.framebuffer_width,
        ) {
            Ok(f) => f,
            Err(e) => return Err(e),
        };

        state.graphics_cmd_bufs = match Self::create_command_buffer(
            &state.device,
            state.swapchain.max_frames_in_flight as usize,
        ) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        state.recreating_swapchain = false;

        Ok(())
    }

    fn create_command_buffer(
        device: &VulkanDevice,
        frames: usize,
    ) -> Result<VulkanCommandBuffer, VulkanError> {
        let cmd_buf = match VulkanCommandBuffer::allocate(
            device,
            true,
            device.graphics_command_pool,
            frames as u32,
        ) {
            Ok(c) => Ok(c),
            Err(e) => return Err(e),
        };
        cmd_buf
    }

    fn regenerate_framebuffers(
        dev: &VulkanDevice,
        rend_pass: &VulkanRenderPass,
        swap: &VulkanSwapchain,
        height: u32,
        width: u32,
    ) -> Result<Vec<VulkanFramebuffer>, VulkanError> {
        let mut swap_framebuffers = Vec::new();
        for i in 0..swap.views.len() {
            let mut swap_attachments = Vec::new();
            swap_attachments.push(swap.views[i]);
            swap_attachments.push(swap.depth_attachment.view.unwrap());
            let buf =
                match VulkanFramebuffer::create(&dev, &rend_pass, height, width, &swap_attachments)
                {
                    Ok(f) => f,
                    Err(e) => return Err(e),
                };
            swap_framebuffers.push(buf);
        }
        Ok(swap_framebuffers)
    }

    fn create_buffers(
        instance: &Instance,
        device: &VulkanDevice,
    ) -> Result<(VulkanBuffer, VulkanBuffer), VulkanError> {
        let memory_property_flag = MemoryPropertyFlags::DEVICE_LOCAL;

        const VERTEX_BUFFER_SIZE: usize = size_of::<Vector3D>() * 1024;

        let vertex_buffer = match VulkanBuffer::create(
            instance,
            device,
            VERTEX_BUFFER_SIZE as u64,
            BufferUsageFlags::VERTEX_BUFFER
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
            memory_property_flag,
            true,
        ) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        const INDEX_BUFFER_SIZE: usize = size_of::<u32>() * 1024;

        let index_buffer = match VulkanBuffer::create(
            instance,
            device,
            INDEX_BUFFER_SIZE as u64,
            BufferUsageFlags::INDEX_BUFFER
                | BufferUsageFlags::TRANSFER_DST
                | BufferUsageFlags::TRANSFER_SRC,
            memory_property_flag,
            true,
        ) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        Ok((vertex_buffer, index_buffer))
    }

    fn upload_data_range<T: Copy>(
        instance: &Instance,
        device: &VulkanDevice,
        pool: CommandPool,
        fence: Fence,
        queue: Queue,
        buffer: &VulkanBuffer,
        offset: u64,
        data: &[T],
    ) -> Result<(), VulkanError> {
        let memory_flags = MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT;
        let staging_buffer = match VulkanBuffer::create(
            instance,
            device,
            size_of_val(data) as u64,
            BufferUsageFlags::TRANSFER_SRC,
            memory_flags,
            true,
        ) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        match staging_buffer.load_data(
            device,
            offset,
            size_of_val(data) as u64,
            MemoryMapFlags::empty(),
            data,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        match VulkanBuffer::copy_to(
            device,
            pool,
            fence,
            queue,
            staging_buffer.buffer,
            0,
            buffer.buffer,
            offset,
            size_of_val(data) as u64,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

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
                    frame.destroy(&state.device);
                }
                state.object_shader.destroy(&state.device);
                state.in_flight_frames.destroy(&state.device);
                state
                    .device
                    .device
                    .destroy_command_pool(state.device.graphics_command_pool, None);
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
                    frame.destroy(&state.device);
                }
                state.object_shader.destroy(&state.device);
                state.in_flight_frames.destroy(&state.device);
                state
                    .device
                    .device
                    .destroy_command_pool(state.device.graphics_command_pool, None);
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
