use crate::application::renderer::vulkan::vulkan_backend::VulkanBackendError;
use ash::{
    Device, Instance,
    khr::surface,
    vk::{
        CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo, DeviceCreateInfo,
        DeviceQueueCreateInfo, Format, FormatFeatureFlags, KHR_SWAPCHAIN_NAME, PhysicalDevice,
        PhysicalDeviceFeatures, PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
        PhysicalDeviceType, PresentModeKHR, Queue, QueueFlags, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR, SurfaceKHR,
    },
};
use std::ffi::CStr;

type Result<T> = std::result::Result<T, VulkanBackendError>;

#[repr(C)]
struct PhysicalDeviceRequirements {
    graphics: bool,
    present: bool,
    compute: bool,
    transfer: bool,
    device_extension_names: Vec<&'static CStr>,
    sampler_anisotropy: bool,
    discrete_gpu: bool,
}

#[repr(C)]
pub struct SwapchainSupportInfo {
    pub capabilities: Option<SurfaceCapabilitiesKHR>,
    pub formats: Vec<SurfaceFormatKHR>,
    pub present_modes: Vec<PresentModeKHR>,
}

#[repr(C)]
struct PhysicalDeviceQueueFamilyInfo {
    graphics_family_index: i32,
    present_family_index: i32,
    compute_family_index: i32,
    transfer_family_index: i32,
}

#[repr(C)]
pub struct VulkanDevice {
    pub device: Device,
    pub physical_device: PhysicalDevice,

    pub swapchain_support: SwapchainSupportInfo,
    pub depth_format: Format,
    pub graphics_command_pool: CommandPool,
    pub graphics_queue_index: i32,
    pub present_queue_index: i32,
    transfer_queue_index: i32,
    pub graphics_queue: Queue,
    transfer_queue: Queue,
    pub present_queue: Queue,
    properties: PhysicalDeviceProperties,
    features: PhysicalDeviceFeatures,
    memory: PhysicalDeviceMemoryProperties,
}

impl VulkanDevice {
    pub fn new(
        instance: &Instance,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
    ) -> Result<Self> {
        let physical_devices = unsafe {
            match instance.enumerate_physical_devices() {
                Ok(devs) => devs,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "Could not enumerate physical devices",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        for phys_dev in physical_devices {
            let properties = unsafe { instance.get_physical_device_properties(phys_dev) };
            let features = unsafe { instance.get_physical_device_features(phys_dev) };
            let memory = unsafe { instance.get_physical_device_memory_properties(phys_dev) };
            let mut requirements = PhysicalDeviceRequirements {
                graphics: true,
                present: true,
                compute: true,
                transfer: true,
                device_extension_names: Vec::new(),
                sampler_anisotropy: true,
                discrete_gpu: false,
            };
            requirements
                .device_extension_names
                .push(ash::vk::KHR_SWAPCHAIN_NAME);
            let mut queue_info = PhysicalDeviceQueueFamilyInfo {
                graphics_family_index: -1,
                present_family_index: -1,
                compute_family_index: -1,
                transfer_family_index: -1,
            };
            let mut swap_info = SwapchainSupportInfo {
                capabilities: None,
                formats: Vec::new(),
                present_modes: Vec::new(),
            };
            match VulkanDevice::physical_device_meets_requirements(
                instance,
                &phys_dev,
                surface,
                surface_loader,
                &properties,
                &features,
                &requirements,
                &mut queue_info,
                &mut swap_info,
            ) {
                Ok(e) => {
                    if !e {
                        continue;
                    }
                }
                Err(e) => return Err(e),
            }

            let present_shares_graphics_queue =
                queue_info.graphics_family_index == queue_info.present_family_index;
            let transfer_shares_graphics_queue =
                queue_info.graphics_family_index == queue_info.transfer_family_index;
            let mut index_count = 1;
            if !present_shares_graphics_queue {
                index_count += 1;
            }
            if !transfer_shares_graphics_queue {
                index_count += 1;
            }
            let mut indices: Vec<i32> = Vec::new();
            indices.reserve(index_count as usize);

            indices.push(queue_info.graphics_family_index);
            if !present_shares_graphics_queue {
                indices.push(queue_info.present_family_index);
            }
            if !transfer_shares_graphics_queue {
                indices.push(queue_info.transfer_family_index);
            }

            let queue_priority = [1.0f32];

            let queue_create_info = indices
                .iter()
                .map(|index| {
                    DeviceQueueCreateInfo::default()
                        .queue_family_index(*index as u32)
                        .queue_priorities(&queue_priority)
                })
                .collect::<Vec<_>>();
            let device_features = PhysicalDeviceFeatures::default().sampler_anisotropy(true);
            let extension_names = [KHR_SWAPCHAIN_NAME.as_ptr()];
            let device_create_info = DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_info)
                .enabled_features(&device_features)
                .enabled_extension_names(&extension_names);
            let dev = unsafe {
                match instance.create_device(phys_dev, &device_create_info, None) {
                    Ok(dev) => dev,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create logical device",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            let graphics_queue =
                unsafe { dev.get_device_queue(queue_info.graphics_family_index as u32, 0) };
            let transfer_queue =
                unsafe { dev.get_device_queue(queue_info.transfer_family_index as u32, 0) };
            let present_queue =
                unsafe { dev.get_device_queue(queue_info.present_family_index as u32, 0) };
            let pool_create_info = CommandPoolCreateInfo::default()
                .queue_family_index(queue_info.graphics_family_index as u32)
                .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            let graph_pool = unsafe {
                match dev.create_command_pool(&pool_create_info, None) {
                    Ok(g) => g,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create graphics command pool",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            return Ok(VulkanDevice {
                device: dev,
                physical_device: phys_dev,
                properties: properties,
                features: features,
                memory: memory,
                swapchain_support: swap_info,
                graphics_command_pool: graph_pool,
                graphics_queue_index: queue_info.graphics_family_index,
                present_queue_index: queue_info.present_family_index,
                transfer_queue_index: queue_info.transfer_family_index,
                graphics_queue,
                transfer_queue,
                present_queue,
                depth_format: Format::default(),
            });
        }
        return Err(VulkanBackendError::OperationFailed {
            issue: "Could not find suitable device",
            file: file!(),
            line: line!(),
        });
    }

    pub fn query_swapchain_support(
        phys_dev: &PhysicalDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        swapchain_support_info: &mut SwapchainSupportInfo,
    ) -> Result<()> {
        let capabilities = unsafe {
            match surface_loader.get_physical_device_surface_capabilities(*phys_dev, *surface) {
                Ok(c) => c,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get surface capabilities",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let formats = unsafe {
            match surface_loader.get_physical_device_surface_formats(*phys_dev, *surface) {
                Ok(f) => f,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get surface formats",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let present_modes = unsafe {
            match surface_loader.get_physical_device_surface_present_modes(*phys_dev, *surface) {
                Ok(p) => p,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get present modes",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        swapchain_support_info.capabilities = Some(capabilities);
        swapchain_support_info.formats = formats;
        swapchain_support_info.present_modes = present_modes;
        Ok(())
    }

    fn physical_device_meets_requirements(
        instance: &Instance,
        phys_dev: &PhysicalDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        dev_properties: &PhysicalDeviceProperties,
        features: &PhysicalDeviceFeatures,
        requirements: &PhysicalDeviceRequirements,
        queue_family_info: &mut PhysicalDeviceQueueFamilyInfo,
        swapchain_support_info: &mut SwapchainSupportInfo,
    ) -> Result<bool> {
        queue_family_info.compute_family_index = -1;
        queue_family_info.graphics_family_index = -1;
        queue_family_info.present_family_index = -1;
        queue_family_info.transfer_family_index = -1;

        if requirements.discrete_gpu {
            if dev_properties.device_type != PhysicalDeviceType::DISCRETE_GPU {
                return Ok(false);
            }
        }

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(*phys_dev) };
        println!("Graphics | Present  | Compute  | Name");
        let mut min_transfer_score: u8 = 255;
        for (index, queue_family) in queue_families.iter().enumerate() {
            let mut current_transfer_score: u8 = 0;
            if queue_family.queue_flags & QueueFlags::GRAPHICS == QueueFlags::GRAPHICS {
                queue_family_info.graphics_family_index = index as i32;
                current_transfer_score += 1;
            }
            if queue_family.queue_flags & QueueFlags::COMPUTE == QueueFlags::COMPUTE {
                queue_family_info.compute_family_index = index as i32;
                current_transfer_score += 1;
            }
            if queue_family.queue_flags & QueueFlags::TRANSFER == QueueFlags::TRANSFER {
                if current_transfer_score <= min_transfer_score {
                    min_transfer_score = current_transfer_score;
                    queue_family_info.transfer_family_index = index as i32;
                }
            }

            let res = unsafe {
                match surface_loader.get_physical_device_surface_support(
                    *phys_dev,
                    index as u32,
                    *surface,
                ) {
                    Ok(b) => b,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "Failed to get physical device surface support",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            };
            if res {
                queue_family_info.present_family_index = index as i32;
            }
        }
        let name = match dev_properties.device_name_as_c_str() {
            Ok(s) => s.to_str().unwrap_or("could not convert cstr to str"),
            Err(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "Could not get device name",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        println!(
            "   {}     |    {}     |    {}     | {}",
            queue_family_info.graphics_family_index,
            queue_family_info.present_family_index,
            queue_family_info.compute_family_index,
            name
        );
        if (!requirements.graphics
            || (requirements.graphics && queue_family_info.graphics_family_index != -1))
            && (!requirements.compute
                || (requirements.compute && queue_family_info.compute_family_index != -1))
            && (!requirements.present
                || (requirements.present && queue_family_info.present_family_index != -1))
            && (!requirements.transfer
                || (requirements.transfer && queue_family_info.transfer_family_index != -1))
        {
            println!("Device meets queue requirements")
        }
        match VulkanDevice::query_swapchain_support(
            phys_dev,
            surface,
            surface_loader,
            swapchain_support_info,
        ) {
            Ok(_) => (),
            Err(err) => return Err(err),
        }
        if swapchain_support_info.formats.len() < 1
            || swapchain_support_info.present_modes.len() < 1
        {
            return Ok(false);
        }
        let extensions = unsafe {
            match instance.enumerate_device_extension_properties(*phys_dev) {
                Ok(ext) => ext,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get extension properties",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        for req in &requirements.device_extension_names {
            let mut found = false;
            for ext in &extensions {
                let name = match ext.extension_name_as_c_str() {
                    Ok(e) => e,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not get extension name",
                            file: file!(),
                            line: line!(),
                        });
                    }
                };
                if *req == name {
                    found = true;
                }
            }
            if !found {
                println!("device not suitable...skipping device");
                return Ok(false);
            }
        }
        if requirements.sampler_anisotropy && features.sampler_anisotropy < 1 {
            println!("device not does not have sampler anisotropy...skipping device");
            return Ok(false);
        }
        Ok(true)
    }

    pub fn detect_depth_format(&mut self, instance: &Instance) -> bool {
        let candidates = [
            Format::D32_SFLOAT,
            Format::D32_SFLOAT_S8_UINT,
            Format::D24_UNORM_S8_UINT,
        ];
        let flags = FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT;
        for f in &candidates {
            let properties =
                unsafe { instance.get_physical_device_format_properties(self.physical_device, *f) };
            if (properties.linear_tiling_features & flags) == flags {
                self.depth_format = *f;
                return true;
            } else if (properties.optimal_tiling_features & flags) == flags {
                self.depth_format = *f;
                return true;
            }
        }
        false
    }
}
