use crate::application::{
    basic::{
        filesystem::{FileHandle, FileModes},
        math::consts::INVALID_ID,
    },
    resources::resource_types::{Resource, ResourceData},
    systems::resource_system::ResourceSysError,
};

type Result<T> = std::result::Result<T, ResourceSysError>;
pub struct BinaryLoader;

impl BinaryLoader {
    pub fn load(name: &str, path: &str, _base_path: &str) -> Result<Resource> {
        let file_path = format!("{}/{}/{}", "bin/assets", path, name,);

        let mut file = FileHandle::open(&file_path, FileModes::READ, true).map_err(|e| {
            ResourceSysError::FileError {
                source: e,
                file: file!(),
                line: line!(),
            }
        })?;
        let v = file
            .read_all_bytes()
            .map_err(|e| ResourceSysError::FileError {
                source: e,
                file: file!(),
                line: line!(),
            })?;
        let res = Resource {
            loader_id: INVALID_ID,
            name: name.to_string(),
            full_path: file_path,
            data: ResourceData::BinaryResourceData(v),
        };
        Ok(res)
    }

    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }
}
