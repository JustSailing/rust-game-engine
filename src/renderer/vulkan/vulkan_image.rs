use super::{
    vulkan_backend::{Error as VulkanError, VulkanContext},
    vulkan_device::VulkanDevice,
};
use ash::{
    Instance,
    vk::{
        DeviceMemory, Extent3D, Format, Image, ImageAspectFlags, ImageCreateInfo, ImageLayout,
        ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, MemoryAllocateInfo, MemoryPropertyFlags,
        SampleCountFlags, SharingMode,
    },
};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

pub struct VulkanImage {
    image: Image,
    memory: DeviceMemory,
    pub view: Option<ImageView>,
    width: u32,
    height: u32,
}

impl VulkanImage {
    pub fn create(
        instance: &Instance,
        image_type: ImageType,
        width: u32,
        height: u32,
        format: Format,
        tiling: ImageTiling,
        usage: ImageUsageFlags,
        memory_flags: MemoryPropertyFlags,
        create_view: bool,
        view_aspect_flags: ImageAspectFlags,
        device: &VulkanDevice,
    ) -> Result<Self> {
        let image_create_info = ImageCreateInfo::default()
            .image_type(image_type)
            .extent(Extent3D::default().height(height).width(width).depth(1))
            .mip_levels(4)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(SampleCountFlags::TYPE_1)
            .sharing_mode(SharingMode::EXCLUSIVE);
        let image = unsafe {
            match device.device.create_image(&image_create_info, None) {
                Ok(i) => i,
                Err(_) => {
                    return Err(
                        VulkanError::OperationFailed("could not create image").into(),
                    );
                }
            }
        };

        let memory_requirements = unsafe { device.device.get_image_memory_requirements(image) };
        let memory_type = VulkanContext::find_memory_index(
            instance,
            &device,
            memory_requirements.memory_type_bits,
            memory_flags,
        );

        if memory_type == -1 {
            return Err(VulkanError::OperationFailed(
                "required memory type not found. Image not valid.",
            )
            .into());
        }
        let memory_allocate_info = MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type as u32);

        let device_memory = unsafe {
            match device.device.allocate_memory(&memory_allocate_info, None) {
                Ok(dm) => dm,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not allocate memory for image",
                    )
                    .into());
                }
            }
        };
        let _ = unsafe {
            match device.device.bind_image_memory(image, device_memory, 0) {
                Ok(_) => (),
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not bind memory for image",
                    )
                    .into());
                }
            }
        };
        let mut image_view: Option<ImageView> = None;
        if create_view {
            image_view = Some(Self::create_view(device, image, format, view_aspect_flags)?);
            return Ok(VulkanImage {
                image: image,
                memory: device_memory,
                view: image_view,
                width: width,
                height: height,
            });
        }

        Ok(VulkanImage {
            image: image,
            memory: device_memory,
            view: image_view,
            width: width,
            height: height,
        })
    }

    fn create_view(
        device: &VulkanDevice,
        image: Image,
        format: Format,
        aspect_flags: ImageAspectFlags,
    ) -> Result<ImageView> {
        let view_create_info = ImageViewCreateInfo::default()
            .image(image)
            .view_type(ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                ImageSubresourceRange::default()
                    .aspect_mask(aspect_flags)
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .layer_count(1)
                    .level_count(1),
            );
        unsafe {
            match device.device.create_image_view(&view_create_info, None) {
                Ok(v) => Ok(v),
                Err(_) => {
                    Err(VulkanError::OperationFailed("could not create image view").into())
                }
            }
        }
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        if let Some(iv) = self.view {
            unsafe { device.device.destroy_image_view(iv, None) };
        }
        unsafe { device.device.free_memory(self.memory, None) };
        unsafe { device.device.destroy_image(self.image, None) };
    }
}
