use crate::application::{
    basic::{
        filesystem::{FileHandle, FileModes},
        math::{consts::INVALID_ID, vec4::Vec4},
    },
    resources::resource_types::{MaterialConfig, Resource, ResourceData},
    systems::resource_system::ResourceSysError,
};

type Result<T> = std::result::Result<T, ResourceSysError>;
pub struct MaterialLoader;

impl MaterialLoader {
    pub fn load(name: &str, path: &str, base_path: &str) -> Result<Resource> {
        let full_path = format!("{}/{}/{}.{}", base_path, path, name, "gmt");
        let mut file_handle =
            FileHandle::open(&full_path, FileModes::READ, false).map_err(|e| {
                ResourceSysError::FileError {
                    source: e,
                    file: file!(),
                    line: line!(),
                }
            })?;
        let mut config = MaterialConfig::default();
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
            }

            let split: Vec<&str> = line.split('=').collect();
            match split[0] {
                "name" => config.name = split[1].trim().to_string(),
                "diffuse_colour" => {
                    let values: Vec<&str> = split[1].split(' ').collect();
                    let mut dif_col = Vec4::new_zeroes();
                    for (i, value) in values.iter().enumerate() {
                        dif_col.data[i] = value.trim().parse::<f32>().unwrap();
                    }
                    config.diffuse_colour = dif_col;
                }
                "diffuse_map_name" => config.diffuse_map_name = split[1].trim().to_string(),
                "specular_map_name" => config.specular_map_name = split[1].trim().to_string(),
                "normal_map_name" => config.normal_map_name = split[1].trim().to_string(),
                "shininess" => {
                    config.shininess = split[1].trim().to_string().parse::<f32>().unwrap()
                }
                "shader" => config.shader_name = split[1].trim().to_string(),
                _ => println!(
                    "{}={} not added to material config",
                    split[0].trim(),
                    split[1].trim()
                ),
            }
        }

        let res_data = ResourceData::MaterialResourceData(config);

        let res = Resource {
            loader_id: INVALID_ID,
            name: name.to_string(),
            full_path,
            data: res_data,
        };
        Ok(res)
    }

    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }
}
