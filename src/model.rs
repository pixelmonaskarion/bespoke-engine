use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl Model {
    pub fn new(vertices: Vec<impl ToRaw>, indices: &[u16], device: & dyn DeviceExt) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: &vertices.iter().map(|vertex| { vertex.to_raw().to_vec() }).collect::<Vec<Vec<u8>>>().concat(),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = indices.len() as u32;
        let num_vertices = vertices.len() as u32;
        Model {
            vertex_buffer,
            index_buffer,
            num_indices,
            num_vertices
        }
    }
}

pub trait ToRaw {
    fn to_raw(&self) -> &[u8];
}