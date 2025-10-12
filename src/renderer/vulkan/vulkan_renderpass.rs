use ash::vk::{
    AccessFlags, AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, ClearColorValue, ClearDepthStencilValue, ClearValue,
    DependencyFlags, Extent2D, Format, Framebuffer, ImageLayout, Offset2D, PipelineBindPoint,
    PipelineStageFlags, Rect2D, RenderPass, RenderPassBeginInfo, RenderPassCreateInfo,
    SUBPASS_EXTERNAL, SampleCountFlags, SubpassContents, SubpassDependency, SubpassDescription,
};

use crate::application::basic::math::vec4::Vec4;

use super::vulkan_backend::VulkanBackendError;
use super::vulkan_command_buffer::{CommandBufferState, VulkanCommandBuffer};
use super::vulkan_device::VulkanDevice;

type Result<T> = std::result::Result<T, VulkanBackendError>;

pub enum RenderPassState {
    Ready,
    Recording,
    InRenderpass,
    RecordingEnded,
    Submitted,
    NotAllocated,
}

#[derive(PartialEq, Eq)]
pub enum ClearFlag {
    None,
    ColourBuffer = 1 << 1,
    DepthBuffer = 1 << 2,
    StencilBuffer = 1 << 3,
}

impl ClearFlag {
    pub fn value(&self) -> usize {
        match self {
            ClearFlag::None => 0,
            ClearFlag::ColourBuffer => 1 << 1,
            ClearFlag::DepthBuffer => 1 << 2,
            ClearFlag::StencilBuffer => 1 << 3,
        }
    }
    pub fn is_set(flag: ClearFlag, bits: usize) -> bool {
        if flag.value() & bits != 0 {
            true
        } else {
            false
        }
    }
}

pub struct VulkanRenderPass {
    pub renderpass: RenderPass,
    pub render_area: Vec4,
    pub clear_colour: Vec4,
    depth: f32,
    stencil: u32,
    state: RenderPassState,
    clear_flags: usize,
    has_previous_pass: bool,
    has_next_pass: bool,
}

impl VulkanRenderPass {
    pub fn create(
        render_area: Vec4,
        clear_colour: Vec4,
        depth: f32,
        stencil: u32,
        device: &VulkanDevice,
        format: Format,
        depth_format: Format,
        clear_flag: usize,
        has_previous_pass: bool,
        has_next_pass: bool,
    ) -> Result<Self> {
        let mut subpass =
            SubpassDescription::default().pipeline_bind_point(PipelineBindPoint::GRAPHICS);

        let mut attachment_description_ct: usize = 0;
        let mut attachment_descriptions = [AttachmentDescription::default(); 2];

        let color_attachment = AttachmentDescription::default()
            .format(format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(if ClearFlag::is_set(ClearFlag::ColourBuffer, clear_flag) {
                AttachmentLoadOp::CLEAR
            } else {
                AttachmentLoadOp::LOAD
            })
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(if has_previous_pass {
                ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            } else {
                ImageLayout::UNDEFINED
            })
            .final_layout(if has_next_pass {
                ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            } else {
                ImageLayout::PRESENT_SRC_KHR
            })
            .flags(AttachmentDescriptionFlags::empty());

        attachment_descriptions[attachment_description_ct] = color_attachment;
        attachment_description_ct += 1;

        let color_attachment_ref = AttachmentReference::default()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        subpass.p_color_attachments = &color_attachment_ref;
        subpass.color_attachment_count = 1;
        if ClearFlag::is_set(ClearFlag::DepthBuffer, clear_flag) {
            let depth_attachment = AttachmentDescription::default()
                .format(depth_format)
                .samples(SampleCountFlags::TYPE_1)
                .load_op(AttachmentLoadOp::CLEAR)
                .store_op(AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                .initial_layout(ImageLayout::UNDEFINED)
                .final_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            let depth_attachment_ref = AttachmentReference::default()
                .attachment(1)
                .layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            attachment_descriptions[attachment_description_ct] = depth_attachment;

            subpass.p_depth_stencil_attachment = &depth_attachment_ref;
            attachment_description_ct += 1;
        } else {
            unsafe { attachment_descriptions[attachment_description_ct] = std::mem::zeroed() }
        }

        let dependency = SubpassDependency::default()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dependency_flags(DependencyFlags::empty());

        let renderpass_create_info = RenderPassCreateInfo::default()
            .attachments(&attachment_descriptions[0..attachment_description_ct])
            .dependencies(std::slice::from_ref(&dependency))
            .subpasses(std::slice::from_ref(&subpass));

        let renderpass = unsafe {
            match device
                .device
                .create_render_pass(&renderpass_create_info, None)
            {
                Ok(r) => r,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create renderpass",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        Ok(VulkanRenderPass {
            renderpass: renderpass,
            render_area: render_area,
            clear_colour: clear_colour,
            depth: depth,
            stencil: stencil,
            state: RenderPassState::Ready,
            has_next_pass: has_next_pass,
            has_previous_pass: has_previous_pass,
            clear_flags: clear_flag,
        })
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device.device.destroy_render_pass(self.renderpass, None);
        }
    }

    pub fn begin(
        &self,
        device: &VulkanDevice,
        command_buffer: &mut VulkanCommandBuffer,
        index: usize,
        frame_buffer: Framebuffer,
    ) {
        let mut begin_info = RenderPassBeginInfo::default()
            .render_pass(self.renderpass)
            .framebuffer(frame_buffer)
            .render_area(
                Rect2D::default()
                    .extent(
                        Extent2D::default()
                            .height(self.render_area.get_h() as u32)
                            .width(self.render_area.get_w() as u32),
                    )
                    .offset(
                        Offset2D::default()
                            .x(self.render_area.get_x() as i32)
                            .y(self.render_area.get_y() as i32),
                    ),
            );

        let mut clear_value_count = 0;

        let mut clear_values: [ClearValue; 2];
        clear_values = [ClearValue::default(), ClearValue::default()];
        if ClearFlag::is_set(ClearFlag::ColourBuffer, self.clear_flags) {
            clear_values[clear_value_count] = ClearValue {
                color: ClearColorValue {
                    float32: self.clear_colour.data,
                },
            };
            clear_value_count += 1;
        }
        if ClearFlag::is_set(ClearFlag::DepthBuffer, self.clear_flags) {
            clear_values[clear_value_count] = ClearValue {
                depth_stencil: ClearDepthStencilValue::default().depth(self.depth).stencil(
                    if ClearFlag::is_set(ClearFlag::StencilBuffer, self.clear_flags) {
                        self.stencil
                    } else {
                        0
                    },
                ),
            };
            clear_value_count += 1;
        }
        if clear_value_count > 0 {
            begin_info.p_clear_values = clear_values.as_ptr();
            begin_info.clear_value_count = clear_value_count as u32;
        }
        unsafe {
            device.device.cmd_begin_render_pass(
                command_buffer.command_buffer[index],
                &begin_info,
                SubpassContents::INLINE,
            )
        };
        command_buffer.state = CommandBufferState::InRenderPass
    }

    pub fn end(
        &self,
        device: &VulkanDevice,
        command_buffer: &mut VulkanCommandBuffer,
        index: usize,
    ) {
        unsafe {
            device
                .device
                .cmd_end_render_pass(command_buffer.command_buffer[index]);
        }
        command_buffer.state = CommandBufferState::Recording;
    }
}
