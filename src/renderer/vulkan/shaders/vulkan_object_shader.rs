use ash::vk::{
    Extent2D, Format, Offset2D, PipelineBindPoint, PipelineShaderStageCreateInfo, Rect2D,
    ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, VertexInputAttributeDescription,
    Viewport,
};

use super::super::{
    vulkan_backend::VulkanError, vulkan_command_buffer::VulkanCommandBuffer,
    vulkan_device::VulkanDevice, vulkan_pipeline::VulkanPipeline,
    vulkan_renderpass::VulkanRenderPass,
};
use crate::application::basic::{
    filesystem::{FileHandle, FileModes},
    math::vec3::Vec3,
};

struct VulkanShaderStage<'a> {
    //create_info: ShaderModuleCreateInfo<'a>,
    shader_mod: ShaderModule,
    shader_stage_create_info: PipelineShaderStageCreateInfo<'a>,
}

const OBJECT_SHADER_STAGE_COUNT: usize = 2;
const BUILT_IN_NAME: &str = "Builtin.ObjectShader";
pub struct VulkanObjectShader<'a> {
    stages: [VulkanShaderStage<'a>; OBJECT_SHADER_STAGE_COUNT],
    pipeline: VulkanPipeline,
}

impl<'a> VulkanObjectShader<'a> {
    pub fn create(
        device: &VulkanDevice,
        renderpass: &VulkanRenderPass,
        width: u32,
        height: u32,
    ) -> Result<Self, VulkanError> {
        let stage_type_strs = ["vert", "frag"];
        let mut shader_stages: [VulkanShaderStage; OBJECT_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };
        let stage_flags: [ShaderStageFlags; 2] =
            [ShaderStageFlags::VERTEX, ShaderStageFlags::FRAGMENT];
        for i in 0..OBJECT_SHADER_STAGE_COUNT {
            let shader_stage = match Self::create_shader_module(
                device,
                BUILT_IN_NAME,
                stage_type_strs[i],
                stage_flags[i],
            ) {
                Ok(shader_stage) => shader_stage,
                Err(e) => return Err(e),
            };
            shader_stages[i] = shader_stage;
        }

        let viewport = Viewport::default()
            .x(0.0)
            .y(height as f32)
            .width(width as f32)
            .height(-(height as i32) as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        // Scissor

        let scissor = Rect2D::default()
            .offset(Offset2D::default().x(0).y(0))
            .extent(Extent2D::default().height(height).width(width));
        // Attributes
        let mut offset: usize = 0;
        const ATTRIBUTE_COUNT: usize = 1;
        let mut attribute_descriptions: [VertexInputAttributeDescription; ATTRIBUTE_COUNT] =
            unsafe { std::mem::zeroed() };
        // Position
        let formats: [Format; ATTRIBUTE_COUNT] = [Format::R32G32B32_SFLOAT];

        let sizes: [usize; ATTRIBUTE_COUNT] = [size_of::<Vec3>()];

        for (i, attr) in attribute_descriptions.iter_mut().enumerate() {
            *attr = VertexInputAttributeDescription::default()
                .binding(0)
                .location(i as u32)
                .format(formats[i])
                .offset(offset as u32);
            offset += sizes[i];
        }
        //TODO: Desciptor set layouts.

        let mut stage_create_infos: [PipelineShaderStageCreateInfo; OBJECT_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };

        for (size, info) in stage_create_infos.iter_mut().enumerate() {
            *info = shader_stages[size].shader_stage_create_info;
        }

        let pipeline = match VulkanPipeline::create(
            device,
            renderpass,
            &attribute_descriptions,
            None, //descriptor_set_layout,
            &stage_create_infos,
            viewport,
            scissor,
            false,
        ) {
            Ok(p) => p,
            Err(e) => return Err(e),
        };
        Ok(Self {
            stages: shader_stages,
            pipeline: pipeline,
        })
    }
    fn create_shader_module(
        device: &VulkanDevice,
        name: &str,
        stage_type_str: &str,
        stage_flag: ShaderStageFlags,
    ) -> Result<VulkanShaderStage<'a>, VulkanError> {
        let file_name = format!("bin/assets/shaders/{}.{}.spv", name, stage_type_str);
        println!("file name: {}", file_name);
        let mut file_handle = match FileHandle::open(&file_name, FileModes::READ, false) {
            Ok(f) => f,
            Err(_) => return Err(VulkanError::OperationFailed("could not open file")),
        };
        let code = match ash::util::read_spv(&mut file_handle.file) {
            Ok(c) => c,
            Err(_) => return Err(VulkanError::OperationFailed("could not read spirv file")),
        };

        let module_create_info = ShaderModuleCreateInfo::default().code(&code);
        let shader_module = unsafe {
            match device
                .device
                .create_shader_module(&module_create_info, None)
            {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not create shader module",
                    ));
                }
            }
        };

        let pipeline_shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(stage_flag)
            .module(shader_module)
            .name(c"main");

        Ok(VulkanShaderStage {
            shader_mod: shader_module,
            shader_stage_create_info: pipeline_shader_stage_create_info,
        })
    }

    pub fn use_shader(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        image_index: u32,
    ) {
        self.pipeline.bind(
            device,
            command_buffer,
            image_index as usize,
            PipelineBindPoint::GRAPHICS,
        );
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        self.pipeline.destroy(device);
        for i in &self.stages {
            unsafe { device.device.destroy_shader_module(i.shader_mod, None) };
        }
    }
}
