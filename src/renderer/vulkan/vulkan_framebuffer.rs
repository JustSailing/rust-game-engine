use ash::vk::{Framebuffer, FramebufferCreateInfo, ImageView};

use super::{
    vulkan_backend::VulkanError, vulkan_device::VulkanDevice, vulkan_renderpass::VulkanRenderPass,
};

pub struct VulkanFramebuffer {
    pub framebuffer: Framebuffer,
    attachments: Vec<ImageView>,
    //renderpass: &'a VulkanRenderPass,
}

impl VulkanFramebuffer {
    pub fn create(
        device: &VulkanDevice,
        renderpass: &VulkanRenderPass,
        height: u32,
        width: u32,
        attachments: &Vec<ImageView>,
    ) -> Result<Self, VulkanError> {
        let attachments = attachments.clone();
        let framebuffer_create_info = FramebufferCreateInfo::default()
            .render_pass(renderpass.renderpass)
            .attachments(&attachments)
            .height(height)
            .width(width)
            .layers(1);

        let framebuffer = unsafe {
            match device
                .device
                .create_framebuffer(&framebuffer_create_info, None)
            {
                Ok(f) => f,
                Err(_) => return Err(VulkanError::OperationFailed("could not create framebuffer")),
            }
        };
        Ok(VulkanFramebuffer { framebuffer: framebuffer, attachments: attachments})
    }

    pub fn destroy(&self, device: &VulkanDevice) {
      unsafe{
        device.device.destroy_framebuffer(self.framebuffer, None);
      }
    }
}
