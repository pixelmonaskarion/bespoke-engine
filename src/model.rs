use std::ops::Range;

use wgpu::{util::DeviceExt, Buffer, IndexFormat, RenderPass};

#[derive(Debug)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub instance_buffer: Option<wgpu::Buffer>,
    pub num_instances: u32,
    pub index_format: IndexFormat,
}

impl Model {
    pub fn new_instances<I: IndexFormatType + bytemuck::Pod>(vertices: Vec<impl ToRaw>, indices: &[I], instances: Vec<impl ToRaw>, device: & dyn DeviceExt) -> Self {
        let [vertex_buffer, index_buffer] = Self::buffers(
            &vertices.iter().map(|vertex| { vertex.to_raw() }).collect::<Vec<Vec<u8>>>().concat(), 
            bytemuck::cast_slice(indices), 
            device);
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &instances.iter().map(|instance| instance.to_raw()).collect::<Vec<_>>().concat(),
                usage: wgpu::BufferUsages::VERTEX,
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
        }
    }

    pub fn new<I: IndexFormatType + bytemuck::Pod>(vertices: Vec<impl ToRaw>, indices: &[I], device: & dyn DeviceExt) -> Self {
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
        }
    }

    pub fn new_vertex_buffer<I: IndexFormatType + bytemuck::Pod>(vertex_buffer: Buffer, num_vertices: u32, instances: Vec<impl ToRaw>, indices: &[I], device: & dyn DeviceExt) -> Self {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: &instances.iter().map(|instance| instance.to_raw()).collect::<Vec<_>>().concat(),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let num_indices = indices.len() as u32;
        let num_instances = instances.len() as u32;
        Model {
            vertex_buffer,
            index_buffer,
            instance_buffer: Some(instance_buffer),
            num_indices,
            num_vertices,
            num_instances,
            index_format: I::get_index_format(),
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
                usage: wgpu::BufferUsages::VERTEX,
            }
        ));
        self.num_instances = instances.len() as u32;
    }
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
    
    fn render_instances<'a: 'b, 'c: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instances: &'c Buffer, range: Range<u32>) {
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
    fn render_instances<'a: 'b, 'c: 'b, 'b>(&'a self, render_pass: &mut RenderPass<'b>, instances: &'c Buffer, range: Range<u32>);
}