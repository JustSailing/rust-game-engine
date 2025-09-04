use ash::{
    Device, Entry, Instance,
    ext::debug_utils,
    khr::{surface, swapchain, xlib_surface},
    vk,
    vk::DebugUtilsMessengerEXT,
};

use std::{
    borrow::Cow,
    ffi::{self, CString},
};

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext {
    instance: Instance,
    #[cfg(feature = "debug")]
    dbg_messenger: DebugUtilsMessengerEXT,
    #[cfg(feature = "debug")]
    dbg_util_loader: debug_utils::Instance,
}

impl VulkanContext {
    pub fn initialize(name: &str) -> Result<(), VulkanError> {
        unsafe {
            if let Some(ref _state) = VULKAN_STATE {
                return Err(VulkanError::OperationFailed(
                    "Vulkan Context was already initialized",
                ));
            }
        }

        let entry = Entry::linked();
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

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers_names_raw);
        let instance = unsafe {
            match entry.create_instance(&create_info, None) {
                Ok(instance) => instance,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "Could not create instance [fn] VulkanContext::initialize",
                    ));
                }
            }
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
                    instance: instance,
                    //dbg_messenger: None,
                });
            }
        }
        println!("Vulkan Instance created");
        Ok(())
    }

    pub fn shutdown() -> Result<(), VulkanError> {
        #[cfg(feature = "debug")]
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state
                    .dbg_util_loader
                    .destroy_debug_utils_messenger(state.dbg_messenger, None);
                state.instance.destroy_instance(None);
                VULKAN_STATE = None;
            }
        }
        unsafe {
            if let Some(ref mut state) = VULKAN_STATE {
                state.instance.destroy_instance(None);
                VULKAN_STATE = None;
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
