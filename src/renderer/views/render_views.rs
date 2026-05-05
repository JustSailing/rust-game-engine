use crate::basic::event::{EventCodes, EventCtx};
use crate::basic::math::consts::INVALID_ID;
use crate::basic::math::{consts::deg_to_rad, matrix4::Matrix4, vec3::Vec3, vec4::Vec4};
use crate::renderer::frontend_renderer::{Renderer, RendererError};
use crate::renderer::renderer_types::{
    GeometryRenderData, PacketData, RenderView, RenderViewKnownType, RenderViewPacket,
    RendererDebugViewMode, RenderpassHandle,
};
use crate::resources::resource_types::TextureFlags;
use crate::systems::camera_system::{CameraHandle, CameraSystem, DEFAULT_CAMERA_NAME};
use crate::systems::{
    geometry_system::GeometrySystem, material_system::MaterialSystem, shader_system::ShaderSystem,
    texture_system::TextureSystem,
};

use std::{cell::RefCell, rc::Rc};

#[repr(C)]
#[derive(Clone, Copy)]
struct GeometryDistance {
    pub grd: GeometryRenderData,
    pub distance: f32,
}

#[repr(C)]
pub struct UIInternalData {
    pub shader_id: usize,
    pub camera: CameraHandle,
    pub near_clip: f32,
    pub far_clip: f32,
    pub projection_matrix: Matrix4,
    pub view_matrix: Matrix4,
}

impl Default for UIInternalData {
    fn default() -> Self {
        Self {
            shader_id: INVALID_ID,
            camera: INVALID_ID,
            near_clip: Default::default(),
            far_clip: Default::default(),
            projection_matrix: Matrix4::identity(),
            view_matrix: Matrix4::identity(),
        }
    }
}

#[repr(C)]
pub struct UIRenderView {
    pub id: usize,
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub known_type: RenderViewKnownType,
    pub passes: Vec<RenderpassHandle>,
    pub custom_shader_name: String,
    pub internal_data: UIInternalData,
}

impl Default for UIRenderView {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            name: Default::default(),
            width: 0,
            height: 0,
            known_type: RenderViewKnownType::UI,
            passes: Vec::new(),
            custom_shader_name: Default::default(),
            internal_data: Default::default(),
        }
    }
}

impl UIRenderView {
    pub fn id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn known_type(mut self, known_type: RenderViewKnownType) -> Self {
        self.known_type = known_type;
        self
    }

    pub fn passes(mut self, passes: &Vec<RenderpassHandle>) -> Self {
        self.passes = passes.clone();
        self
    }

    pub fn custom_shader_name(mut self, custom_shader_name: &str) -> Self {
        self.custom_shader_name = custom_shader_name.to_string();
        self
    }

    pub fn internal_data(mut self, internal_data: UIInternalData) -> Self {
        self.internal_data = internal_data;
        self
    }
}

impl RenderView for UIRenderView {
    fn create(
        &mut self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        _camera_system: &Rc<RefCell<CameraSystem>>,
    ) -> Result<(), RendererError> {
        let internal_data = UIInternalData {
            shader_id: shader_system.borrow().get_shader_id("Shader.Builtin.UI")?,
            camera: INVALID_ID,
            near_clip: -100.0,
            far_clip: 100.0,
            projection_matrix: Matrix4::orthographic(0.0, 1280.0, 720.0, 0.0, -100.0, 100.0),
            view_matrix: Matrix4::identity(),
        };
        self.internal_data = internal_data;
        Ok(())
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn handle_event(
        &mut self,
        _code: usize,
        _sender: *const std::os::raw::c_void,
        _listener: *const std::os::raw::c_void,
        _data: &EventCtx,
    ) -> bool {
        // NOTE: UIRenderView doesn't use this yet. Only used for now in the world render view
        // to switch from normal view, ligting view, default view
        true
    }
    fn resize(
        &mut self,
        width: u16,
        height: u16,
        renderer_system: &Rc<RefCell<Renderer>>,
    ) -> Result<(), RendererError> {
        self.width = width;
        self.height = height;
        self.internal_data.projection_matrix = Matrix4::orthographic(
            0.0,
            self.width as f32,
            self.height as f32,
            0.0,
            self.internal_data.near_clip,
            self.internal_data.far_clip,
        );
        for handle in self.passes.iter_mut() {
            let mut renderer = renderer_system.borrow_mut();
            let renderpass = renderer.get_mut_renderpass_by_id(*handle)?;
            renderpass.render_area.set_x(0.0);
            renderpass.render_area.set_y(0.0);
            renderpass.render_area.set_w(width as f32);
            renderpass.render_area.set_h(height as f32);
        }
        Ok(())
    }
    fn build_packet(
        &self,
        mesh_packet: &mut PacketData,
        _camera_system: &Rc<RefCell<CameraSystem>>,
        _geometry_system: &Rc<RefCell<GeometrySystem>>,
        _material_system: &Rc<RefCell<MaterialSystem>>,
        _texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<RenderViewPacket, RendererError> {
        let mut packet = RenderViewPacket {
            view_matrix: self.internal_data.view_matrix,
            projection_matrix: self.internal_data.projection_matrix,
            view_position: Vec3::new_zeroes(),
            ambient_colour: Vec4::new_zeroes(),
            custom_shader_name: self.custom_shader_name.clone(),
            data: Vec::<GeometryRenderData>::new(),
            extended: None,
        };
        let m_packet = match mesh_packet {
            PacketData::Mesh(m) => m,
            _ => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "MeshPacketData",
                    given: "SomethingElse",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        for mesh in m_packet.meshes.iter_mut() {
            for g in mesh.geometries.iter_mut() {
                let render_data = GeometryRenderData {
                    model: mesh.transform.borrow_mut().get_world(),
                    geometry_handle: *g,
                };
                packet.data.push(render_data);
            }
        }
        Ok(packet)
    }
    fn render(
        &self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        material_system: &Rc<RefCell<MaterialSystem>>,
        renderer_system: &Rc<RefCell<Renderer>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        frame_number: u64,
        render_view_packet: &mut RenderViewPacket,
    ) -> Result<(), RendererError> {
        for pass in self.passes.iter() {
            renderer_system.borrow_mut().begin_renderpass_by_id(*pass)?;
            shader_system
                .borrow_mut()
                .use_by_id(self.internal_data.shader_id)?;
            // NOTE: this might be an issue since binding and updating the same descriptor
            // within the same command buffer invalidates teh descriptor
            material_system.borrow().apply_global(
                self.internal_data.shader_id as u32,
                &self.internal_data.projection_matrix,
                &self.internal_data.view_matrix,
                &Vec3::new(1.0, 1.0, 1.0),
                &Vec4::new(1.0, 1.0, 1.0, 1.0),
                0,
            )?;
            // NOTE: here just incase the issue with the descriptor becomes invalidated
            // let mut global_updated = false;

            for g in render_view_packet.data.iter_mut() {
                let geo_sys = geometry_system.borrow();
                let geo = geo_sys.get_geometry(g.geometry_handle)?;
                let mut material_sys = material_system.borrow_mut();
                let needs_update = material_sys
                    .get_material(geo.material_handle)?
                    .render_frame_number
                    != frame_number;
                if needs_update {
                    let material_ref = material_sys.get_material(geo.material_handle)?;

                    material_sys.apply_instance(
                        &material_ref,
                        geo.material_instance_id,
                        &g.model,
                    )?;

                     material_sys
                        .get_mut_material(g.geometry_handle)?
                        .render_frame_number = frame_number;
                }
                g.geometry_handle = geo.internal_id;
                renderer_system.borrow_mut().draw_geometry(g, 0.0)?;
            }
            renderer_system.borrow_mut().end_renderpass()?;
        }
        Ok(())
    }
}

#[repr(C)]
pub struct WorldInternalData {
    pub shader_id: usize,
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub projection_matrix: Matrix4,
    pub world_camera: CameraHandle,
    pub ambient_colour: Vec4,
    pub render_mode: u32,
}

impl Default for WorldInternalData {
    fn default() -> Self {
        Self {
            shader_id: INVALID_ID,
            fov: Default::default(),
            near_clip: Default::default(),
            far_clip: Default::default(),
            projection_matrix: Matrix4::identity(),
            world_camera: INVALID_ID,
            ambient_colour: Vec4::new_ones(),
            render_mode: 0,
        }
    }
}

#[repr(C)]
pub struct WorldRenderView {
    pub id: usize,
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub known_type: RenderViewKnownType,
    pub passes: Vec<RenderpassHandle>,
    pub custom_shader_name: String,
    pub internal_data: WorldInternalData,
}

impl Default for WorldRenderView {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            name: Default::default(),
            width: Default::default(),
            height: Default::default(),
            known_type: RenderViewKnownType::World,
            passes: Vec::new(),
            custom_shader_name: Default::default(),
            internal_data: Default::default(),
        }
    }
}

impl WorldRenderView {
    pub fn id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn known_type(mut self, known_type: RenderViewKnownType) -> Self {
        self.known_type = known_type;
        self
    }

    pub fn passes(mut self, passes: &Vec<RenderpassHandle>) -> Self {
        self.passes = passes.clone();
        self
    }

    pub fn custom_shader_name(mut self, custom_shader_name: &str) -> Self {
        self.custom_shader_name = custom_shader_name.to_string();
        self
    }

    pub fn internal_data(mut self, internal_data: WorldInternalData) -> Self {
        self.internal_data = internal_data;
        self
    }
}

impl RenderView for WorldRenderView {
    fn create(
        &mut self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        camera_system: &Rc<RefCell<CameraSystem>>,
    ) -> Result<(), RendererError> {
        let internal_data = WorldInternalData {
            fov: deg_to_rad(45.0),
            shader_id: shader_system
                .borrow()
                .get_shader_id("Shader.Builtin.Material")?,
            near_clip: 0.1,
            far_clip: 1000.0,
            projection_matrix: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
            ambient_colour: Vec4::new(0.25, 0.25, 0.25, 1.0),
            render_mode: RendererDebugViewMode::Default as u32,
            world_camera: camera_system
                .borrow_mut()
                .acquire(DEFAULT_CAMERA_NAME, true)?,
        };
        self.internal_data = internal_data;
        Ok(())
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn handle_event(
        &mut self,
        code: usize,
        _sender: *const std::os::raw::c_void,
        _listener: *const std::os::raw::c_void,
        data: &EventCtx,
    ) -> bool {
        let data = match EventCodes::from(code) {
            EventCodes::SetRenderMode => match data {
                EventCtx::U32(d) => d,
                _ => todo!(),
            },
            _ => todo!(),
        };
        if data[0] == RendererDebugViewMode::Default as u32 {
            self.internal_data.render_mode = data[0]
        } else if data[0] == RendererDebugViewMode::Lighting as u32 {
            self.internal_data.render_mode = data[0]
        } else if data[0] == RendererDebugViewMode::Normals as u32 {
            self.internal_data.render_mode = data[0]
        }
        true
    }

    fn resize(
        &mut self,
        width: u16,
        height: u16,
        renderer_system: &Rc<RefCell<Renderer>>,
    ) -> Result<(), RendererError> {
        self.width = width;
        self.height = height;
        self.internal_data.projection_matrix = Matrix4::perspective(
            self.internal_data.fov,
            self.width as f32 / self.height as f32,
            self.internal_data.near_clip,
            self.internal_data.far_clip,
        );
        for handle in self.passes.iter_mut() {
            let mut renderer = renderer_system.borrow_mut();
            let renderpass = renderer.get_mut_renderpass_by_id(*handle)?;
            renderpass.render_area.set_x(0.0);
            renderpass.render_area.set_y(0.0);
            renderpass.render_area.set_w(width as f32);
            renderpass.render_area.set_h(height as f32);
        }

        Ok(())
    }
    fn build_packet(
        &self,
        mesh_packet: &mut PacketData,
        camera_system: &Rc<RefCell<CameraSystem>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        material_system: &Rc<RefCell<MaterialSystem>>,
        texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<RenderViewPacket, RendererError> {
        let mut packet = RenderViewPacket {
            view_matrix: *camera_system
                .borrow()
                .get_camera(self.internal_data.world_camera)
                .get_view(),
            projection_matrix: self.internal_data.projection_matrix,
            view_position: *camera_system
                .borrow()
                .get_camera(self.internal_data.world_camera)
                .get_position(),
            ambient_colour: Vec4::new_zeroes(),
            custom_shader_name: self.custom_shader_name.clone(),
            data: Vec::<GeometryRenderData>::new(),
            extended: None,
        };

        let m_packet = match mesh_packet {
            PacketData::Mesh(m) => m,
            _ => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "MeshPacketData",
                    given: "SomethingElse",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let mut geometry_distances = Vec::<GeometryDistance>::new();
        for mesh in m_packet.meshes.iter_mut() {
            for g in mesh.geometries.iter_mut() {
                if *g == INVALID_ID {
                    println!("geometry handle is INVALID");
                    continue;
                }
                let geo_sys = geometry_system.borrow();
                let geo = geo_sys.get_geometry(*g)?;
                let render_data = GeometryRenderData {
                    model: mesh.transform.borrow_mut().get_world(),
                    geometry_handle: *g,
                };
                let mat_sys = material_system.borrow();
                let mat = mat_sys.get_material(geo.material_handle)?;
                let tex_sys = texture_system.borrow();
                let transparency = tex_sys
                    .get_texture(mat.diffuse_map.texture_handle)?
                    .flags
                    .contains(TextureFlags::Transparency);
                if !transparency {
                    packet.data.push(render_data);
                } else {
                    let center = geo.center.transform(&render_data.model);
                    let distance = center
                        .distance(camera_system.borrow().get_default_camera().get_position())
                        .abs();
                    let geo_dist = GeometryDistance {
                        grd: render_data,
                        distance: distance,
                    };
                    geometry_distances.push(geo_dist);
                }
            }
        }
        quicksort(&mut geometry_distances, false);
        for i in 0..geometry_distances.len() {
            packet.data.push(geometry_distances[i].grd);
        }

        Ok(packet)
    }
    fn render(
        &self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        material_system: &Rc<RefCell<MaterialSystem>>,
        renderer_system: &Rc<RefCell<Renderer>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        frame_number: u64,
        render_view_packet: &mut RenderViewPacket,
    ) -> Result<(), RendererError> {
        for pass in self.passes.iter() {
            renderer_system.borrow_mut().begin_renderpass_by_id(*pass)?;
            shader_system
                .borrow_mut()
                .use_by_id(self.internal_data.shader_id)?;
            // NOTE: this might be an issue since binding and updating the same descriptor
            // within the same command buffer invalidates teh descriptor
            material_system.borrow().apply_global(
                self.internal_data.shader_id as u32,
                &render_view_packet.projection_matrix,
                &render_view_packet.view_matrix,
                &render_view_packet.view_position,
                &render_view_packet.ambient_colour,
                self.internal_data.render_mode,
            )?;

            // NOTE: here just incase the issue with the descriptor becomes invalidated
            // let mut global_updated = false;

            for g in render_view_packet.data.iter_mut() {
                let geo_sys = geometry_system.borrow();
                if g.geometry_handle == INVALID_ID {
                    println!("Geomtery Render Data: geometry handle is invlaid");
                    continue;
                }
                let geo = geo_sys.get_geometry(g.geometry_handle)?;
                let mut material_sys = material_system.borrow_mut();
                if geo.material_handle == INVALID_ID {
                    println!("geometry: material handle is invlaid");
                    continue;
                }
                let needs_update = material_sys
                    .get_material(geo.material_handle)?
                    .render_frame_number
                    != frame_number;
                if needs_update {
                    let material_ref = material_sys.get_material(geo.material_handle)?;

                    material_sys.apply_instance(
                        &material_ref,
                        geo.material_instance_id,
                        &g.model,
                    )?;
                } else {
                    material_sys
                        .get_mut_material(g.geometry_handle)?
                        .render_frame_number = frame_number;
                }
                g.geometry_handle = geo.internal_id;
                renderer_system.borrow_mut().draw_geometry(g, 0.0)?;
            }
            renderer_system.borrow_mut().end_renderpass()?;
        }
        Ok(())
    }
}

#[repr(C)]
pub struct SkyboxInternalData {
    pub shader_id: usize,
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub projection_matrix: Matrix4,
    pub world_camera: CameraHandle,
    pub projection_location: u16,
    pub view_location: u16,
    pub cube_map_location: u16,
}

impl Default for SkyboxInternalData {
    fn default() -> Self {
        Self {
            shader_id: INVALID_ID,
            fov: Default::default(),
            near_clip: Default::default(),
            far_clip: Default::default(),
            projection_matrix: Matrix4::identity(),
            world_camera: INVALID_ID,
            projection_location: INVALID_ID as u16,
            view_location: INVALID_ID as u16,
            cube_map_location: INVALID_ID as u16,
        }
    }
}

#[repr(C)]
pub struct SkyboxRenderView {
    pub id: usize,
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub known_type: RenderViewKnownType,
    pub passes: Vec<RenderpassHandle>,
    pub custom_shader_name: String,
    pub internal_data: SkyboxInternalData,
}

impl Default for SkyboxRenderView {
    fn default() -> Self {
        Self {
            id: INVALID_ID,
            name: Default::default(),
            width: Default::default(),
            height: Default::default(),
            known_type: RenderViewKnownType::Skybox,
            passes: Vec::new(),
            custom_shader_name: Default::default(),
            internal_data: Default::default(),
        }
    }
}

impl SkyboxRenderView {
    pub fn id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn known_type(mut self, known_type: RenderViewKnownType) -> Self {
        self.known_type = known_type;
        self
    }

    pub fn passes(mut self, passes: &Vec<RenderpassHandle>) -> Self {
        self.passes = passes.clone();
        self
    }

    pub fn custom_shader_name(mut self, custom_shader_name: &str) -> Self {
        self.custom_shader_name = custom_shader_name.to_string();
        self
    }

    pub fn internal_data(mut self, internal_data: SkyboxInternalData) -> Self {
        self.internal_data = internal_data;
        self
    }
}

impl RenderView for SkyboxRenderView {
    fn create(
        &mut self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        camera_system: &Rc<RefCell<CameraSystem>>,
    ) -> Result<(), RendererError> {
        let shader_id = shader_system
            .borrow()
            .get_shader_id("Shader.Builtin.Skybox")?;
        let projection_location = shader_system
            .borrow()
            .get_uniform_index_by_shader_id(shader_id, "projection")?;
        let view_location = shader_system
            .borrow()
            .get_uniform_index_by_shader_id(shader_id, "view")?;
        let cube_map_location = shader_system
            .borrow()
            .get_uniform_index_by_shader_id(shader_id, "cube_texture")?;
        let internal_data = SkyboxInternalData {
            fov: deg_to_rad(45.0),
            shader_id: shader_id,
            near_clip: 0.1,
            far_clip: 1000.0,
            projection_matrix: Matrix4::perspective(deg_to_rad(45.0), 1280.0 / 720.0, 0.1, 1000.0),
            world_camera: camera_system
                .borrow_mut()
                .acquire(DEFAULT_CAMERA_NAME, true)?,
            projection_location: projection_location,
            view_location: view_location,
            cube_map_location: cube_map_location,
        };
        self.internal_data = internal_data;
        Ok(())
    }

    fn get_id(&self) -> usize {
        self.id
    }

    fn handle_event(
        &mut self,
        _code: usize,
        _sender: *const std::os::raw::c_void,
        _listener: *const std::os::raw::c_void,
        _data: &EventCtx,
    ) -> bool {
        // NOTE: Skybox doesn't handle any events at this time
        true
    }

    fn resize(
        &mut self,
        width: u16,
        height: u16,
        renderer_system: &Rc<RefCell<Renderer>>,
    ) -> Result<(), RendererError> {
        self.width = width;
        self.height = height;
        self.internal_data.projection_matrix = Matrix4::perspective(
            self.internal_data.fov,
            self.width as f32 / self.height as f32,
            self.internal_data.near_clip,
            self.internal_data.far_clip,
        );
        for handle in self.passes.iter_mut() {
            let mut renderer = renderer_system.borrow_mut();
            let renderpass = renderer.get_mut_renderpass_by_id(*handle)?;
            renderpass.render_area.set_x(0.0);
            renderpass.render_area.set_y(0.0);
            renderpass.render_area.set_w(width as f32);
            renderpass.render_area.set_h(height as f32);
        }

        Ok(())
    }
    fn build_packet(
        &self,
        data_packet: &mut PacketData,
        camera_system: &Rc<RefCell<CameraSystem>>,
        _geometry_system: &Rc<RefCell<GeometrySystem>>,
        _material_system: &Rc<RefCell<MaterialSystem>>,
        _texture_system: &Rc<RefCell<TextureSystem>>,
    ) -> Result<RenderViewPacket, RendererError> {
        let s_packet = match data_packet {
            PacketData::Skybox(s) => s,
            _ => {
                return Err(RendererError::WrongResourceDataType {
                    expected: "MeshPacketData",
                    given: "SomethingElse",
                    file: file!(),
                    line: line!(),
                });
            }
        };
        let packet = RenderViewPacket {
            view_matrix: *camera_system
                .borrow()
                .get_camera(self.internal_data.world_camera)
                .get_view(),
            projection_matrix: self.internal_data.projection_matrix,
            view_position: *camera_system
                .borrow()
                .get_camera(self.internal_data.world_camera)
                .get_position(),
            ambient_colour: Vec4::new_zeroes(),
            custom_shader_name: self.custom_shader_name.clone(),
            data: Vec::new(),
            extended: Some(s_packet.skybox.clone()),
        };

        Ok(packet)
    }
    fn render(
        &self,
        shader_system: &Rc<RefCell<ShaderSystem>>,
        _material_system: &Rc<RefCell<MaterialSystem>>,
        renderer_system: &Rc<RefCell<Renderer>>,
        geometry_system: &Rc<RefCell<GeometrySystem>>,
        _frame_number: u64,
        render_view_packet: &mut RenderViewPacket,
    ) -> Result<(), RendererError> {
        for pass in self.passes.iter() {
            renderer_system.borrow_mut().begin_renderpass_by_id(*pass)?;
            shader_system
                .borrow_mut()
                .use_by_id(self.internal_data.shader_id)?;
            // NOTE: this might be an issue since binding and updating the same descriptor
            // within the same command buffer invalidates teh descriptor

            render_view_packet.view_matrix.data[12] = 0.0;
            render_view_packet.view_matrix.data[13] = 0.0;
            render_view_packet.view_matrix.data[14] = 0.0;
            renderer_system.borrow_mut().bind_globals_for_shader(
                shader_system
                    .borrow_mut()
                    .get_mut_shader_by_id(self.internal_data.shader_id)?,
            )?;
            shader_system.borrow_mut().set_uniform_by_index(
                self.internal_data.projection_location,
                &render_view_packet.projection_matrix as *const Matrix4 as _,
            )?;
            shader_system.borrow_mut().set_uniform_by_index(
                self.internal_data.view_location,
                &render_view_packet.view_matrix as *const Matrix4 as _,
            )?;
            shader_system.borrow_mut().apply_globals()?;
            shader_system
                .borrow_mut()
                .bind_instance(render_view_packet.extended.as_ref().unwrap().instance_id)?;
            shader_system.borrow_mut().apply_instance()?;
            let geo_sys = geometry_system.borrow();
            if let Some(ref skybox) = render_view_packet.extended {
                let geo = geo_sys.get_geometry(skybox.geometry_handle)?;
                let mut geo_data = GeometryRenderData {
                    model: Matrix4::identity(),
                    geometry_handle: geo.internal_id,
                };
                renderer_system
                    .borrow_mut()
                    .draw_geometry(&mut geo_data, 1.0)?;
            }
        }
        renderer_system.borrow_mut().end_renderpass()?;

        Ok(())
    }
}

fn quicksort(arr: &mut [GeometryDistance], ascending: bool) {
    if arr.len() <= 1 {
        return;
    }

    let pivot_index = partition(arr, ascending);

    let (left, right) = arr.split_at_mut(pivot_index);
    quicksort(left, ascending);
    quicksort(&mut right[1..], ascending);
}

fn partition(arr: &mut [GeometryDistance], ascending: bool) -> usize {
    let pivot_index = arr.len() / 2;
    arr.swap(pivot_index, arr.len() - 1); // Move pivot to the end

    let mut i = 0;
    for j in 0..arr.len() - 1 {
        if ascending {
            if arr[j].distance <= arr[arr.len() - 1].distance {
                arr.swap(i, j);
                i += 1;
            }
        } else {
            if arr[j].distance >= arr[arr.len() - 1].distance {
                arr.swap(i, j);
                i += 1;
            }
        }
    }

    arr.swap(i, arr.len() - 1);
    i
}
