use ash::vk::{
    BufferUsageFlags, CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo,
    CommandBufferLevel, CommandBufferUsageFlags, CommandPool, Fence, Queue, SubmitInfo,
};

use super::{vulkan_backend::VulkanError, vulkan_device::VulkanDevice};

pub enum CommandBufferState {
    Ready,
    Recording,
    InRenderPass,
    RecordingEnded,
    Submitted,
    NotAllocated,
}

pub struct VulkanCommandBuffer {
    pub command_buffer: Vec<CommandBuffer>,
    pub state: CommandBufferState,
}

impl VulkanCommandBuffer {
    pub fn allocate(
        device: &VulkanDevice,
        is_primary: bool,
        pool: CommandPool,
        buffer_count: u32,
    ) -> Result<VulkanCommandBuffer, VulkanError> {
        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(if is_primary {
                CommandBufferLevel::PRIMARY
            } else {
                CommandBufferLevel::SECONDARY
            })
            .command_buffer_count(buffer_count);
        let command_buf = unsafe {
            match device.device.allocate_command_buffers(&allocate_info) {
                Ok(c) => c,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not allocate command buffer",
                    ));
                }
            }
        };
        Ok(VulkanCommandBuffer {
            command_buffer: command_buf,
            state: CommandBufferState::Ready,
        })
    }
    pub fn free(&mut self, device: &VulkanDevice, pool: CommandPool) {
        unsafe {
            device
                .device
                .free_command_buffers(pool, &self.command_buffer);
        }

        self.command_buffer.clear();
        self.state = CommandBufferState::NotAllocated;
    }

    pub fn begin(
        &mut self,
        device: &VulkanDevice,
        single_use: bool,
        renderpass_continue: bool,
        simultaneous_use: bool,
        buffer_index: usize,
    ) -> Result<(), VulkanError> {
        let mut begin_info = CommandBufferBeginInfo::default();
        if single_use {
            begin_info.flags |= CommandBufferUsageFlags::ONE_TIME_SUBMIT;
        }
        if renderpass_continue {
            begin_info.flags |= CommandBufferUsageFlags::RENDER_PASS_CONTINUE;
        }
        if simultaneous_use {
            begin_info.flags |= CommandBufferUsageFlags::SIMULTANEOUS_USE;
        }
        unsafe {
            match device
                .device
                .begin_command_buffer(self.command_buffer[buffer_index], &begin_info)
            {
                Ok(_) => self.state = CommandBufferState::Recording,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not begin command buffer",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn end(&mut self, device: &VulkanDevice, buffer_index: usize) -> Result<(), VulkanError> {
        unsafe {
            match device
                .device
                .end_command_buffer(self.command_buffer[buffer_index])
            {
                Ok(_) => {
                    self.state = CommandBufferState::RecordingEnded;
                }
                Err(_) => return Err(VulkanError::OperationFailed("ending command buffer failed")),
            }
        }

        Ok(())
    }

    pub fn allocate_and_begin_single_use(
        device: &VulkanDevice,
        pool: CommandPool,
    ) -> Result<VulkanCommandBuffer, VulkanError> {
        let mut cmd_buf = match Self::allocate(device, true, pool, 1) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        match cmd_buf.begin(device, true, false, false, 0) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(cmd_buf)
    }

    pub fn end_single_use(
        &mut self,
        device: &VulkanDevice,
        pool: CommandPool,
        queue: Queue,
        fence: Fence,
    ) -> Result<(), VulkanError> {
        match self.end(device, 0) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let submit_info = SubmitInfo::default().command_buffers(&self.command_buffer);
        let submit_infos = [submit_info];
        unsafe {
            match device.device.queue_submit(queue, &submit_infos, fence) {
                Ok(_) => {}
                Err(_) => return Err(VulkanError::OperationFailed("could not submit queue")),
            }

            match device.device.queue_wait_idle(queue) {
                Ok(_) => {}
                Err(_) => return Err(VulkanError::OperationFailed("could not wait for queue")),
            }
        }
        self.free(device, pool);
        Ok(())
    }
}
