use ash::{
    Instance,
    vk::{
        BufferUsageFlags, DescriptorBufferInfo, DescriptorPool, DescriptorPoolCreateInfo,
        DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout,
        DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, Extent2D,
        Format, MemoryMapFlags, MemoryPropertyFlags, Offset2D, PipelineBindPoint,
        PipelineShaderStageCreateInfo, Rect2D, ShaderModule, ShaderModuleCreateInfo,
        ShaderStageFlags, VertexInputAttributeDescription, Viewport, WriteDescriptorSet,
    },
};

use super::super::{
    vulkan_backend::Error as VulkanError, vulkan_buffer::VulkanBuffer,
    vulkan_command_buffer::VulkanCommandBuffer, vulkan_device::VulkanDevice,
    vulkan_pipeline::VulkanPipeline, vulkan_renderpass::VulkanRenderPass,
};
use crate::application::basic::{
    filesystem::{FileHandle, FileModes},
    math::{matrix4::Matrix4, vec3::Vec3},
};
use crate::application::renderer::renderer_types::GlobalUniformObj;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

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
    global_descriptor_set_layout: DescriptorSetLayout,
    global_descriptor_pool: DescriptorPool,
    global_descriptor_sets: Vec<DescriptorSet>,
    global_uniform_buffer: VulkanBuffer,
    pub global_ubo: GlobalUniformObj,
}

impl<'a> VulkanObjectShader<'a> {
    pub fn create(
        instance: &Instance,
        device: &VulkanDevice,
        renderpass: &VulkanRenderPass,
        width: u32,
        height: u32,
        max_frames: u32,
    ) -> Result<Self> {
        let stage_type_strs = ["vert", "frag"];
        let mut shader_stages: [VulkanShaderStage; OBJECT_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };
        let stage_flags: [ShaderStageFlags; 2] =
            [ShaderStageFlags::VERTEX, ShaderStageFlags::FRAGMENT];
        for i in 0..OBJECT_SHADER_STAGE_COUNT {
            let shader_stage = Self::create_shader_module(
                device,
                BUILT_IN_NAME,
                stage_type_strs[i],
                stage_flags[i],
            )?;
            shader_stages[i] = shader_stage;
        }

        //Global Desciptors

        let global_ubo_layout_binding = DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .stage_flags(ShaderStageFlags::VERTEX);

        let global_layout_info = DescriptorSetLayoutCreateInfo::default()
            .bindings(std::slice::from_ref(&global_ubo_layout_binding));

        let global_descritpor_set_layout = unsafe {
            match device
                .device
                .create_descriptor_set_layout(&global_layout_info, None)
            {
                Ok(d) => d,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not create global descriptor set layout".into(),
                    )
                    .into());
                }
            }
        };

        let global_pool_size = DescriptorPoolSize::default()
            .descriptor_count(max_frames)
            .ty(DescriptorType::UNIFORM_BUFFER);

        let global_pool_info = DescriptorPoolCreateInfo::default()
            .pool_sizes(std::slice::from_ref(&global_pool_size))
            .max_sets(max_frames);

        let global_descriptor_pool = unsafe {
            match device
                .device
                .create_descriptor_pool(&global_pool_info, None)
            {
                Ok(p) => p,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not create descritpor pool".into(),
                    )
                    .into());
                }
            }
        };

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

        const DESCRIPTOR_SET_LAYOUT_COUNT: usize = 1;
        let layouts: [DescriptorSetLayout; DESCRIPTOR_SET_LAYOUT_COUNT] =
            [global_descritpor_set_layout];

        let mut stage_create_infos: [PipelineShaderStageCreateInfo; OBJECT_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };

        for (size, info) in stage_create_infos.iter_mut().enumerate() {
            *info = shader_stages[size].shader_stage_create_info;
        }

        let pipeline = VulkanPipeline::create(
            device,
            renderpass,
            &attribute_descriptions,
            &layouts, //descriptor_set_layout,
            &stage_create_infos,
            viewport,
            scissor,
            false,
        )?;

        let global_buffer = VulkanBuffer::create(
            instance,
            device,
            (size_of::<GlobalUniformObj>() as u64) * max_frames as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::DEVICE_LOCAL
                | MemoryPropertyFlags::HOST_VISIBLE
                | MemoryPropertyFlags::HOST_COHERENT,
            true,
        )?;

        let global_layouts = [
            global_descritpor_set_layout,
            global_descritpor_set_layout,
            global_descritpor_set_layout,
        ];

        let descritor_allocate_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(global_descriptor_pool)
            .set_layouts(&global_layouts);

        let global_descriptor_sets = unsafe {
            match device
                .device
                .allocate_descriptor_sets(&descritor_allocate_info)
            {
                Ok(gd) => gd,
                Err(_) => {
                    return Err(VulkanError::OperationFailed(
                        "could not allocate descriptor sets".into(),
                    )
                    .into());
                }
            }
        };
        let ubo = GlobalUniformObj {
            projection: Matrix4::new_zeros(),
            view: Matrix4::new_zeros(),
            padding: [Matrix4::identity(), Matrix4::identity()],
        };

        Ok(Self {
            stages: shader_stages,
            pipeline: pipeline,
            global_descriptor_pool,
            global_descriptor_sets: global_descriptor_sets,
            global_uniform_buffer: global_buffer,
            global_ubo: ubo,
            global_descriptor_set_layout: global_descritpor_set_layout,
        })
    }
    fn create_shader_module(
        device: &VulkanDevice,
        name: &str,
        stage_type_str: &str,
        stage_flag: ShaderStageFlags,
    ) -> Result<VulkanShaderStage<'a>> {
        let file_name = format!("bin/assets/shaders/{}.{}.spv", name, stage_type_str);
        println!("file name: {}", file_name);
        let mut file_handle = match FileHandle::open(&file_name, FileModes::READ, false) {
            Ok(f) => f,
            Err(_) => return Err(VulkanError::OperationFailed("could not open file".into()).into()),
        };
        let code = match ash::util::read_spv(&mut file_handle.file) {
            Ok(c) => c,
            Err(_) => {
                return Err(
                    VulkanError::OperationFailed("could not read spirv file".into()).into(),
                );
            }
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
                        "could not create shader module".into(),
                    )
                    .into());
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

    pub fn update_object(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        image_index: u32,
        model: Matrix4,
    ) {
        let cmd_buf = command_buffer.command_buffer[image_index as usize];
        unsafe {
            device.device.cmd_push_constants(
                cmd_buf,
                self.pipeline.layout,
                ShaderStageFlags::VERTEX,
                0,
                std::slice::from_raw_parts(
                    &model as *const Matrix4 as *const u8,
                    size_of::<Matrix4>(),
                ),
            );
        }
    }

    pub fn update_global_state(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        current_frame: u32,
    ) -> Result<()> {
        let cmd_buf = command_buffer.command_buffer[current_frame as usize];
        let global_descriptor = self.global_descriptor_sets[current_frame as usize];

        let range = size_of::<GlobalUniformObj>();
        let offset = (size_of::<GlobalUniformObj>() * (current_frame as usize)) as u64;

        self.global_uniform_buffer.load_data(
            device,
            offset,
            range as u64, //size_of_val(&arr) as u64,
            MemoryMapFlags::empty(),
            std::slice::from_ref(&self.global_ubo),
        )?;

        let buffer_info = DescriptorBufferInfo::default()
            .buffer(self.global_uniform_buffer.buffer)
            .offset(offset)
            .range(range as u64);

        let descriptor_write = WriteDescriptorSet::default()
            .dst_set(self.global_descriptor_sets[current_frame as usize])
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .dst_binding(0)
            .dst_array_element(0);

        unsafe {
            device
                .device
                .update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[]);
        }
        unsafe {
            device.device.cmd_bind_descriptor_sets(
                cmd_buf,
                PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                std::slice::from_ref(&global_descriptor),
                &[],
            );
        }
        Ok(())
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        self.global_uniform_buffer.destroy(device);
        self.pipeline.destroy(device);
        unsafe {
            device
                .device
                .destroy_descriptor_pool(self.global_descriptor_pool, None);
            device
                .device
                .destroy_descriptor_set_layout(self.global_descriptor_set_layout, None);
        }
        for i in &self.stages {
            unsafe { device.device.destroy_shader_module(i.shader_mod, None) };
        }
    }
}
