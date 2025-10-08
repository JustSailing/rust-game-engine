use std::ffi::c_void;
use std::ptr::copy_nonoverlapping as memcpy;

use super::{
    vulkan_backend::VulkanBackendError, vulkan_backend::VulkanContext,
    vulkan_command_buffer::VulkanCommandBuffer, vulkan_device::VulkanDevice,
};
use ash::Instance;
use ash::vk::{
    Buffer, BufferCopy, BufferCreateInfo, BufferUsageFlags, CommandPool, DeviceMemory, DeviceSize,
    Fence, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, Queue, SharingMode,
};

type Result<T> = std::result::Result<T, VulkanBackendError>;
pub struct VulkanBuffer {
    pub buffer: Buffer,
    size: u64,
    usage_flags: BufferUsageFlags,
    is_locked: bool,
    memory: DeviceMemory,
    memory_index: i32,
    memory_property_flags: MemoryPropertyFlags,
}

impl VulkanBuffer {
    pub fn create(
        instance: &Instance,
        device: &VulkanDevice,
        size: u64,
        usage: BufferUsageFlags,
        memory_property_flags: MemoryPropertyFlags,
        bind_on_create: bool,
    ) -> Result<VulkanBuffer> {
        let buffer_create_info = BufferCreateInfo::default()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage)
            .size(size as DeviceSize);

        let buffer = unsafe {
            match device.device.create_buffer(&buffer_create_info, None) {
                Ok(b) => b,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "could not create buffer",
                    });
                }
            }
        };

        let requirements = unsafe { device.device.get_buffer_memory_requirements(buffer) };

        let memory_index = VulkanContext::find_memory_index(
            instance,
            device,
            requirements.memory_type_bits,
            memory_property_flags,
        );

        if memory_index == -1 {
            return Err(VulkanBackendError::OperationFailed {
                issue : "could not find memory index",
            });
        }

        let allocate_info = MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_index as u32);

        let memory = unsafe {
            match device.device.allocate_memory(&allocate_info, None) {
                Ok(m) => m,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "could not allocate memory",
                    });
                }
            }
        };

        let vk_buffer = Self {
            buffer,
            size: size,
            usage_flags: usage,
            is_locked: false,
            memory: memory,
            memory_index,
            memory_property_flags: memory_property_flags,
        };
        if bind_on_create {
            match vk_buffer.bind(device, 0) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(vk_buffer)
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device.device.free_memory(self.memory, None);
            device.device.destroy_buffer(self.buffer, None);
        }
    }

    pub fn resize(
        &mut self,
        device: &VulkanDevice,
        new_size: u64,
        queue: Queue,
        pool: CommandPool,
    ) -> Result<()> {
        let buffer_create_info = BufferCreateInfo::default()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .size(new_size)
            .usage(self.usage_flags);

        let new_buffer = unsafe {
            match device.device.create_buffer(&buffer_create_info, None) {
                Ok(b) => b,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "failed to create buffer",
                    });
                }
            }
        };

        let requirements = unsafe { device.device.get_buffer_memory_requirements(new_buffer) };

        let memory_info = MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(self.memory_index as u32);

        let new_memory = unsafe {
            match device.device.allocate_memory(&memory_info, None) {
                Ok(m) => m,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "could not allocate memory",
                    });
                }
            }
        };

        unsafe {
            match device.device.bind_buffer_memory(new_buffer, new_memory, 0) {
                Ok(_) => {}
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "could not bind new memory",
                    });
                }
            }
        }

        Self::copy_to(
            device,
            pool,
            Fence::null(),
            queue,
            self.buffer,
            0,
            new_buffer,
            0,
            self.size,
        )?;
        let _ = unsafe { device.device.device_wait_idle() };

        unsafe {
            device.device.free_memory(self.memory, None);
            device.device.destroy_buffer(self.buffer, None);
        };

        self.buffer = new_buffer;
        self.memory = new_memory;
        self.size = new_size;

        Ok(())
    }

    fn bind(&self, device: &VulkanDevice, offset: u64) -> Result<()> {
        match unsafe {
            device
                .device
                .bind_buffer_memory(self.buffer, self.memory, offset)
        } {
            Ok(_) => Ok(()),
            Err(_) => Err(VulkanBackendError::OperationFailed {
                issue : "could not bind buffer memory",
            }),
        }
    }

    pub fn lock_memory(
        &self,
        device: &VulkanDevice,
        offset: u64,
        size: u64,
        flags: MemoryMapFlags,
    ) -> Result<*mut c_void> {
        unsafe {
            match device.device.map_memory(self.memory, offset, size, flags) {
                Ok(d) => Ok(d),
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue : "could not map memory",
                }),
            }
        }
    }

    pub fn unlock_memory(&self, device: &VulkanDevice) {
        unsafe {
            device.device.unmap_memory(self.memory);
        }
    }

    pub fn load_data<T: Copy>(
        &self,
        device: &VulkanDevice,
        offset: u64,
        size: u64,
        flags: MemoryMapFlags,
        data: &[T],
    ) -> Result<()> {
        let data_ptr = unsafe {
            match device.device.map_memory(self.memory, offset, size, flags) {
                Ok(d) => d,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue : "could not map memory",
                    });
                }
            }
        };

        unsafe {
            memcpy(data.as_ptr(), data_ptr.cast(), data.len());
            device.device.unmap_memory(self.memory);
        }

        Ok(())
    }

    pub fn copy_to(
        device: &VulkanDevice,
        pool: CommandPool,
        _fence: Fence,
        queue: Queue,
        source: Buffer,
        src_offset: u64,
        dst: Buffer,
        dst_offset: u64,
        size: u64,
    ) -> Result<()> {
        let _ = unsafe { device.device.queue_wait_idle(queue) };
        let mut command_buffer = VulkanCommandBuffer::allocate_and_begin_single_use(device, pool)?;

        let copy_region = BufferCopy::default()
            .src_offset(src_offset)
            .dst_offset(dst_offset)
            .size(size);

        unsafe {
            device.device.cmd_copy_buffer(
                command_buffer.command_buffer[0],
                source,
                dst,
                &[copy_region],
            );
        }

        command_buffer.end_single_use(device, pool, queue)
    }
}
