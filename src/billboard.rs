use crate::{binding::Descriptor, culling::culled, instance::Instance, model::{calculate_bounding_box, Model, Render, ToRaw}, VertexTrait};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{Matrix4, Quaternion, Vector3};
use wgpu::Device;

pub struct Billboard {
    model: Model,
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
}

impl Billboard {
    pub fn new(width: f32, height: f32, size: f32, position: Vector3<f32>, rotation: Quaternion<f32>, device: &Device) -> Self {
        let vertices = vec![
            Vertex { position: [size*-width/2.0, size*-height/2.0, 0.0], tex_pos: [0.0, 1.0], normal: [0.0, 0.0, 0.0] },
            Vertex { position: [size*-width/2.0, size*height/2.0, 0.0], tex_pos: [0.0, 0.0], normal: [0.0, 0.0, 0.0] },
            Vertex { position: [size*width/2.0, size*-height/2.0, 0.0], tex_pos: [1.0, 1.0], normal: [0.0, 0.0, 0.0] },
            Vertex { position: [size*width/2.0, size*height/2.0, 0.0], tex_pos: [1.0, 0.0], normal: [0.0, 0.0, 0.0] },
        ];
        let bounding_box = calculate_bounding_box(&vertices);
        let model = Model::new_instances(vertices, &[0_u16, 1, 2, 2, 1, 3], vec![Instance {position, rotation}], bounding_box, device);
        Self {
            model,
            position,
            rotation,
        }
    }

    pub fn set_position(&mut self, position: Vector3<f32>, device: &Device) {
        self.position = position;
        self.create_instance(device);
    }

    pub fn set_rotation(&mut self, rotation: Quaternion<f32>, device: &Device) {
        self.rotation = rotation;
        self.create_instance(device);
    }

    pub fn set_both(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>, device: &Device) {
        self.position = position;
        self.rotation = rotation;
        self.create_instance(device);
    }

    fn create_instance(&mut self, device: &Device) {
        self.model.update_instances(vec![Instance {position: self.position, rotation: self.rotation}], device);
    }
}

impl Render for Billboard {
    fn render<'a: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>) {
        self.model.render(render_pass);
    }
    fn render_instances<'a: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>, instances: &wgpu::Buffer, range: std::ops::Range<u32>) {
        self.model.render_instances(render_pass, instances, range);
    }
    fn render_culled_transformed<'a: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>, instance_transform: Option<cgmath::Matrix4<f32>>, camera: &crate::camera::Camera) {
        if !culled(&self.model, instance_transform.unwrap_or(Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)), camera) {
            self.render(render_pass);
        }
    }

    fn render_culled<'a: 'b, 'b>(&'a self, camera: &crate::binding::UniformBinding<crate::camera::Camera>, render_pass: &mut wgpu::RenderPass<'b>, culling: &mut crate::culling::CullingCompute, surface_ctx: &dyn crate::surface_context::SurfaceCtx) {
        todo!()
    }
}

#[repr(C)]
#[derive(NoUninit, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_pos: [f32; 2],
    pub normal: [f32; 3],
}

impl VertexTrait for Vertex {
    fn pos(&self) -> Vector3<f32> {
        return Vector3::new(self.position[0], self.position[1], self.position[2]);
    }
}

impl Descriptor for Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl ToRaw for Vertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}