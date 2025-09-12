use ash::{
    Entry, Instance,
    khr::surface::Instance as SurfaceInstance,
    khr::{surface, xlib_surface},
    vk::{self, SurfaceKHR},
};
#[cfg(feature = "debug")]
use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
#[cfg(feature = "debug")]
use std::{borrow::Cow, ffi};

use std::{ffi::CString, os::raw::c_void};

use crate::application::{
    basic::window::Window, renderer::renderer_types::vulkan::vulkan_device::VulkanDevice,
};

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext {
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
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
        let dev = match VulkanDevice::new(&instance, &surface, &surface_loader) {
            Ok(dev) => dev,
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
                    device: dev,
                    surface_loader: surface_loader,
                    surface: surface,
                    instance: instance,
                    dbg_messenger: debug_messenger,
                    dbg_util_loader: debug_utils_loader,
                })
            };
        }

        #[cfg(not(feature = "debug"))]
        {
            unsafe {
                VULKAN_STATE = Some(VulkanContext {
                    device: dev,
                    instance: instance,
                    surface: surface,
                    surface_loader: surface_loader,
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
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
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
