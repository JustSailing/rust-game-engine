use ash::{
    Entry, Instance,
    khr::{
        surface::{self, Instance as SurfaceInstance},
        xlib_surface,
    },
    vk::{self, MemoryPropertyFlags, SurfaceKHR},
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};

use std::{ffi::CString, os::raw::c_void};

use super::{
    vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice,
    vulkan_renderpass::VulkanRenderPass,
    vulkan_swapchain::VulkanSwapchain,
    vulkan_sync_objects::{InFlightFrames, SyncObjects},
};
use crate::application::basic::window::Window;

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
    in_fligh_frames: InFlightFrames,
    graphics_cmd_bufs: VulkanCommandBuffer,
    main_renderpass: VulkanRenderPass,
    swapchain: VulkanSwapchain,
    framebuffer_width: u32,
    framebuffer_height: u32,
    device: VulkanDevice,
    surface_loader: SurfaceInstance,
    surface: SurfaceKHR,
    instance: Instance,
}

impl VulkanContext {
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
        let mut dev = match VulkanDevice::new(&instance, &surface, &surface_loader) {
            Ok(dev) => dev,
            Err(e) => return Err(e),
        };

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

        let graph_cmd_buf = match Self::create_command_buffer(&dev, swap.image_count as usize) {
            Ok(g) => g,
            Err(e) => return Err(e),
        };

        let mut sync_objects = Vec::new();
        for i in 0..swap.max_frames_in_flight {
            let obj = match SyncObjects::create(&dev, true) {
                Ok(o) => o,
                Err(e) => return Err(e),
            };
            sync_objects.push(obj);
        }

        let in_flight_frames = InFlightFrames::new(sync_objects);

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
                    dbg_messenger: debug_messenger,
                    dbg_util_loader: debug_utils_loader,
                })
            };
        }

        #[cfg(not(feature = "debug"))]
        {
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
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
        Ok(())
    }
    pub fn begin_frame(delta: f32) -> Result<(), VulkanError> {
        Ok(())
    }
    pub fn end_frame(delta: f32) -> Result<(), VulkanError> {
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
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                let _ = state.device.device.device_wait_idle();
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
