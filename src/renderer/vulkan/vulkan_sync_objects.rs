use ash::vk::{self, Fence, FenceCreateFlags, FenceCreateInfo, Semaphore, SemaphoreCreateInfo};

use super::{vulkan_backend::VulkanBackendError, vulkan_device::VulkanDevice};

type Result<T> = std::result::Result<T, VulkanBackendError>;
#[derive(Clone, Copy)]
pub struct SyncObjects {
    pub image_avail_semaphore: Semaphore,
    pub render_finished_semaphore: Semaphore,
    pub fence: Fence,
}

impl SyncObjects {
    pub fn create(device: &VulkanDevice, signaled: bool) -> Result<Self> {
        let fence_create_info = FenceCreateInfo::default().flags(if signaled {
            FenceCreateFlags::SIGNALED
        } else {
            FenceCreateFlags::default()
        });
        let fence = unsafe {
            match device.device.create_fence(&fence_create_info, None) {
                Ok(f) => f,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create fence",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let sema1 = unsafe {
            let sema_info = SemaphoreCreateInfo::default();
            match device.device.create_semaphore(&sema_info, None) {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create semaphore",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        let sema2 = unsafe {
            let sema_info = SemaphoreCreateInfo::default();
            match device.device.create_semaphore(&sema_info, None) {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create semaphore",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        Ok(SyncObjects {
            image_avail_semaphore: sema1,
            render_finished_semaphore: sema2,
            fence: fence,
        })
    }

    pub fn fence_wait(&self, device: &VulkanDevice, time_out: u64) -> Result<bool> {
        unsafe {
            match device
                .device
                .wait_for_fences(std::slice::from_ref(&self.fence), true, time_out)
            {
                Ok(_) => Ok(true),
                Err(vk::Result::SUCCESS) => Ok(true),
                Err(vk::Result::TIMEOUT) => {
                    print!("fence timed out");
                    Ok(false)
                }
                Err(vk::Result::ERROR_DEVICE_LOST) => Err(VulkanBackendError::OperationFailed {
                    issue: "fence: device lost",
                    file: file!(),
                    line: line!(),
                }),
                Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => {
                    Err(VulkanBackendError::OperationFailed {
                        issue: "fence: out of host memory",
                        file: file!(),
                        line: line!(),
                    })
                }
                Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => {
                    Err(VulkanBackendError::OperationFailed {
                        issue: "fence: out of device memory",
                        file: file!(),
                        line: line!(),
                    })
                }
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue: "fence: unknown error occured",
                    file: file!(),
                    line: line!(),
                }),
            }
        }
    }

    pub fn reset_fence(&self, device: &VulkanDevice) -> Result<()> {
        unsafe {
            match device
                .device
                .reset_fences(std::slice::from_ref(&self.fence))
            {
                Ok(_) => Ok(()),
                Err(_) => Err(VulkanBackendError::OperationFailed {
                    issue: "could not reset fence",
                    file: file!(),
                    line: line!(),
                }),
            }
        }
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device
                .device
                .destroy_semaphore(self.image_avail_semaphore, None);
            device
                .device
                .destroy_semaphore(self.render_finished_semaphore, None);
            device.device.destroy_fence(self.fence, None);
        }
    }
}

pub struct InFlightFrames {
    pub sync_objs: Vec<SyncObjects>,
    pub current_frame: usize,
}

impl InFlightFrames {
    pub fn new(sync_objects: Vec<SyncObjects>) -> Self {
        Self {
            sync_objs: sync_objects,
            current_frame: 0,
        }
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        self.sync_objs.iter().for_each(|o| o.destroy(device));
    }
}

impl Iterator for InFlightFrames {
    type Item = SyncObjects;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.sync_objs[self.current_frame];

        self.current_frame = (self.current_frame + 1) % self.sync_objs.len();
        Some(next)
    }
}
