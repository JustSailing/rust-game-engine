use crate::application::{
    basic::{
        filesystem::{FileHandle, FileModes},
        math::consts::INVALID_ID,
    },
    resources::resource_types::{
        Resource, ResourceData, ShaderAttributeConfig, ShaderAttributeType, ShaderConfig,
        ShaderScope, ShaderStage, ShaderUniformConfig, ShaderUniformType,
    },
    systems::resource_system::ResourceSysError,
};

type Result<T> = std::result::Result<T, ResourceSysError>;

pub struct ShaderLoader;

impl ShaderLoader {
    pub fn load(name: &str, path: &str, base_path: &str) -> Result<Resource> {
        let full_path = format!("{}/{}/{}.config", base_path, path, name);
        let mut file_handle =
            FileHandle::open(&full_path, FileModes::READ, false).map_err(|e| {
                ResourceSysError::FileError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?;

        let mut shader_config = ShaderConfig::default();

        let lines = file_handle
            .read_lines()
            .map_err(|e| ResourceSysError::FileError {
                source: e,
                file: file!(),
                line: line!(),
            })?;

        for line in lines.iter() {
            if line.len() == 0 {
                continue;
            } else if line.chars().nth(0).unwrap() == '#' {
                continue;
            } else if line.chars().nth(0).unwrap() == '\n' {
                continue;
            }

            let split: Vec<&str> = line.split('=').collect();
            match split[0].trim() {
                "name" => shader_config.name = split[1].trim().to_string(),
                "renderpass" => shader_config.renderpass_name = split[1].trim().to_string(),
                "stages" => {
                    let shader_stages: Vec<&str> = split[1].split(',').map(|s| s.trim()).collect();
                    shader_stages
                        .iter()
                        .map(|stage| match *stage {
                            "vertex" => shader_config.stages.push(ShaderStage::Vertex),
                            "fragment" => shader_config.stages.push(ShaderStage::Fragment),
                            "compute" => shader_config.stages.push(ShaderStage::Compute),
                            "geometry" => shader_config.stages.push(ShaderStage::Geometry),
                            _ => println!("{} no support for shader stage", stage),
                        })
                        .count();
                }
                "stagefiles" => {
                    let files: Vec<&str> = split[1].split(',').map(|s| s.trim()).collect();
                    files
                        .iter()
                        .map(|file| {
                            shader_config.stage_filenames.push(file.to_string());
                        })
                        .count();
                }
                "use_local" => {
                    let check =
                        split[1]
                            .trim()
                            .parse::<i32>()
                            .map_err(|e| ResourceSysError::ParseErr {
                                source: e,
                                file: file!(),
                                line: line!(),
                            })?;
                    shader_config.use_locals = if 1 == check { true } else { false };
                }
                "use_instance" => {
                    let check =
                        split[1]
                            .trim()
                            .parse::<i32>()
                            .map_err(|e| ResourceSysError::ParseErr {
                                source: e,
                                file: file!(),
                                line: line!(),
                            })?;
                    shader_config.use_instances = if 1 == check { true } else { false };
                }
                "attribute" => {
                    let attr: Vec<&str> = split[1].split(',').map(|s| s.trim()).collect();
                    let shader_attr = Self::get_attribute(&attr);
                    if shader_attr.attribute_type != ShaderAttributeType::Unknown {
                        shader_config.attributes.push(shader_attr);
                    }
                }
                "uniform" => {
                    let uniform: Vec<&str> = split[1].split(',').map(|s| s.trim()).collect();
                    let uniform_config = Self::get_uniform(&uniform)?;
                    if uniform_config.uniform_type != ShaderUniformType::Unknown {
                        shader_config.uniforms.push(uniform_config);
                    }
                }
                _ => println!("{split:?} not added to shader config",),
            }
        }
        let res_data = ResourceData::ShaderResourceData(shader_config);
        let mut res = Resource::default();
        res.loader_id = INVALID_ID;
        res.full_path = full_path;
        res.name = name.to_string();
        res.data = res_data;
        Ok(res)
    }

    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }

    fn get_attribute(attr: &Vec<&str>) -> ShaderAttributeConfig {
        match attr[0] {
            "f32" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 4,
                attribute_type: ShaderAttributeType::Float32,
            },
            "vec2" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 8,
                attribute_type: ShaderAttributeType::Float32_2,
            },
            "vec3" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 12,
                attribute_type: ShaderAttributeType::Float32_2,
            },
            "vec4" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 16,
                attribute_type: ShaderAttributeType::Float32_4,
            },
            "u8" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 1,
                attribute_type: ShaderAttributeType::Uint8,
            },
            "u16" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 2,
                attribute_type: ShaderAttributeType::Uint16,
            },
            "u32" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 4,
                attribute_type: ShaderAttributeType::Uint32,
            },
            "i8" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 1,
                attribute_type: ShaderAttributeType::Int8,
            },
            "i16" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 2,
                attribute_type: ShaderAttributeType::Int16,
            },
            "i32" => ShaderAttributeConfig {
                name: attr[1].to_string(),
                size: 4,
                attribute_type: ShaderAttributeType::Int32,
            },
            _ => {
                println!(
                    "warning: unsupported attributed type: {}, name: {}",
                    attr[0], attr[1]
                );
                ShaderAttributeConfig {
                    name: attr[1].to_string(),
                    size: 0,
                    attribute_type: ShaderAttributeType::Unknown,
                }
            }
        }
    }

    fn get_uniform(uniform: &Vec<&str>) -> Result<ShaderUniformConfig> {
        match uniform[0] {
            "f32" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 4,
                    location: 0,
                    uniform_type: ShaderUniformType::Float32,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "vec2" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 8,
                    location: 0,
                    uniform_type: ShaderUniformType::Float32_2,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "vec3" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 12,
                    location: 0,
                    uniform_type: ShaderUniformType::Float32_3,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "vec4" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 16,
                    location: 0,
                    uniform_type: ShaderUniformType::Float32_4,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "u8" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 1,
                    location: 0,
                    uniform_type: ShaderUniformType::Uint8,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "u16" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 2,
                    location: 0,
                    uniform_type: ShaderUniformType::Uint16,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "u32" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 4,
                    location: 0,
                    uniform_type: ShaderUniformType::Uint32,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "i8" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 1,
                    location: 0,
                    uniform_type: ShaderUniformType::Int8,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "i16" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 2,
                    location: 0,
                    uniform_type: ShaderUniformType::Int16,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "i32" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 4,
                    location: 0,
                    uniform_type: ShaderUniformType::Int32,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "mat4" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 64,
                    location: 0,
                    uniform_type: ShaderUniformType::Matrix4,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            "samp" => {
                let scope = uniform[1]
                    .parse::<u32>()
                    .map_err(|e| ResourceSysError::ParseErr {
                        source: e,
                        file: file!(),
                        line: line!(),
                    })?;
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 0,
                    location: 0,
                    uniform_type: ShaderUniformType::Sampler,
                    scope: match scope {
                        0 => ShaderScope::Global,
                        1 => ShaderScope::Instance,
                        2 => ShaderScope::Local,
                        _ => {
                            println!("warning: shader scope should be 0..2");
                            ShaderScope::Unknown
                        }
                    },
                })
            }
            _ => {
                println!(
                    "warning: unsupported attributed type: {}, scope: {}, name: {}",
                    uniform[0], uniform[1], uniform[2]
                );
                Ok(ShaderUniformConfig {
                    name: uniform[2].to_string(),
                    size: 0,
                    location: 0,
                    uniform_type: ShaderUniformType::Unknown,
                    scope: ShaderScope::Unknown,
                })
            }
        }
    }
}
