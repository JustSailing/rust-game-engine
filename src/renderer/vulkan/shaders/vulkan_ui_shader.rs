use ash::{
    vk::{
        BufferUsageFlags, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, Extent2D, Format, ImageLayout, MemoryMapFlags, MemoryPropertyFlags, Offset2D, PipelineBindPoint, PipelineShaderStageCreateInfo, Rect2D, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, VertexInputAttributeDescription, Viewport, WriteDescriptorSet
    }, Instance
};

use crate::application::{
    basic::math::{consts::INVALID_ID, matrix4::Matrix4, vec2::Vec2, vec3::Vector2D, vec4::Vec4}, renderer::{
        renderer_types::{UIglobalUBO, UIinstanceUBO},
        vulkan::{
            vulkan_backend::VulkanBackendError, vulkan_buffer::VulkanBuffer, vulkan_command_buffer::VulkanCommandBuffer, vulkan_device::VulkanDevice, vulkan_pipeline::VulkanPipeline, vulkan_renderpass::VulkanRenderPass
        },
    }, resources::resource_types::{Material, ResourceData, ResourceType, TextureUse}, systems::resource_system::ResourceSystem
};

const VULKAN_MAX_UI_COUNT: usize = 1024;
const VULKAN_UI_SHADER_DESCRIPTOR_COUNT: usize = 2;
const VULKAN_UI_SHADER_SAMPLER_COUNT: usize = 1;
const UI_SHADER_STAGE_COUNT: usize = 2;
const BUILT_IN_NAME: &str = "Builtin.UIshader";

type Result<T> = std::result::Result<T, VulkanBackendError>;

#[derive(Clone, Copy)]
struct VulkanDescriptorState {
    generations: [usize; 3],
}

#[derive(Clone, Copy)]
struct VulkanUIshaderInstanceState {
    descriptor_sets: [DescriptorSet; 3],
    descriptor_states: [VulkanDescriptorState; VULKAN_UI_SHADER_DESCRIPTOR_COUNT],
}

struct VulkanShaderStage<'a> {
    shader_mod: ShaderModule,
    shader_stage_create_info: PipelineShaderStageCreateInfo<'a>,
}

pub struct VulkanUIshader<'a> {
    stages: [VulkanShaderStage<'a>; UI_SHADER_STAGE_COUNT],
    pipeline: VulkanPipeline,
    global_descriptor_set_layout: DescriptorSetLayout,
    global_descriptor_pool: DescriptorPool,
    global_descriptor_sets: Vec<DescriptorSet>,
    global_uniform_buffer: VulkanBuffer,
    pub global_ubo: UIglobalUBO,
    object_descriptor_pool: DescriptorPool,
    object_descriptor_set_layout: DescriptorSetLayout,
    object_uniform_buffer: VulkanBuffer,
    object_uniform_buffer_index: u32,
    instance_states: [VulkanUIshaderInstanceState; VULKAN_MAX_UI_COUNT],
    sampler_uses: [TextureUse; VULKAN_UI_SHADER_SAMPLER_COUNT],
}

impl<'a> VulkanUIshader<'a> {
    pub fn create(
        instance: &Instance,
        device: &VulkanDevice,
        renderpass: &VulkanRenderPass,
        width: u32,
        height: u32,
        max_frames: u32,
        //default_diffuse: Option<&'a Texture>,
    ) -> Result<Self> {
        let stage_type_strs = ["vert", "frag"];
        let mut shader_stages: [VulkanShaderStage; UI_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };
        let stage_flags: [ShaderStageFlags; 2] =
            [ShaderStageFlags::VERTEX, ShaderStageFlags::FRAGMENT];
        for i in 0..UI_SHADER_STAGE_COUNT {
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create global descriptor set layout",
                    });
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create descritpor pool",
                    });
                }
            }
        };
        const LOCAL_SAMPLER_COUNT: u32 = 1;
        let descriptor_types: [DescriptorType; VULKAN_UI_SHADER_DESCRIPTOR_COUNT] = [
            DescriptorType::UNIFORM_BUFFER,         // binding 0 - uniform buffer
            DescriptorType::COMBINED_IMAGE_SAMPLER, // binding 1 - diffuse sampler
        ];

        let mut bindings: [DescriptorSetLayoutBinding; VULKAN_UI_SHADER_DESCRIPTOR_COUNT] =
            unsafe { std::mem::zeroed() };

        for i in 0..VULKAN_UI_SHADER_DESCRIPTOR_COUNT {
            bindings[i] = DescriptorSetLayoutBinding::default()
                .binding(i as u32)
                .descriptor_count(1)
                .descriptor_type(descriptor_types[i])
                .stage_flags(ShaderStageFlags::FRAGMENT);
        }

        let layout_info = DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        let object_descriptor_layout = unsafe {
            match device
                .device
                .create_descriptor_set_layout(&layout_info, None)
            {
                Ok(d) => d,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create object descriptor layout",
                    });
                }
            }
        };

        let mut object_pool_sizes: [DescriptorPoolSize; VULKAN_UI_SHADER_DESCRIPTOR_COUNT] =
            [DescriptorPoolSize::default(); VULKAN_UI_SHADER_DESCRIPTOR_COUNT];
        object_pool_sizes[0].descriptor_count = VULKAN_MAX_UI_COUNT as u32;
        object_pool_sizes[0].ty = DescriptorType::UNIFORM_BUFFER;

        object_pool_sizes[1].descriptor_count =
            LOCAL_SAMPLER_COUNT * VULKAN_MAX_UI_COUNT as u32;
        object_pool_sizes[1].ty = DescriptorType::COMBINED_IMAGE_SAMPLER;

        let object_pool_info = DescriptorPoolCreateInfo::default()
            .pool_sizes(&object_pool_sizes)
            .max_sets(VULKAN_MAX_UI_COUNT as u32);

        let object_descriptor_pool = unsafe {
            match device
                .device
                .create_descriptor_pool(&object_pool_info, None)
            {
                Ok(p) => p,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create descriptor pool",
                    });
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
        const ATTRIBUTE_COUNT: usize = 2;

        let mut attribute_descriptions: [VertexInputAttributeDescription; ATTRIBUTE_COUNT] =
            [VertexInputAttributeDescription::default(); ATTRIBUTE_COUNT];
        // Position
        let formats: [Format; ATTRIBUTE_COUNT] = [Format::R32G32_SFLOAT, Format::R32G32_SFLOAT];

        let sizes: [usize; ATTRIBUTE_COUNT] = [size_of::<Vec2>(), size_of::<Vec2>()];

        for (i, attr) in attribute_descriptions.iter_mut().enumerate() {
            *attr = VertexInputAttributeDescription::default()
                .binding(0)
                .location(i as u32)
                .format(formats[i])
                .offset(offset as u32);
            offset += sizes[i];
        }

        const DESCRIPTOR_SET_LAYOUT_COUNT: usize = 2;
        let layouts: [DescriptorSetLayout; DESCRIPTOR_SET_LAYOUT_COUNT] =
            [global_descritpor_set_layout, object_descriptor_layout];

        let mut stage_create_infos: [PipelineShaderStageCreateInfo; UI_SHADER_STAGE_COUNT] =
            unsafe { std::mem::zeroed() };

        for (size, info) in stage_create_infos.iter_mut().enumerate() {
            *info = shader_stages[size].shader_stage_create_info;
        }

        let pipeline = VulkanPipeline::create(
            device,
            renderpass,
            size_of::<Vector2D>() as u32,
            &attribute_descriptions,
            &layouts, //descriptor_set_layout,
            &stage_create_infos,
            viewport,
            scissor,
            false,
            false,
        )?;

        let global_buffer = VulkanBuffer::create(
            instance,
            device,
            (size_of::<UIglobalUBO>() as u64) * max_frames as u64,
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
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not allocate descriptor sets".into(),
                    });
                }
            }
        };
        let ubo = UIglobalUBO {
            projection: Matrix4::new_zeros(),
            view: Matrix4::new_zeros(),
            padding: [Matrix4::identity(), Matrix4::identity()],
        };

        let object_buffer = VulkanBuffer::create(
            instance,
            device,
            (size_of::<UIinstanceUBO>() as u64) * max_frames as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            true,
        )?;

        Ok(Self {
            stages: shader_stages,
            pipeline: pipeline,
            global_descriptor_pool,
            global_descriptor_sets: global_descriptor_sets,
            global_uniform_buffer: global_buffer,
            global_ubo: ubo,
            global_descriptor_set_layout: global_descritpor_set_layout,
            object_descriptor_pool: object_descriptor_pool,
            object_descriptor_set_layout: object_descriptor_layout,
            object_uniform_buffer: object_buffer,
            object_uniform_buffer_index: 0,
            instance_states: [VulkanUIshaderInstanceState {
                descriptor_sets: [DescriptorSet::default(); 3],
                descriptor_states: [VulkanDescriptorState {
                    generations: [INVALID_ID; 3],
                }; VULKAN_UI_SHADER_DESCRIPTOR_COUNT],
            }; VULKAN_MAX_UI_COUNT],
            sampler_uses: [TextureUse::Unknown; VULKAN_UI_SHADER_SAMPLER_COUNT],
        })
    }
    fn create_shader_module(
        device: &VulkanDevice,
        name: &str,
        stage_type_str: &str,
        stage_flag: ShaderStageFlags,
    ) -> Result<VulkanShaderStage<'a>> {
        let file_name = format!("shaders/{}.{}.spv", name, stage_type_str);
        //println!("file name: {}", file_name);
        let bin_res = ResourceSystem::load(&file_name, ResourceType::Binary)?;
        let data = match bin_res.data {
            ResourceData::Unknown => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type: Unknown expected: Binary",
                });
            }
            ResourceData::ImageResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type: Image expected: Binary",
                });
            }
            ResourceData::MaterialResourceData(_) => {
                return Err(VulkanBackendError::OperationFailed {
                    issue: "wrong resource type: Material expected: Binary",
                });
            }
            ResourceData::BinaryResourceData(ref items) => items,
        };

        let (_, code, _) = unsafe { data.align_to::<u32>() };

        let module_create_info = ShaderModuleCreateInfo::default().code(code);
        let shader_module = unsafe {
            match device
                .device
                .create_shader_module(&module_create_info, None)
            {
                Ok(s) => s,
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not create shader module".into(),
                    });
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

    pub fn set_model(
        &mut self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        image_index: u32,
        model: Matrix4,
    ) -> Result<()> {
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
        Ok(())
    }

    pub fn apply_material(
        &mut self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        image_index: u32,
        material: &mut Material,
    ) -> Result<()> {
        let object_id = material.internal_id;
        let object_state = &mut self.instance_states[object_id];
        let object_descriptor = object_state.descriptor_sets[image_index as usize];

        let mut descriptor_writes: [WriteDescriptorSet; VULKAN_UI_SHADER_DESCRIPTOR_COUNT] =
            [WriteDescriptorSet::default(); VULKAN_UI_SHADER_DESCRIPTOR_COUNT];

        let mut descriptor_count = 0;
        let mut descriptor_index = 0;

        let range = size_of::<UIinstanceUBO>();
        let offset = size_of::<UIinstanceUBO>() * object_id;
        let mut obo = UIinstanceUBO {
            diffuse_color: Vec4::new_zeroes(),
            padding: [Vec4::new_zeroes(); 3],
        };

        // static mut ACCUMULATOR: f32 = 0.0;
        // unsafe {
        //     ACCUMULATOR += 0.01;
        // }
        // let s = unsafe { (ACCUMULATOR.sin() + 1.0) / 2.0 };
        obo.diffuse_color = material.diffuse_colour;

        self.object_uniform_buffer.load_data(
            device,
            offset as u64,
            range as u64,
            MemoryMapFlags::empty(),
            std::slice::from_ref(&obo),
        )?;
        let buffer_info = DescriptorBufferInfo::default()
            .buffer(self.object_uniform_buffer.buffer)
            .offset(offset as u64)
            .range(range as u64);

        if object_state.descriptor_states[descriptor_index].generations[image_index as usize]
            == INVALID_ID
        {
            let descriptor = WriteDescriptorSet::default()
                .buffer_info(std::slice::from_ref(&buffer_info))
                .dst_set(object_descriptor)
                .dst_binding(descriptor_index as u32)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1);
            descriptor_writes[descriptor_count] = descriptor;
            descriptor_count += 1;
            object_state.descriptor_states[descriptor_index].generations[image_index as usize] =
                material.generation;
        }
        descriptor_index += 1;

        const SAMPLER_COUNT: usize = 1;
        let mut image_info = [DescriptorImageInfo::default(); SAMPLER_COUNT];
        for (_, d) in image_info.iter_mut().enumerate() {
            let use_type = material.diffuse_map.use_type;
            match use_type {
                TextureUse::Unknown => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "unable to bind sample to unknown use",
                    });
                }
                TextureUse::MapDiffuse => {}
            };

            let texture = material.diffuse_map.texture.as_mut().unwrap();

            let descriptor_generaton = &mut object_state.descriptor_states[descriptor_index]
                .generations[image_index as usize];
            if *descriptor_generaton != texture.generation as usize
                || *descriptor_generaton == INVALID_ID
            {
                *d = DescriptorImageInfo::default()
                    .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(texture.internal_data.image.view.unwrap())
                    .sampler(texture.internal_data.sampler);
                let descriptor = WriteDescriptorSet::default()
                    .dst_set(object_descriptor)
                    .dst_binding(descriptor_index as u32)
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(std::slice::from_ref(d));
                descriptor_writes[descriptor_count as usize] = descriptor;
                descriptor_count += 1;
                if texture.generation != INVALID_ID {
                    *descriptor_generaton = texture.generation;
                } else {
                    texture.generation = *descriptor_generaton;
                }
                descriptor_index += 1;
            }
        }

        if descriptor_count == 1 {
            unsafe {
                device.device.update_descriptor_sets(
                    std::slice::from_ref(&descriptor_writes.first().unwrap()),
                    &[],
                );
            }
        } else if descriptor_count == 2 {
            unsafe {
                device
                    .device
                    .update_descriptor_sets(&descriptor_writes, &[]);
            }
        }
        unsafe {
            device.device.cmd_bind_descriptor_sets(
                command_buffer.command_buffer[image_index as usize],
                PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                1,
                std::slice::from_ref(&object_descriptor),
                &[],
            );
        }
        Ok(())
    }

    pub fn update_global_state(
        &self,
        device: &VulkanDevice,
        command_buffer: &VulkanCommandBuffer,
        current_frame: u32,
        _delta: f32,
    ) -> Result<()> {
        let cmd_buf = command_buffer.command_buffer[current_frame as usize];
        let global_descriptor = self.global_descriptor_sets[current_frame as usize];

        let range = size_of::<UIglobalUBO>();
        let offset = (size_of::<UIglobalUBO>() * (current_frame as usize)) as u64;

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

    pub fn acquire_resources(
        &mut self,
        device: &VulkanDevice,
        material: &mut Material,
    ) -> Result<()> {
        material.internal_id = self.object_uniform_buffer_index as usize;
        self.object_uniform_buffer_index += 1;
        let obj_id = material.internal_id;

        let layouts = [self.object_descriptor_set_layout; 3];
        let instance_state = &mut self.instance_states[obj_id];
        let alloc_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.object_descriptor_pool)
            .set_layouts(&layouts);
        instance_state.descriptor_sets = unsafe {
            match device.device.allocate_descriptor_sets(&alloc_info) {
                Ok(ds) => ds.as_slice().try_into().unwrap(),
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not allocate descriptor sets",
                    });
                }
            }
        };
        Ok(())
    }

    pub fn release_resources(&mut self, device: &VulkanDevice, material: &Material) -> Result<()> {
        let instance_state = &mut self.instance_states[material.internal_id];

        unsafe {
            match device
                .device
                .free_descriptor_sets(self.object_descriptor_pool, &instance_state.descriptor_sets)
            {
                Ok(_) => (),
                Err(_) => {
                    return Err(VulkanBackendError::OperationFailed {
                        issue: "could not free descriptor sets",
                    });
                }
            }
        }

        instance_state.descriptor_states[material.internal_id]
            .generations
            .iter_mut()
            .map(|i| *i = INVALID_ID)
            .count();

        Ok(())
    }

    pub fn destroy(&self, device: &VulkanDevice) {
        unsafe {
            device
                .device
                .destroy_descriptor_pool(self.object_descriptor_pool, None);
            device
                .device
                .destroy_descriptor_set_layout(self.object_descriptor_set_layout, None);
        }
        self.object_uniform_buffer.destroy(device);

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
