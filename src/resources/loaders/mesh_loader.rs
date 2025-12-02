use std::mem::MaybeUninit;
use std::path::Path;

use crate::application::basic::filesystem::{FileHandle, FileModes};
use crate::application::basic::math::vec4::Vec4;
use crate::application::basic::math::{consts::INVALID_ID, vec2::Vec2, vec3::Vec3, vec3::Vector3D};
use crate::application::resources::resource_types::{
    GeometryConfig, MaterialConfig, Resource, ResourceData,
};
use crate::application::systems::geometry_system::{
    geometry_deduplicate_vertices, geometry_generate_tangents,
};
use crate::application::systems::resource_system::ResourceSysError;

type Result<T> = std::result::Result<T, ResourceSysError>;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum MeshFileType {
    NotFound,
    GSM,
    OBJ,
}

pub struct SupportedMeshFileTypes {
    pub extensions: &'static str,
    pub mesh_file_type: MeshFileType,
    pub is_binary: bool,
}

pub struct MeshVertexIndexData {
    pub position_index: u32,
    pub normal_index: u32,
    pub coord_index: u32,
}

impl Default for MeshVertexIndexData {
    fn default() -> Self {
        Self {
            position_index: INVALID_ID as u32,
            normal_index: INVALID_ID as u32,
            coord_index: INVALID_ID as u32,
        }
    }
}

pub struct MeshFaceData {
    pub vertices: [MeshVertexIndexData; 3],
}

impl Default for MeshFaceData {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
        }
    }
}

pub struct MeshGroupData {
    faces: Vec<MeshFaceData>,
}

pub struct MeshLoader;

impl MeshLoader {
    pub fn load(name: &str, path: &str, base_path: &str) -> Result<Resource> {
        const SUPPORTED_FILETYPE_COUNT: usize = 2;
        let supported_filetypes: [SupportedMeshFileTypes; 2] = [
            SupportedMeshFileTypes {
                extensions: ".gsm",
                mesh_file_type: MeshFileType::GSM,
                is_binary: true,
            },
            SupportedMeshFileTypes {
                extensions: ".obj",
                mesh_file_type: MeshFileType::OBJ,
                is_binary: false,
            },
        ];

        let mut file_type = MeshFileType::NotFound;
        let mut resource = Resource::default();
        let mut full_file_path = String::from("");
        let mut f: MaybeUninit<FileHandle> = MaybeUninit::<FileHandle>::uninit();
        for i in 0..SUPPORTED_FILETYPE_COUNT {
            full_file_path = format!(
                "{}/{}/{}{}",
                base_path, path, name, supported_filetypes[i].extensions
            );
            if !FileHandle::exists(&full_file_path) {
                continue;
            }
            f.write(FileHandle::open(
                &full_file_path,
                crate::application::basic::filesystem::FileModes::READ,
                supported_filetypes[i].is_binary,
            )?);
            file_type = supported_filetypes[i].mesh_file_type;
            break;
        }

        if file_type == MeshFileType::NotFound {
            return Err(ResourceSysError::ResourceLoadError {
                name: "MeshFileType Not Found".to_string(),
                file: file!(),
                line: line!(),
            });
        }
        let mut file = unsafe { f.assume_init() };
        let mut geometry_configs = Vec::<GeometryConfig<Vector3D, u32>>::new();

        match file_type {
            MeshFileType::GSM => {
                Self::load_gsm_file(&mut file, &mut geometry_configs)?;
            }
            MeshFileType::OBJ => {
                let gsm_name = format!("{}/{}/{}{}", base_path, path, name, ".gsm");
                Self::import_obj_file(&mut file, &gsm_name, &mut geometry_configs)?;
            }
            _ => {
                return Err(ResourceSysError::ResourceLoadError {
                    name: "MeshFileType Not Found".to_string(),
                    file: file!(),
                    line: line!(),
                });
            }
        }
        resource.data = ResourceData::MeshResourceData(geometry_configs);
        resource.full_path = full_file_path;
        resource.loader_id = INVALID_ID;
        resource.name = name.to_string();
        Ok(resource)
    }
    pub fn unload(res: &mut Resource) -> Result<()> {
        res.loader_id = INVALID_ID;
        Ok(())
    }

    fn import_obj_file(
        file_handle: &mut FileHandle,
        gsm_name: &String,
        geometry_configs: &mut Vec<GeometryConfig<Vector3D, u32>>,
    ) -> Result<()> {
        let mut positions = Vec::<Vec3>::with_capacity(10000);
        let mut normals = Vec::<Vec3>::with_capacity(10000);
        let mut coords = Vec::<Vec2>::with_capacity(10000);
        let mut groups = Vec::<MeshGroupData>::with_capacity(4);

        let mut material_file_name = String::from("");
        let mut name = String::from("");
        let mut current_mat_name_count = 0;
        let mut material_names = vec![String::from(""); 64];

        let lines = file_handle.read_lines()?;

        for line in lines.iter() {
            if line.len() < 1 {
                continue;
            }

            let first_char = line.chars().nth(0).unwrap();
            match first_char {
                '#' => continue,
                'v' => {
                    let second_char = line.chars().nth(1).unwrap();
                    match second_char {
                        ' ' => {
                            let vertices: Vec<&str> = line[1..].trim().split(' ').collect();
                            let pos = Vec3 {
                                data: [
                                    vertices[0].trim().parse().unwrap(),
                                    vertices[1].trim().parse().unwrap(),
                                    vertices[2].trim().parse().unwrap(),
                                ],
                            };
                            positions.push(pos);
                        }
                        'n' => {
                            let vertices: Vec<&str> = line[2..].trim().split(' ').collect();
                            let norm = Vec3 {
                                data: [
                                    vertices[0].trim().parse().unwrap(),
                                    vertices[1].trim().parse().unwrap(),
                                    vertices[2].trim().parse().unwrap(),
                                ],
                            };
                            normals.push(norm);
                        }
                        't' => {
                            let vertices: Vec<&str> = line[2..].trim().split(' ').collect();
                            let coord = Vec2 {
                                data: [vertices[0].parse().unwrap(), vertices[1].parse().unwrap()],
                            };
                            coords.push(coord);
                        }
                        _ => continue,
                    }
                }
                's' => continue,
                'f' => {
                    let mut face = MeshFaceData::default();
                    let indices: Vec<&str> = line[1..].trim().split(' ').collect();
                    let normal_count = normals.len();
                    let coord_count = coords.len();
                    if normal_count == 0 || coord_count == 0 {
                        println!(
                            "normal count {} coord count {} len of indicies {}",
                            normal_count,
                            coord_count,
                            indices.len()
                        );
                        face.vertices[0].position_index = indices[0].parse().unwrap();
                        face.vertices[1].position_index = indices[1].parse().unwrap();
                        face.vertices[2].position_index = indices[2].parse().unwrap();
                    } else {
                        let indices0 = indices[0].trim().split("/").collect::<Vec<_>>();
                        // println!(
                        //     "normal count {} coord count {} len of indices0 {}",
                        //     normal_count,
                        //     coord_count,
                        //     indices0.len()
                        // );
                        face.vertices[0].position_index = indices0[0].trim().parse().unwrap();
                        face.vertices[0].coord_index = indices0[1].trim().parse().unwrap();
                        face.vertices[0].normal_index = indices0[2].trim().parse().unwrap();

                        let indices1 = indices[1].trim().split("/").collect::<Vec<_>>();
                        // println!(
                        //     "normal count {} coord count {} len of indices1 {}",
                        //     normal_count,
                        //     coord_count,
                        //     indices1.len()
                        // );
                        face.vertices[1].position_index = indices1[0].trim().parse().unwrap();
                        face.vertices[1].coord_index = indices1[1].trim().parse().unwrap();
                        face.vertices[1].normal_index = indices1[2].trim().parse().unwrap();

                        let indices2 = indices[2].trim().split("/").collect::<Vec<_>>();
                        // println!(
                        //     "normal count {} coord count {} len of indices2 {}",
                        //     normal_count,
                        //     coord_count,
                        //     indices2.len()
                        // );
                        face.vertices[2].position_index = indices2[0].trim().parse().unwrap();
                        face.vertices[2].coord_index = indices2[1].trim().parse().unwrap();
                        face.vertices[2].normal_index = indices2[2].trim().parse().unwrap();
                    }
                    let group_index = groups.len() - 1;
                    groups[group_index].faces.push(face);
                }
                'm' => {
                    // first should be 'mtllib'
                    material_file_name = line.split(' ').collect::<Vec<_>>()[1].trim().to_string();
                }
                'u' => {
                    groups.push(MeshGroupData { faces: Vec::new() });
                    material_names[current_mat_name_count] =
                        line.split(' ').collect::<Vec<_>>()[1].trim().to_string();
                    current_mat_name_count += 1;
                }
                'g' => {
                    let group_count = groups.len();
                    for i in 0..group_count {
                        let mut geo_config = GeometryConfig::<Vector3D, u32>::default();
                        geo_config.name = name.clone();
                        geo_config.name.push_str(i.to_string().as_str());
                        geo_config.material_name = material_names[i].clone();

                        Self::process_subobject(
                            &positions,
                            &normals,
                            &coords,
                            &groups[i].faces,
                            &mut geo_config,
                        )?;

                        geometry_configs.push(geo_config);
                        groups[i].faces.clear();
                        material_names[i].clear();
                    }
                    current_mat_name_count = 0;
                    groups.clear();
                    name.clear();
                    name = line.split(' ').collect::<Vec<_>>()[1].trim().to_string();
                }
                _ => continue,
            }
        }
        let group_count = groups.len();
        for i in 0..group_count {
            let mut geo_config = GeometryConfig::<Vector3D, u32>::default();
            geo_config.name = name.clone();
            geo_config.name.push_str(i.to_string().as_str());
            geo_config.material_name = material_names[i].clone();

            Self::process_subobject(
                &positions,
                &normals,
                &coords,
                &groups[i].faces,
                &mut geo_config,
            )?;

            geometry_configs.push(geo_config);
            groups[i].faces.clear();
            material_names[i].clear();
        }
        groups.clear();
        normals.clear();
        positions.clear();
        coords.clear();

        if material_file_name.len() > 0 {
            let full_mtl_path = format!("assets/models/{}", material_file_name.trim());
            Self::import_obj_material_library_file(&full_mtl_path)?;
        }

        for i in 0..geometry_configs.len() {
            println!(
                "Geometry de-duplication process starting on geometry object: {} ...",
                &geometry_configs[i].name
            );

            let v = geometry_deduplicate_vertices(&mut geometry_configs[i]);
            geometry_configs[i].vertices = v;
        }
        Self::write_gsm_file(gsm_name, &name, &geometry_configs)?;
        Ok(())
    }

    fn load_gsm_file(
        _file_handle: &mut FileHandle,
        _geometry_configs: &mut Vec<GeometryConfig<Vector3D, u32>>,
    ) -> Result<()> {
        Ok(())
    }

    fn write_gsm_file(
        gsm_name: &String,
        _name: &String,
        _geometry_configs: &Vec<GeometryConfig<Vector3D, u32>>,
    ) -> Result<()> {
        if FileHandle::exists(&gsm_name) {
            println!("WARN: {} already exists", &gsm_name);
            return Ok(());
        }

        Ok(())
    }

    fn write_gmt_file(material_config: &MaterialConfig) -> Result<()> {
        let file_name = format!("assets/materials/{}{}", material_config.name, ".gmt");
        if FileHandle::exists(&file_name) {
            println!("WARN: {} already exists", &file_name);
            return Ok(());
        }
        let mut file_handle = FileHandle::open(&file_name, FileModes::WRITE, true)?;

        println!("writing gmt file: {}", file_name);

        file_handle.write("# material file\n")?;

        file_handle.write("version=0.1\n")?;

        let name = format!("name={}\n", material_config.name);
        file_handle.write(name.as_str())?;

        let diffuse_colour = format!(
            "diffuse_colour={} {} {} {}\n",
            material_config.diffuse_colour.data[0],
            material_config.diffuse_colour.data[1],
            material_config.diffuse_colour.data[2],
            material_config.diffuse_colour.data[3]
        );
        file_handle.write(diffuse_colour.as_str())?;

        let shininess = format!("shininess={}\n", material_config.shininess);
        file_handle.write(shininess.as_str())?;
        if material_config.diffuse_map_name.len() > 0 {
            let texture_name = format!("diffuse_map_name={}\n", material_config.diffuse_map_name);
            file_handle.write(texture_name.as_str())?;
        }

        if material_config.specular_map_name.len() > 0 {
            let texture_name = format!("specular_map_name={}\n", material_config.specular_map_name);
            file_handle.write(texture_name.as_str())?;
        }

        if material_config.normal_map_name.len() > 0 {
            let texture_name = format!("normal_map_name={}\n", material_config.normal_map_name);
            file_handle.write(texture_name.as_str())?;
        }
        let shader = format!("shader={}\n", material_config.shader_name);
        file_handle.write(shader.as_str())?;
        Ok(())
    }

    fn import_obj_material_library_file(file_path: &String) -> Result<()> {
        println!("importing obj .mtl file {} ...", file_path);
        let mut file_handle = FileHandle::open(file_path, FileModes::READ, true)?;

        let mut current_mat_config = MaterialConfig::default();
        let mut hit_name = false;

        let lines = file_handle.read_lines()?;

        for line in lines.iter() {
            let l = line.trim();
            if l.len() < 1 {
                continue;
            }

            let first_char = l.chars().nth(0).unwrap();
            match first_char {
                '#' => continue,
                'K' => {
                    let second_char = l.chars().nth(1).unwrap();
                    match second_char {
                        'a' | 'd' => {
                            let diffuse_colour_values =
                                l[2..].trim().split(' ').collect::<Vec<_>>();
                            current_mat_config.diffuse_colour.data = [
                                diffuse_colour_values[0].parse().unwrap(),
                                diffuse_colour_values[1].parse().unwrap(),
                                diffuse_colour_values[2].parse().unwrap(),
                                1.0,
                            ]
                        }
                        's' => {
                            let specular_values = l[2..].trim().split(' ').collect::<Vec<_>>();
                            let _spec1: f32 = specular_values[0].parse().unwrap();
                            let _spec2: f32 = specular_values[1].parse().unwrap();
                            let _spec3: f32 = specular_values[2].parse().unwrap();
                        }
                        _ => continue,
                    }
                }
                'N' => {
                    let second_char = l.chars().nth(1).unwrap();
                    match second_char {
                        's' => {
                            current_mat_config.shininess = l[2..].trim().parse().unwrap();
                        }
                        _ => continue,
                    }
                }
                'm' => {
                    let kind_and_name = l.split(' ').collect::<Vec<_>>();
                    match kind_and_name[0] {
                        "map_Kd" => {
                            let path = Path::new(kind_and_name[1].split("\\").last().unwrap());
                            if let Some(name) = path.file_stem() {
                                current_mat_config.diffuse_map_name =
                                    name.to_str().unwrap().trim().to_string();
                            }
                        }
                        "map_Ks" => {
                            let path = Path::new(kind_and_name[1].split("\\").last().unwrap());
                            if let Some(name) = path.file_stem() {
                                current_mat_config.specular_map_name =
                                    name.to_str().unwrap().trim().to_string();
                            }
                        }
                        "map_bump" => {
                            let path = Path::new(kind_and_name[1].split("\\").last().unwrap());
                            if let Some(name) = path.file_stem() {
                                current_mat_config.normal_map_name =
                                    name.to_str().unwrap().trim().to_string();
                            }
                        }
                        _ => {}
                    }
                }
                'b' => {
                    let kind_and_name = l.split(' ').collect::<Vec<_>>();
                    let path = Path::new(kind_and_name[1].split("\\").last().unwrap());
                    if let Some(name) = path.file_stem() {
                        current_mat_config.normal_map_name =
                            name.to_str().unwrap().trim().to_string();
                    }
                }
                'n' => {
                    let kind_and_name = l.split(' ').collect::<Vec<_>>();
                    if kind_and_name[0].trim() == "newmtl" {
                        current_mat_config.shader_name = "Shader.Builtin.Material".to_string();
                        if current_mat_config.shininess == 0.0 {
                            current_mat_config.shininess = 8.0;
                        }
                        if hit_name {
                            Self::write_gmt_file(&current_mat_config)?;
                            current_mat_config = MaterialConfig::default();
                        }

                        hit_name = true;
                        current_mat_config.name = kind_and_name[1].trim().to_string();
                    }
                }
                _ => continue,
            }
        }

        current_mat_config.shader_name = "Shader.Builtin.Material".to_string();
        if current_mat_config.shininess == 0.0 {
            current_mat_config.shininess = 8.0;
        }
        Self::write_gmt_file(&current_mat_config)?;

        Ok(())
    }

    fn process_subobject(
        positions: &Vec<Vec3>,
        normals: &Vec<Vec3>,
        coords: &Vec<Vec2>,
        faces: &Vec<MeshFaceData>,
        geometry_config: &mut GeometryConfig<Vector3D, u32>,
    ) -> Result<()> {
        let face_count = faces.len();
        let normal_count = normals.len();
        let coord_count = coords.len();

        let mut extent_set = false;

        let global_skip_normals = normal_count == 0;
        let global_skip_coords = coord_count == 0;

        for f in 0..face_count {
            let face = &faces[f];
            for i in 0..3 {
                let index_data = &face.vertices[i];
                geometry_config.indices.push((i + (f * 3)) as u32);

                let mut vert = Vector3D::default();

                let pos = positions[(index_data.position_index - 1) as usize];
                vert.position = pos;

                if pos.data[0] < geometry_config.min_extents.data[0] || !extent_set {
                    geometry_config.min_extents.data[0] = pos.data[0];
                }
                if pos.data[1] < geometry_config.min_extents.data[1] || !extent_set {
                    geometry_config.min_extents.data[1] = pos.data[1];
                }
                if pos.data[2] < geometry_config.min_extents.data[2] || !extent_set {
                    geometry_config.min_extents.data[2] = pos.data[2];
                }

                if pos.data[0] > geometry_config.max_extents.data[0] || !extent_set {
                    geometry_config.max_extents.data[0] = pos.data[0];
                }
                if pos.data[1] > geometry_config.max_extents.data[1] || !extent_set {
                    geometry_config.max_extents.data[1] = pos.data[1];
                }
                if pos.data[2] > geometry_config.max_extents.data[2] || !extent_set {
                    geometry_config.max_extents.data[2] = pos.data[2];
                }

                extent_set = true;

                if global_skip_normals {
                    vert.normal = Vec3::new_zeroes();
                } else {
                    vert.normal = normals[(index_data.normal_index - 1) as usize];
                }

                if global_skip_coords {
                    vert.coord = Vec2::new_zeroes();
                } else {
                    vert.coord = coords[(index_data.coord_index - 1) as usize];
                }
                vert.colour = Vec4::new_ones();
                geometry_config.vertices.push(vert);
            }
        }

        for i in 0..3 {
            geometry_config.center.data[i] =
                (geometry_config.min_extents.data[i] + geometry_config.max_extents.data[i]) / 2.0;
        }

        geometry_generate_tangents(&mut geometry_config.vertices, &mut geometry_config.indices);
        Ok(())
    }
}
