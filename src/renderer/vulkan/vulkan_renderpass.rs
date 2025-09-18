use ash::vk::{
    AccessFlags, AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, ClearColorValue, ClearDepthStencilValue, ClearValue,
    DependencyFlags, Extent2D, Format, Framebuffer, ImageLayout, Offset2D, PipelineBindPoint,
    PipelineStageFlags, Rect2D, RenderPass, RenderPassBeginInfo, RenderPassCreateInfo,
    SUBPASS_EXTERNAL, SampleCountFlags, SubpassContents, SubpassDependency, SubpassDescription,
};

use super::vulkan_backend::VulkanError;
use super::vulkan_command_buffer::{CommandBufferState, VulkanCommandBuffer};
use super::vulkan_device::VulkanDevice;

pub enum RenderPassState {
    Ready,
    Recording,
    InRenderpass,
    RecordingEnded,
    Submitted,
    NotAllocated,
}

pub struct VulkanRenderPass {
    pub renderpass: RenderPass,
    x: f32,
    y: f32,
    pub w: f32,
    pub h: f32,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
    depth: f32,
    stencil: u32,
    state: RenderPassState,
}

impl VulkanRenderPass {
    pub fn create(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        depth: f32,
        stencil: u32,
        device: &VulkanDevice,
        format: Format,
        depth_format: Format,
    ) -> Result<Self, VulkanError> {
        let mut subpass =
            SubpassDescription::default().pipeline_bind_point(PipelineBindPoint::GRAPHICS);

        const ATTACHMENT_DESCRIPTION_CT: usize = 2;
        let mut attachment_descriptions =
            [AttachmentDescription::default(); ATTACHMENT_DESCRIPTION_CT];

        let color_attachment = AttachmentDescription::default()
            .format(format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(ImageLayout::UNDEFINED)
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            .flags(AttachmentDescriptionFlags::default());

        attachment_descriptions[0] = color_attachment;

        let color_attachment_ref = AttachmentReference::default()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        subpass.p_color_attachments = &color_attachment_ref;
        subpass.color_attachment_count = 1;

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

        attachment_descriptions[1] = depth_attachment;

        subpass.p_depth_stencil_attachment = &depth_attachment_ref;

        let dependency = SubpassDependency::default()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::default())
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dependency_flags(DependencyFlags::default());
        let dependencies = [dependency];
        let subpasses = [subpass];
        let renderpass_create_info = RenderPassCreateInfo::default()
            .attachments(&attachment_descriptions)
            .dependencies(&dependencies)
            .subpasses(&subpasses);
        let renderpass = unsafe {
            match device
                .device
                .create_render_pass(&renderpass_create_info, None)
            {
                Ok(r) => r,
                Err(_) => return Err(VulkanError::OperationFailed("could not create renderpass")),
            }
        };
        Ok(VulkanRenderPass {
            renderpass: renderpass,
            x: x,
            y: y,
            w: w,
            h: h,
            r: r,
            g: g,
            b: b,
            a: a,
            depth: depth,
            stencil: stencil,
            state: RenderPassState::Ready,
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
                            .height(self.h as u32)
                            .width(self.w as u32),
                    )
                    .offset(Offset2D::default().x(self.x as i32).y(self.y as i32)),
            );
        let mut clear_values: [ClearValue; 2];
        unsafe {
            clear_values = [std::mem::zeroed(), std::mem::zeroed()];
        }
        clear_values[0] = ClearValue {
            color: ClearColorValue {
                float32: [self.r, self.g, self.b, self.a],
            },
        };
        clear_values[1] = ClearValue {
            depth_stencil: ClearDepthStencilValue {
                depth: self.depth,
                stencil: self.stencil,
            },
        };

        begin_info.p_clear_values = clear_values.as_ptr();
        begin_info.clear_value_count = 2;
        unsafe {
            device.device.cmd_begin_render_pass(
                command_buffer.command_buffer[index],
                &begin_info,
                SubpassContents::INLINE,
            )
        };
        command_buffer.state = CommandBufferState::InRenderPass
    }

    pub fn end(&self, device: &VulkanDevice, command_buffer: &mut VulkanCommandBuffer, index: usize) {
        unsafe {
            device
                .device
                .cmd_end_render_pass(command_buffer.command_buffer[index]);
        }
        command_buffer.state = CommandBufferState::Recording;
    }
}
