use crate::renderer::vulkan::{
    vulkan_backend::{VulkanBackendError, VulkanContext},
    vulkan_buffer::VulkanBuffer,
    vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice,
};
use ash::{
    Instance,
    vk::{
        AccessFlags, BufferImageCopy, DependencyFlags, DeviceMemory, Extent3D, Format, Image,
        ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers,
        ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, MemoryAllocateInfo, MemoryPropertyFlags,
        PipelineStageFlags, SampleCountFlags, SharingMode,
    },
};

type Result<T> = std::result::Result<T, VulkanBackendError>;

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct VulkanImage {
    pub image: Image,
    pub memory: DeviceMemory,
    pub view: Option<ImageView>,
    pub width: u32,
    pub height: u32,
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create image",
                        file: file!(),
                        line: line!(),
                    });
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
            return Err(VulkanBackendError::OperationFailed {
                issue: "required memory type not found. Image not valid.",
                file: file!(),
                line: line!(),
            });
        }
        let memory_allocate_info = MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type as u32);

        let device_memory = unsafe {
            match device.device.allocate_memory(&memory_allocate_info, None) {
                Ok(dm) => dm,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not allocate memory for image",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let _ = unsafe {
            match device.device.bind_image_memory(image, device_memory, 0) {
                Ok(_) => (),
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not bind memory for image",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let mut image_view: Option<ImageView> = None;
        if create_view {
            image_view = Some(Self::create_view(device, image, format, view_aspect_flags)?);
            return Ok(VulkanImage {
                image,
                memory: device_memory,
                view: image_view,
                width,
                height,
            });
        }

        Ok(VulkanImage {
            image,
            memory: device_memory,
            view: image_view,
            width,
            height,
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
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue: "could not create image view",
                    file: file!(),
                    line: line!(),
                }),
            }
        }
    }

    pub fn transition_layout(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        _format: Format,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        index: usize,
    ) -> Result<()> {
        let mut barrier = ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(device.graphics_queue_index as u32)
            .dst_queue_family_index(device.graphics_queue_index as u32)
            .image(self.image)
            .subresource_range(
                ImageSubresourceRange::default()
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        let mut source_stage = PipelineStageFlags::empty();
        let mut dest_stage = PipelineStageFlags::empty();

        if old_layout == ImageLayout::UNDEFINED && new_layout == ImageLayout::TRANSFER_DST_OPTIMAL {
            barrier.src_access_mask = AccessFlags::empty();
            barrier.dst_access_mask = AccessFlags::TRANSFER_WRITE;

            source_stage = PipelineStageFlags::TOP_OF_PIPE;

            dest_stage = PipelineStageFlags::TRANSFER;
        } else if old_layout == ImageLayout::TRANSFER_DST_OPTIMAL
            && new_layout == ImageLayout::SHADER_READ_ONLY_OPTIMAL
        {
            barrier.src_access_mask = AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = AccessFlags::SHADER_READ;

            source_stage = PipelineStageFlags::TRANSFER;
            dest_stage = PipelineStageFlags::FRAGMENT_SHADER;
        } else {
            return Err(VulkanBackendError::OperationFailed {
                issue: "unsupported transition",
                file: file!(),
                line: line!(),
            });
        }

        unsafe {
            device.device.cmd_pipeline_barrier(
                command_buffer.command_buffer[index],
                source_stage,
                dest_stage,
                DependencyFlags::empty(),
                &[],
                &[],
                std::slice::from_ref(&barrier),
            )
        };

        Ok(())
    }

    pub fn copy_from_buffer(
        &self,
        device: &VulkanDevice,
        buffer: &VulkanBuffer,
        command_buffer: &VulkanCommandBuffer,
        index: usize,
    ) {
        let region = BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(
                ImageSubresourceLayers::default()
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image_extent(
                Extent3D::default()
                    .width(self.width)
                    .height(self.height)
                    .depth(1),
            );

        unsafe {
            device.device.cmd_copy_buffer_to_image(
                command_buffer.command_buffer[index],
                buffer.buffer,
                self.image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&region),
            )
        };
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        if let Some(iv) = self.view {
            unsafe { device.device.destroy_image_view(iv, None) };
        }
        unsafe { device.device.free_memory(self.memory, None) };
        unsafe { device.device.destroy_image(self.image, None) };
    }
}
