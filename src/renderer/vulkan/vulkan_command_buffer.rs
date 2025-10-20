use ash::vk::{
    CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel,
    CommandBufferResetFlags, CommandBufferUsageFlags, CommandPool, Fence, Queue, SubmitInfo,
};

use crate::application::renderer::vulkan::{vulkan_backend::VulkanBackendError, vulkan_device::VulkanDevice};

type Result<T> = std::result::Result<T, VulkanBackendError>;

#[repr(C)]
pub enum CommandBufferState {
    Ready,
    Reset,
    Recording,
    InRenderPass,
    RecordingEnded,
    Submitted,
    NotAllocated,
}

#[repr(C)]
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
    ) -> Result<VulkanCommandBuffer> {
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not allocate command buffer",
                        file: file!(),
                        line: line!(),
                    });
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
    ) -> Result<()> {
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not begin command buffer",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn reset(&mut self, device: &VulkanDevice, buffer_index: usize) -> Result<()> {
        unsafe {
            match device.device.reset_command_buffer(
                self.command_buffer[buffer_index],
                CommandBufferResetFlags::empty(),
            ) {
                Ok(_) => {
                    self.state = CommandBufferState::Reset;
                    Ok(())
                }
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not reset command buffer",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }
    }

    pub fn end(&mut self, device: &VulkanDevice, buffer_index: usize) -> Result<()> {
        unsafe {
            match device
                .device
                .end_command_buffer(self.command_buffer[buffer_index])
            {
                Ok(_) => {
                    self.state = CommandBufferState::RecordingEnded;
                }
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "ending command buffer failed",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn allocate_and_begin_single_use(
        device: &VulkanDevice,
        pool: CommandPool,
    ) -> Result<VulkanCommandBuffer> {
        let mut cmd_buf = Self::allocate(device, true, pool, 1)?;

        cmd_buf.begin(device, true, false, false, 0)?;
        Ok(cmd_buf)
    }

    pub fn end_single_use(
        &mut self,
        device: &VulkanDevice,
        pool: CommandPool,
        queue: Queue,
    ) -> Result<()> {
        self.end(device, 0)?;

        let submit_info = SubmitInfo::default().command_buffers(&self.command_buffer);
        let submit_infos = [submit_info];
        unsafe {
            match device
                .device
                .queue_submit(queue, &submit_infos, Fence::null())
            {
                Ok(_) => {}
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not submit queue",
                        file: file!(),
                        line: line!(),
                    });
                }
            }

            match device.device.queue_wait_idle(queue) {
                Ok(_) => {}
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not wait for queue",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        }
        self.free(device, pool);
        Ok(())
    }
}
