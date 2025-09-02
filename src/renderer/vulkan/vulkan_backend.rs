use ash::{
    Device, Entry, Instance,
    ext::debug_utils,
    khr::{swapchain, xlib_surface},
    vk,
};

use std::ffi::CString;

pub enum VulkanError {
    OperationFailed(&'static str),
}

static mut VULKAN_STATE: Option<VulkanContext> = None;

pub struct VulkanContext {
    instance: vk::Instance,
}

impl VulkanContext {
    pub fn initialize(name: &str) -> Result<(), VulkanError> {
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

        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);
        let instance = unsafe {
            match entry.create_instance(&create_info, None) {
                Ok(instance) => instance,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "Could not create [fn] VulkanContext::initialize",
                    ));
                }
            }
        };

        println!("Vulkan Instance created");
        Ok(())
    }
    pub fn shutdown() -> Result<(), VulkanError> {
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
