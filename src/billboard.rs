use crate::{binding::Descriptor, model::{Model, Render, ToRaw}, instance::Instance};
use bytemuck::{bytes_of, NoUninit};
use cgmath::{Quaternion, Vector3};
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
        let model = Model::new_instances(vertices, &[0_u16, 1, 2, 2, 1, 3], vec![Instance {position, rotation}], device);
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
    fn render_instances<'a: 'b, 'c: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>, instances: &'c wgpu::Buffer, range: std::ops::Range<u32>) {
        self.model.render_instances(render_pass, instances, range);
    }
}

#[repr(C)]
#[derive(NoUninit, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_pos: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex {
    #[allow(dead_code)]
    pub fn pos(&self) -> Vector3<f32> {
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