use std::fs;

use crate::application::{
    basic::math::consts::INVALID_ID,
    resources::resource_types::{ImageData, Resource, ResourceData},
    systems::resource_system::ResourceSysError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageLoaderError {
    #[error("image loader error: opening texture file failed {file} {line}")]
    ImageError {
        source: image::ImageError,
        file: &'static str,
        line: u32,
    },
}
type Result<T> = std::result::Result<T, ResourceSysError>;
pub struct ImageLoader;

impl ImageLoader {
    pub fn load(name: &str, path: &str, base_path: &str) -> Result<Resource> {
        // FIXME: need to find name with out extension
        let directory = format!("{}/{}", base_path, path);
        let entries =
            fs::read_dir(&directory).map_err(|_| ResourceSysError::ResourceLoadError {
                name: name.to_string(),
                file: file!(),
                line: line!(),
            })?;

        let mut full_filename = String::from("");
        let mut found = false;
        for entry in entries {
            if let Ok(e) = entry {
                let full_path = e.path();
                let file_name = if full_path.file_stem().is_none() {
                    continue;
                } else {
                    full_path.file_stem().unwrap()
                };
                let file = file_name.to_str().unwrap();
                if file == name {
                    full_filename = full_path.into_os_string().into_string().map_err(|_| {
                        ResourceSysError::ResourceLoadError {
                            name: "could not convert pathbuf to string".to_string(),
                            file: file!(),
                            line: line!(),
                        }
                    })?;
                    found = true;
                    break;
                }
            } else {
                continue;
            }
        }

        if !found {
            return Err(ResourceSysError::ImageLoaderError {
                file: file!(),
                line: line!(),
            });
        }

        let data = image::open(&full_filename)
            .map_err(|e| ResourceSysError::ImageError {
                source: e,
                file: file!(),
                line: line!(),
            })?
            .flipv()
            .to_rgba8();
        let width = data.width();
        let height = data.height();
        let pixels = data.into_raw();
        let channel_count: u8 = 4;

        let res_data = ResourceData::ImageResourceData(ImageData {
            channel_count,
            width,
            height,
            pixels,
        });

        let res = Resource {
            loader_id: INVALID_ID,
            name: name.to_string(),
            full_path: full_filename,
            data: res_data,
        };
        Ok(res)
    }

    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }
}
