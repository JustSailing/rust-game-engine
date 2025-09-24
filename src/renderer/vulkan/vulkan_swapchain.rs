use super::{
    vulkan_backend::{Error as VulkanError, VulkanContext},
    vulkan_device::VulkanDevice,
    vulkan_image::VulkanImage,
};
use ash::{
    Instance,
    khr::{surface, swapchain},
    vk::{
        self, CompositeAlphaFlagsKHR, Extent2D, Fence, Image, ImageAspectFlags,
        ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, MemoryPropertyFlags, PresentInfoKHR, PresentModeKHR,
        Queue, Semaphore, SharingMode, SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR,
        SwapchainKHR,
    },
};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

pub struct VulkanSwapchain {
    pub image_format: SurfaceFormatKHR,
    pub max_frames_in_flight: u8,
    swapchain: SwapchainKHR,
    swapchain_loader: swapchain::Device,
    pub image_count: u32,
    images: Vec<Image>,
    pub views: Vec<ImageView>,
    pub depth_attachment: VulkanImage,
}

impl VulkanSwapchain {
    pub fn create(
        instance: &Instance,
        device: &mut VulkanDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        VulkanDevice::query_swapchain_support(
            &device.physical_device,
            surface,
            surface_loader,
            &mut device.swapchain_support,
        )?;

        if !device.detect_depth_format(instance) {
            return Err(
                VulkanError::OperationFailed("could not detect depth format").into(),
            );
        }

        let mut swapchain_extent = Extent2D { width, height };
        let mut found = false;
        let mut format_: &SurfaceFormatKHR = &SurfaceFormatKHR::default();
        let max_frames_in_flight = 3;
        for format in &device.swapchain_support.formats {
            if format.format == vk::Format::B8G8R8A8_UNORM
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                format_ = format;
                found = true;
                break;
            }
        }
        if !found {
            for format in &device.swapchain_support.formats {
                format_ = format;
                break;
            }
        }
        let mut mode_: &PresentModeKHR = &PresentModeKHR::default();
        found = false;
        for mode in &device.swapchain_support.present_modes {
            if *mode == vk::PresentModeKHR::MAILBOX {
                mode_ = mode;
                found = true;
                break;
            }
        }
        if !found {
            for mode in &device.swapchain_support.present_modes {
                mode_ = mode;
                break;
            }
        }

        if device
            .swapchain_support
            .capabilities
            .unwrap()
            .current_extent
            .width
            != u32::MAX
        {
            swapchain_extent = device
                .swapchain_support
                .capabilities
                .unwrap()
                .current_extent;
        }

        let min = device
            .swapchain_support
            .capabilities
            .unwrap()
            .min_image_extent;
        let max = device
            .swapchain_support
            .capabilities
            .unwrap()
            .max_image_extent;
        swapchain_extent.width = swapchain_extent.width.min(max.width).max(min.width);
        swapchain_extent.height = swapchain_extent.height.min(max.height).max(min.height);
        let mut image_count = device
            .swapchain_support
            .capabilities
            .unwrap()
            .min_image_count;
        if device
            .swapchain_support
            .capabilities
            .unwrap()
            .max_image_count
            > 0
            && image_count
                > device
                    .swapchain_support
                    .capabilities
                    .unwrap()
                    .max_image_count
        {
            image_count = device
                .swapchain_support
                .capabilities
                .unwrap()
                .max_image_count;
        }

        let mut swapchain_create_info = SwapchainCreateInfoKHR::default()
            .min_image_count(image_count)
            .surface(*surface)
            .image_format(format_.format)
            .image_color_space(format_.color_space)
            .image_extent(swapchain_extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT);

        if device.graphics_queue_index != device.present_queue_index {
            let queue_family_indices = [
                device.graphics_queue_index as u32,
                device.present_queue_index as u32,
            ];
            swapchain_create_info.image_sharing_mode = SharingMode::CONCURRENT;
            swapchain_create_info.p_queue_family_indices = queue_family_indices.as_ptr();
            swapchain_create_info.queue_family_index_count = queue_family_indices.len() as u32;
        } else {
            swapchain_create_info.image_sharing_mode = SharingMode::EXCLUSIVE;
        }

        swapchain_create_info.pre_transform = device
            .swapchain_support
            .capabilities
            .unwrap()
            .current_transform;

        swapchain_create_info.composite_alpha = CompositeAlphaFlagsKHR::OPAQUE;
        swapchain_create_info.present_mode = *mode_;
        swapchain_create_info.clipped = true as u32;
        let swapchain_loader = swapchain::Device::new(instance, &device.device);
        let swap = unsafe {
            match swapchain_loader.create_swapchain(&swapchain_create_info, None) {
                Ok(s) => s,
                Err(_) => {
                    return Err(
                        VulkanError::OperationFailed("could not create swapchain").into(),
                    );
                }
            }
        };

        let images = unsafe {
            match swapchain_loader.get_swapchain_images(swap) {
                Ok(i) => i,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not get swapchain images",
                    )
                    .into());
                }
            }
        };
        let mut views: Vec<ImageView> = Vec::with_capacity(images.len());

        for i in 0..images.len() {
            let sub = ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let view_create_info = ImageViewCreateInfo::default()
                .image(images[i])
                .format(format_.format)
                .view_type(ImageViewType::TYPE_2D)
                .subresource_range(sub);
            views.push(unsafe {
                match device.device.create_image_view(&view_create_info, None) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(VulkanError::OperationFailed(
                            "could not create image views",
                        )
                        .into());
                    }
                }
            });
        }

        let depth_attachment = VulkanImage::create(
            instance,
            ImageType::TYPE_2D,
            swapchain_extent.width,
            swapchain_extent.height,
            device.depth_format,
            ImageTiling::OPTIMAL,
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            MemoryPropertyFlags::DEVICE_LOCAL,
            true,
            ImageAspectFlags::DEPTH,
            device,
        )?;

        Ok(VulkanSwapchain {
            image_format: *format_,
            max_frames_in_flight,
            swapchain: swap,
            swapchain_loader: swapchain_loader,
            image_count: images.len() as u32,
            images: images,
            views: views,
            depth_attachment: depth_attachment,
        })
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        self.depth_attachment.destroy(device);
        for v in &self.views {
            unsafe { device.device.destroy_image_view(*v, None) };
        }
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
    }
    pub fn present(
        &self,
        _graphics_queue: &Queue,
        present_queue: &Queue,
        render_complete_semaphore: &Semaphore,
        present_image_index: u32,
    ) -> Result<()> {
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(render_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&present_image_index));
        let res = unsafe {
            self.swapchain_loader
                .queue_present(*present_queue, &present_info)
        };
        match res {
            Ok(_) => return Ok(()),
            // recreate swapchain
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                VulkanContext::recreate_swapchain()?;
                return Ok(());
            }
            _ => {
                return Err(VulkanError::OperationFailed(
                    "present queue did not work properly",
                )
                .into());
            }
        }
    }
    pub fn acquire_next_image_index(
        &self,
        timeout: u64,
        semaphore: Semaphore,
        fence: Fence,
    ) -> Result<(bool, u32)> {
        let res = unsafe {
            self.swapchain_loader
                .acquire_next_image(self.swapchain, timeout, semaphore, fence)
        };
        match res {
            Ok((index, suboptimal)) => {
                return Ok((suboptimal, index))
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {return Ok((true, 0))},
            Err(_) => {
                return Err(
                    VulkanError::OperationFailed("failure to acqurie next image").into(),
                );
            }
        };
    }
}
