use ash::{
    Entry, Instance,
    khr::{
        surface::{self, Instance as SurfaceInstance},
        xlib_surface,
    },
    vk::{
        self, Extent2D, MemoryPropertyFlags, Offset2D, PipelineStageFlags, Rect2D, SubmitInfo,
        SurfaceKHR, Viewport,
    },
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};

use std::{ffi::CString, os::raw::c_void, u64};

use super::{
    super::vulkan::shaders::vulkan_object_shader::VulkanObjectShader,
    vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice,
    vulkan_framebuffer::VulkanFramebuffer,
    vulkan_renderpass::VulkanRenderPass,
    vulkan_swapchain::VulkanSwapchain,
    vulkan_sync_objects::{InFlightFrames, SyncObjects},
};
use crate::application::basic::window::Window;

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext<'a> {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
    object_shader: VulkanObjectShader<'a>,
    images_in_flight: Vec<Option<&'a SyncObjects>>,
    in_fligh_frames: InFlightFrames,
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
        let graph_cmd_buf = match Self::create_command_buffer(&dev, swap.image_count as usize) {
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

        let object_shader = match VulkanObjectShader::create(&dev, &rend_pass, window.width, window.height) {
            Ok(o) => o,
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
                    object_shader: object_shader,
                    images_in_flight: images_in_flight,
                    image_index: 0,
                    recreating_swapchain: false,
                    in_fligh_frames: in_flight_frames,
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
                    object_shader: object_shader,
                    images_in_flight: images_in_flight,
                    image_index: 0,
                    recreating_swapchain: false,
                    in_fligh_frames: in_flight_frames,
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
        let mut state = unsafe {
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
    pub fn begin_frame(delta: f32) -> Result<bool, VulkanError> {
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
        let sync = &state.in_fligh_frames.sync_objs;
        let current_frame = state.in_fligh_frames.current_frame;
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

        let command_buffer = &mut state.graphics_cmd_bufs;
        match command_buffer.begin(
            &state.device,
            false,
            false,
            false,
            state.image_index as usize,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let viewport = Viewport::default()
            .x(0.0)
            .y(state.framebuffer_height as f32)
            .height(state.framebuffer_height as f32)
            .width(state.framebuffer_width as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = Rect2D::default()
            .extent(
                Extent2D::default()
                    .height(state.frame_buffer_last_generation)
                    .width(state.framebuffer_width),
            )
            .offset(Offset2D::default());

        unsafe {
            state.device.device.cmd_set_viewport(
                command_buffer.command_buffer[state.image_index as usize],
                0,
                std::slice::from_ref(&viewport),
            );
        };
        unsafe {
            state.device.device.cmd_set_scissor(
                command_buffer.command_buffer[state.image_index as usize],
                0,
                std::slice::from_ref(&scissor),
            )
        };

        state.main_renderpass.w = state.framebuffer_width as f32;
        state.main_renderpass.h = state.framebuffer_height as f32;
        state.main_renderpass.begin(
            &state.device,
            command_buffer,
            state.image_index as usize,
            state.swapchain_framebuffers[state.image_index as usize].framebuffer,
        );

        Ok(true)
    }
    pub fn end_frame(delta: f32) -> Result<(), VulkanError> {
        let state = unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
            } else {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context already destroyed",
                ));
            }
        };
        let image_index = state.image_index;
        let mut command_buff = &mut state.graphics_cmd_bufs;
        state
            .main_renderpass
            .end(&state.device, &mut command_buff, image_index as usize);
        match command_buff.end(&state.device, image_index as usize) {
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
            Some(&state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame]);

        match state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame]
            .fence_wait(&state.device, u64::MAX)
        {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        // ensure the last frame is not acquiring image from swap
        // let last_current_frame = (state.in_fligh_frames.current_frame + 1)
        //     % state.swapchain.max_frames_in_flight as usize;
        // match state.in_fligh_frames.sync_objs[last_current_frame]
        //     .fence_wait(&state.device, u64::MAX)
        // {
        //     Ok(_) => {}
        //     Err(e) => return Err(e),
        // }
        match state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame]
            .reset_fence(&state.device)
        {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let submit_info = SubmitInfo::default()
            .command_buffers(std::slice::from_ref(
                &state.graphics_cmd_bufs.command_buffer[state.image_index as usize],
            ))
            .signal_semaphores(std::slice::from_ref(
                &state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame as usize]
                    .render_finished_semaphore,
            ))
            .wait_semaphores(std::slice::from_ref(
                &state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame as usize]
                    .image_avail_semaphore,
            ))
            .wait_dst_stage_mask(std::slice::from_ref(
                &PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ));

        unsafe {
            match state.device.device.queue_submit(
                state.device.graphics_queue,
                std::slice::from_ref(&submit_info),
                state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame as usize].fence,
            ) {
                Ok(_) => {}
                Err(_) => return Err(VulkanError::OperationFailed("could not submit to queue")),
            }
        }

        match state.swapchain.present(
            &state.device.graphics_queue,
            &state.device.present_queue,
            &state.in_fligh_frames.sync_objs[state.in_fligh_frames.current_frame]
                .render_finished_semaphore,
            state.image_index,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        state.in_fligh_frames.current_frame = (state.in_fligh_frames.current_frame + 1)
            % state.swapchain.max_frames_in_flight as usize;

        Ok(())
    }

    pub fn find_memory_index(
        instance: &Instance,
        dev: &VulkanDevice,
        type_filter: u32,
        property_flags: MemoryPropertyFlags,
    ) -> Result<i32, VulkanError> {
        let memory_props =
            unsafe { instance.get_physical_device_memory_properties(dev.physical_device) };
        for i in 0..memory_props.memory_type_count {
            let suitable = (type_filter & (1 << i)) != 0;
            let memory_type = memory_props.memory_types[i as usize];

            if suitable && memory_type.property_flags.contains(property_flags) {
                return Ok(i as i32);
            }
        }
        Ok(-1)
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
        for i in 0..state.swapchain.max_frames_in_flight as usize {
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
            state.swapchain.image_count as usize,
        ) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        state.recreating_swapchain = false;

        Ok(())
    }

    fn create_command_buffer(
        device: &VulkanDevice,
        image_count: usize,
    ) -> Result<VulkanCommandBuffer, VulkanError> {
        let cmd_buf = match VulkanCommandBuffer::allocate(
            device,
            true,
            device.graphics_command_pool,
            image_count as u32,
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
}

impl<'a> Drop for VulkanContext<'a> {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                let _ = state.device.device.device_wait_idle();
                for frame in &state.swapchain_framebuffers {
                    frame.destroy(&state.device);
                }
                state.object_shader.destroy(&state.device);
                state.in_fligh_frames.destroy(&state.device);
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
                for frame in &state.swapchain_framebuffers {
                    frame.destroy(&state.device);
                }
                state.object_shader.destroy(&state.device);
                state.in_fligh_frames.destroy(&state.device);
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
