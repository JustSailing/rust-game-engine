use crate::application::renderer::vulkan::{
    vulkan_backend::VulkanBackendError, vulkan_device::VulkanDevice, vulkan_image::VulkanImage,
};

//use crate::application::renderer::renderer_types::RenderTarget;
use crate::application::resources::resource_types::{TextureData, TextureHandle};
use crate::application::systems::texture_system::TextureSystem;
use ash::{
    Instance,
    khr::{surface, swapchain},
    vk::{
        self, CompositeAlphaFlagsKHR, Extent2D, Fence, ImageAspectFlags, ImageSubresourceRange,
        ImageTiling, ImageType, ImageUsageFlags, ImageViewCreateInfo, ImageViewType,
        MemoryPropertyFlags, PresentInfoKHR, PresentModeKHR, Queue, Semaphore, SharingMode,
        SurfaceFormatKHR, SurfaceKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    },
};
use std::cell::RefCell;
use std::rc::Rc;

type Result<T> = std::result::Result<T, VulkanBackendError>;
#[repr(C)]
pub struct VulkanSwapchain {
    pub image_format: SurfaceFormatKHR,
    pub max_frames_in_flight: u8,
    pub swapchain: SwapchainKHR,
    swapchain_loader: swapchain::Device,
    pub image_count: u32,
    pub render_textures: Vec<TextureHandle>,
    pub depth_texture: TextureHandle,
    //pub render_targets: Vec<RenderTarget>,
}

impl VulkanSwapchain {
    pub fn create(
        instance: &Instance,
        device: &mut VulkanDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        width: u32,
        height: u32,
        texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<Self> {
        VulkanDevice::query_swapchain_support(
            &device.physical_device,
            surface,
            surface_loader,
            &mut device.swapchain_support,
        )?;

        if !device.detect_depth_format(instance) {
            return Err(VulkanBackendError::OperationFailed {
                issue: "could not detect depth format",
                file: file!(),
                line: line!(),
            });
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
            if *mode == PresentModeKHR::MAILBOX {
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create swapchain",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let mut render_textures = Vec::new();
        let images = unsafe {
            match swapchain_loader.get_swapchain_images(swap) {
                Ok(i) => i,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get swapchain images",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        for i in 0..images.len() {
            let tex_name = format!("__internal_vulkan_swapchain_image_{}__", i);
            let mut internal_data = TextureData::default();
            internal_data.image.image = images[i];
            internal_data.image.width = swapchain_extent.width;
            internal_data.image.height = swapchain_extent.height;
            let tex = texture_system
                .borrow_mut()
                .wrap_internal(
                    tex_name.as_str(),
                    swapchain_extent.width,
                    swapchain_extent.height,
                    4,
                    false,
                    true,
                    true,
                    internal_data,
                )
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not wrap internal texture",
                    file: file!(),
                    line: line!(),
                })?;
            render_textures.push(tex);
        }

        for i in 0..images.len() {
            let mut tex_sys = texture_system.borrow_mut();
            let texture = tex_sys.get_mut_texture(render_textures[i]).map_err(|_| {
                VulkanBackendError::OperationFailed {
                    issue: "could not get texture",
                    file: file!(),
                    line: line!(),
                }
            })?;
            let sub = ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let view_create_info = ImageViewCreateInfo::default()
                .image(texture.internal_data.image.image)
                .format(format_.format)
                .view_type(ImageViewType::TYPE_2D)
                .subresource_range(sub);

            texture.internal_data.image.view = Some(unsafe {
                match device.device.create_image_view(&view_create_info, None) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create image views",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            });
        }

        let depth_texture_data = TextureData {
            image: VulkanImage::create(
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
            )?,
        };

        let depth_texture = texture_system
            .borrow_mut()
            .wrap_internal(
                "__internal_default_depth_texture__",
                swapchain_extent.width,
                swapchain_extent.height,
                device.channel_count as u8,
                false,
                true,
                true,
                depth_texture_data,
            )
            .map_err(|_| VulkanBackendError::OperationFailed {
                issue: "could not wrap depth texture",
                file: file!(),
                line: line!(),
            })?;

        Ok(VulkanSwapchain {
            image_format: *format_,
            max_frames_in_flight,
            swapchain: swap,
            swapchain_loader,
            image_count: images.len() as u32,
            render_textures,
            depth_texture,
            //render_targets: Vec::new(),
        })
    }

    pub fn destroy(&self, device: &VulkanDevice, texture_system: &Rc<RefCell<TextureSystem>>) {
        match texture_system
            .borrow_mut()
            .get_mut_texture(self.depth_texture)
            .map_err(|_| VulkanBackendError::OperationFailed {
                issue: "could not get depth texture",
                file: file!(),
                line: line!(),
            }) {
            Ok(t) => t.internal_data.image.destroy(device),
            Err(_) => println!("could not destroy depth texture image"),
        }

        for i in 0..self.render_textures.len() {
            match texture_system
                .borrow_mut()
                .get_mut_texture(self.render_textures[i])
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not get depth texture",
                    file: file!(),
                    line: line!(),
                }) {
                Ok(t) => {
                    if let Some(view) = t.internal_data.image.view {
                        unsafe { device.device.destroy_image_view(view, None) };
                    }
                }
                Err(_) => println!("could not destroy depth texture image"),
            }
        }
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
    }

    pub fn recreate(
        &mut self,
        instance: &Instance,
        device: &mut VulkanDevice,
        surface: &SurfaceKHR,
        surface_loader: &surface::Instance,
        width: u32,
        height: u32,
        texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<()> {
        VulkanDevice::query_swapchain_support(
            &device.physical_device,
            surface,
            surface_loader,
            &mut device.swapchain_support,
        )?;

        if !device.detect_depth_format(instance) {
            return Err(VulkanBackendError::OperationFailed {
                issue: "could not detect depth format",
                file: file!(),
                line: line!(),
            });
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
            if *mode == PresentModeKHR::MAILBOX {
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create swapchain",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        for i in 0..self.render_textures.len() {
            texture_system
                .borrow_mut()
                .resize(
                    self.render_textures[i],
                    swapchain_extent.width,
                    swapchain_extent.height,
                    false,
                )
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not resize texture",
                    file: file!(),
                    line: line!(),
                })?;
        }

        self.destroy(&device, texture_system);

        // self.depth_texture
        //     .borrow_mut()
        //     .internal_data
        //     .image
        //     .destroy(device);
        // for i in 0..self.render_textures.len() {
        //     if let Some(view) = self.render_textures[i]
        //         .borrow_mut()
        //         .internal_data
        //         .image
        //         .view
        //     {
        //         unsafe { device.device.destroy_image_view(view, None) };
        //     }
        // }
        // unsafe {
        //     self.swapchain_loader
        //         .destroy_swapchain(self.swapchain, None)
        // };

        let images = unsafe {
            match swapchain_loader.get_swapchain_images(swap) {
                Ok(i) => i,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not get swapchain images",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        for i in 0..images.len() {
            texture_system
                .borrow_mut()
                .get_mut_texture(self.render_textures[i])
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not get mutable texture",
                    file: file!(),
                    line: line!(),
                })?
                .internal_data
                .image
                .image = images[i];
        }

        for i in 0..images.len() {
            let mut tex_sys = texture_system.borrow_mut();
            let texture = tex_sys
                .get_mut_texture(self.render_textures[i])
                .map_err(|_| VulkanBackendError::OperationFailed {
                    issue: "could not get mutable texture",
                    file: file!(),
                    line: line!(),
                })?;
            let sub = ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let view_create_info = ImageViewCreateInfo::default()
                .image(texture.internal_data.image.image)
                .format(format_.format)
                .view_type(ImageViewType::TYPE_2D)
                .subresource_range(sub);
            texture.internal_data.image.view = Some(unsafe {
                match device.device.create_image_view(&view_create_info, None) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(VulkanBackendError::OperationFailed {
                            issue: "could not create image views",
                            file: file!(),
                            line: line!(),
                        });
                    }
                }
            });
        }

        let depth_texture_data = TextureData {
            image: VulkanImage::create(
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
            )?,
        };

        let depth_texture = texture_system
            .borrow_mut()
            .wrap_internal(
                "__internal_default_depth_texture__",
                swapchain_extent.width,
                swapchain_extent.height,
                device.channel_count as u8,
                false,
                true,
                true,
                depth_texture_data,
            )
            .map_err(|_| VulkanBackendError::OperationFailed {
                issue: "could not wrap depth texture",
                file: file!(),
                line: line!(),
            })?;

        self.image_format = *format_;
        self.max_frames_in_flight = max_frames_in_flight;
        self.swapchain = swap;

        self.image_count = images.len() as u32;
        self.depth_texture = depth_texture;
        Ok(())
    }

    pub fn present(
        &self,
        _graphics_queue: &Queue,
        present_queue: &Queue,
        render_complete_semaphore: &Semaphore,
        present_image_index: u32,
    ) -> Result<bool> {
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(std::slice::from_ref(render_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&present_image_index));
        let res = unsafe {
            self.swapchain_loader
                .queue_present(*present_queue, &present_info)
        };
        match res {
            Ok(_) => Ok(true),
            // recreate swapchain
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                //TODO: recreate swapchain
                //VulkanContext::recreate_swapchain()?;
                Ok(false)
            }
            _ => Err(VulkanBackendError::OperationFailed {
                issue: "present queue did not work properly",
                file: file!(),
                line: line!(),
            }),
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
            Ok((index, _)) => Ok((false, index)),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                Ok((true, 0))
            }
            Err(_) => Err(VulkanBackendError::OperationFailed {
                issue: "failure to acquire next image",
                file: file!(),
                line: line!(),
            }),
        }
    }
}
