use ash::vk::{
    BlendFactor, BlendOp, ColorComponentFlags, CompareOp, CullModeFlags, DescriptorSetLayout,
    DynamicState, FrontFace, GraphicsPipelineCreateInfo, LogicOp, Pipeline, PipelineBindPoint,
    PipelineCache, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, PushConstantRange, Rect2D,
    SampleCountFlags, ShaderStageFlags, VertexInputAttributeDescription,
    VertexInputBindingDescription, VertexInputRate, Viewport,
};

use crate::application::basic::math::matrix4::Matrix4;

use super::{
    vulkan_backend::VulkanBackendError, vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice, vulkan_renderpass::VulkanRenderPass,
};

type Result<T> = std::result::Result<T, VulkanBackendError>;

pub struct VulkanPipeline {
    pipeline: Pipeline,
    pub layout: PipelineLayout,
}

impl VulkanPipeline {
    pub fn create(
        device: &VulkanDevice,
        renderpass: &VulkanRenderPass,
        stride: u32,
        attributes: &[VertexInputAttributeDescription],
        descriptor_set_layout: &[DescriptorSetLayout],
        stages: &[PipelineShaderStageCreateInfo],
        viewport: Viewport,
        scissor: Rect2D,
        is_wireframe: bool,
        depth_test_enabled: bool,
    ) -> Result<VulkanPipeline> {
        //view state
        let viewport_state_create_info = PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .viewport_count(1)
            .scissors(std::slice::from_ref(&scissor))
            .scissor_count(1);

        //rasterizer
        let rasterizer_create_info = PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(if is_wireframe {
                PolygonMode::LINE
            } else {
                PolygonMode::FILL
            })
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisampling_create_info = PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let mut depth_stencil = PipelineDepthStencilStateCreateInfo::default();
        if depth_test_enabled {
            depth_stencil = PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false);
        }
        let color_blend_attachement = PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(BlendOp::ADD)
            .color_write_mask(ColorComponentFlags::RGBA);

        let color_blend_state_create_info = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .attachments(std::slice::from_ref(&color_blend_attachement));

        let dynamic_states = [
            DynamicState::VIEWPORT,
            DynamicState::SCISSOR,
            DynamicState::LINE_WIDTH,
        ];
        let dynamic_state_create_info =
            PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let binding_description = VertexInputBindingDescription::default()
            .binding(0)
            .stride(stride)
            .input_rate(VertexInputRate::VERTEX);

        let vertex_input_info = PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(attributes);

        let input_assembly = PipelineInputAssemblyStateCreateInfo::default()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let mut pipeline_layout_create_info =
            PipelineLayoutCreateInfo::default().set_layouts(descriptor_set_layout);

        let push_constant = PushConstantRange::default()
            .stage_flags(ShaderStageFlags::VERTEX)
            .size((size_of::<Matrix4>() * 2) as u32)
            .offset(0);

        pipeline_layout_create_info.p_push_constant_ranges =
            std::slice::from_ref(&push_constant).as_ptr();
        pipeline_layout_create_info.push_constant_range_count = 1;

        let pipeline_layout = unsafe {
            match device
                .device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
            {
                Ok(p) => p,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create pipeline layout",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };

        let pipeline_create_info = GraphicsPipelineCreateInfo::default()
            .stages(stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterizer_create_info)
            .multisample_state(&multisampling_create_info)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blend_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .layout(pipeline_layout)
            .render_pass(renderpass.renderpass)
            .base_pipeline_handle(Pipeline::null())
            .base_pipeline_index(-1);

        let pipeline = unsafe {
            match device.device.create_graphics_pipelines(
                PipelineCache::default(),
                std::slice::from_ref(&pipeline_create_info),
                None,
            ) {
                Ok(p) => p,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create pipeline",
                        file: file!(),
                        line: line!(),
                    });
                }
            }
        };
        Ok(VulkanPipeline {
            // temporary
            pipeline: *pipeline.first().unwrap(),
            layout: pipeline_layout,
        })
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device.device.destroy_pipeline(self.pipeline, None);
            device.device.destroy_pipeline_layout(self.layout, None);
        }
    }

    pub fn bind(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        index: usize,
        bind_point: PipelineBindPoint,
    ) {
        unsafe {
            device.device.cmd_bind_pipeline(
                command_buffer.command_buffer[index],
                bind_point,
                self.pipeline,
            );
        }
    }
}
