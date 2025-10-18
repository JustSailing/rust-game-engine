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
        let mut file_path = format!("{}/{}/{}.{}", base_path, path, name, "jpg");
        file_path = file_path
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '\n')
            .collect();

        let data = image::open(&file_path)
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
            full_path: file_path,
            data: res_data,
        };
        Ok(res)
    }

    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }
}
