use std::ops::Range;

use cgmath::{Matrix4, SquareMatrix};
use wgpu::{util::DeviceExt, Buffer, IndexFormat, RenderPass};

use crate::{binding::UniformBinding, camera::Camera, culling::{culled, CullingCompute, AABB}, surface_context::SurfaceCtx, VertexTrait};

#[derive(Debug)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub instance_buffer: Option<wgpu::Buffer>,
    pub num_instances: u32,
    pub index_format: IndexFormat,
    pub bounding_box: AABB,
}

impl Model {
    pub fn new_instances<I: IndexFormatType + bytemuck::Pod>(vertices: Vec<impl ToRaw>, indices: &[I], instances: Vec<impl ToRaw>, bounding_box: AABB, device: & dyn DeviceExt) -> Self {
        let [vertex_buffer, index_buffer] = Self::buffers(
            &vertices.iter().map(|vertex| { vertex.to_raw() }).collect::<Vec<Vec<u8>>>().concat(), 
            bytemuck::cast_slice(indices), 
            device);
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &instances.iter().map(|instance| instance.to_raw()).collect::<Vec<_>>().concat(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            }
        );
        let num_indices = indices.len() as u32;
        let num_vertices = vertices.len() as u32;
        let num_instances = instances.len() as u32;
        Model {
            vertex_buffer,
            index_buffer,
            instance_buffer: Some(instance_buffer),
            num_indices,
            num_vertices,
            num_instances,
            index_format: I::get_index_format(),
            bounding_box,
        }
    }

    pub fn new<I: IndexFormatType + bytemuck::Pod>(vertices: Vec<impl ToRaw>, indices: &[I], bounding_box: AABB, device: & dyn DeviceExt) -> Self {
        let [vertex_buffer, index_buffer] = Self::buffers(
            &vertices.iter().map(|vertex| { vertex.to_raw() }).collect::<Vec<Vec<u8>>>().concat(), 
            bytemuck::cast_slice(indices), 
            device);
        let num_indices = indices.len() as u32;
        let num_vertices = vertices.len() as u32;
        Model {
            vertex_buffer,
            index_buffer,
            instance_buffer: None,
            num_indices,
            num_vertices,
            num_instances: 1,
            index_format: I::get_index_format(),
            bounding_box,
        }
    }

    pub fn new_buffers(vertex_buffer: Buffer, num_vertices: u32, instances: Vec<impl ToRaw>, index_buffer: Buffer, num_indices: u32, index_format: IndexFormat, bounding_box: AABB, device: & dyn DeviceExt) -> Self {
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &instances.iter().map(|instance| instance.to_raw()).collect::<Vec<_>>().concat(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            }
        );
        let num_instances = instances.len() as u32;
        Model {
            vertex_buffer,
            index_buffer,
            instance_buffer: Some(instance_buffer),
            num_indices,
            num_vertices,
            num_instances,
            index_format,
            bounding_box,
        }
    }

    fn buffers(vertices: &[u8], indices: &[u8], device: &dyn DeviceExt) -> [Buffer; 2] {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: indices,
            usage: wgpu::BufferUsages::INDEX,
        });
        [vertex_buffer, index_buffer]
    }

    pub fn update_instances(&mut self, instances: Vec<impl ToRaw>, device: & dyn DeviceExt) {
        self.instance_buffer = Some(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &instances.iter().map(|instance| instance.to_raw()).collect::<Vec<_>>().concat(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            }
        ));
        self.num_instances = instances.len() as u32;
    }
}

pub fn calculate_bounding_box(vertices: &Vec<impl VertexTrait>) -> AABB {
    let mut bounding_box = AABB {
        dimensions: [0.0; 3],
    };
    for vertex in vertices {
        let pos = vertex.pos();
        if pos.x.abs() > bounding_box.dimensions[0] {
            bounding_box.dimensions[0] = pos.x.abs();
        }
        if pos.y.abs() > bounding_box.dimensions[1] {
            bounding_box.dimensions[1] = pos.y.abs();
        }
        if pos.z.abs() > bounding_box.dimensions[2] {
            bounding_box.dimensions[2] = pos.z.abs();
        }
    }
    bounding_box
}

pub trait IndexFormatType {
    fn get_index_format() -> IndexFormat;
}

impl IndexFormatType for u32 {
    fn get_index_format() -> IndexFormat {
        IndexFormat::Uint32
    }
}

impl IndexFormatType for u16 {
    fn get_index_format() -> IndexFormat {
        IndexFormat::Uint16
    }
}

impl Render for Model {
    fn render<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        if let Some(instance_buffer) = &self.instance_buffer {
            render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        }
        render_pass.set_index_buffer(self.index_buffer.slice(..), self.index_format);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.num_instances);
    }
    
    fn render_culled_transformed<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instance_transform: Option<Matrix4<f32>>, camera: &Camera) {
        if !culled(self, instance_transform.unwrap_or(Matrix4::identity()), camera) {
            self.render(render_pass);
        }
    }

    fn render_culled<'a: 'b, 'b>(&'a self, camera: &UniformBinding<Camera>, render_pass: &mut RenderPass<'b>, culling: &mut CullingCompute, surface_ctx: &dyn SurfaceCtx) {
        if let Some(instance_buffer) = &self.instance_buffer {
            let (culled_instances, num_instances) = culling.run(&instance_buffer, self.num_instances, &self.bounding_box, camera, surface_ctx.device(), surface_ctx.queue());
            self.render_instances(render_pass, &culled_instances, 0..num_instances);
        } else {
            self.render_culled_transformed(render_pass, None, &camera.value);
        }
    }
    
    fn render_instances<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instances: &Buffer, range: Range<u32>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instances.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), self.index_format);
        render_pass.draw_indexed(0..self.num_indices, 0, range);
    }
}

pub trait ToRaw {
    fn to_raw(&self) -> Vec<u8>;
}

pub trait Render {
    fn render<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>);
    fn render_instances<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instances: &Buffer, range: Range<u32>);
    fn render_culled_transformed<'a: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instance_transform: Option<Matrix4<f32>>, camera: &Camera);
    fn render_culled<'a: 'b, 'b>(&'a self, camera: &UniformBinding<Camera>, render_pass: &mut RenderPass<'b>, culling: &mut CullingCompute, surface_ctx: &dyn SurfaceCtx);
}