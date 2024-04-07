use std::{io::{BufReader, Cursor}, ops::Range, path::Path};

use bytemuck::bytes_of;
use wgpu::{util::DeviceExt, Buffer};

use crate::{binding::Descriptor, model::{Render, ToRaw}, texture::Texture};

pub struct Material {
    pub name: String,
    pub diffuse_texture: crate::texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: Option<String>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct MeshModel {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Render for MeshModel {
    fn render<'a: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>) {
        for (i, material) in self.materials.iter().enumerate() {
            render_pass.set_bind_group(i as u32, &material.bind_group, &[]);
        }
        for mesh in &self.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
        }
    }

    fn render_instances<'a: 'b, 'c: 'b, 'b>(&'a self, render_pass: &mut wgpu::RenderPass<'b>, instances: &'c Buffer, range: Range<u32>) {
        for (i, material) in self.materials.iter().enumerate() {
            render_pass.set_bind_group(i as u32, &material.bind_group, &[]);
        }
        for mesh in &self.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..mesh.num_elements, 0, range.clone());
        }
    }
}

impl MeshModel {
    pub fn load_model(
        name: Option<String>,
        source_path: &Path,
        load_resource_string: impl Fn(&str) -> Option<String>,
        load_resource: impl Fn(&str) -> Option<&&[u8]>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<MeshModel> {
        let path_string = source_path.as_os_str().to_str().unwrap();
        let obj_cursor = Cursor::new(load_resource_string(path_string).unwrap());
        let mut obj_reader = BufReader::new(obj_cursor);
        
        let (models, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(load_resource(source_path.parent().unwrap().join(p).as_os_str().to_str().unwrap()).unwrap())))
            },
        )?;
    
    let mut materials = Vec::new();
    for m in obj_materials? {
        if let Some(diffuse_texture) = &m.diffuse_texture {
            let diffuse_texture = load_texture(source_path.parent().unwrap().join(diffuse_texture).as_os_str().to_str().unwrap(), &load_resource, device, queue)?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler.as_ref().unwrap()),
                    },
                    ],
                    label: None,
                });
                
                materials.push(Material {
                    name: m.name,
                    diffuse_texture,
                    bind_group,
                })
            }
        }
        
        let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
            .map(|i| ModelVertex {
                position: [
                    m.mesh.positions[i * 3],
                    m.mesh.positions[i * 3 + 1],
                    m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                        ],
                    })
                    .collect::<Vec<_>>();
                
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Vertex Buffer", name.clone().unwrap_or("".to_string()))),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Index Buffer", name.clone().unwrap_or("".to_string()))),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                
                Mesh {
                    name: name.clone(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();
        
        Ok(MeshModel { meshes, materials })
    }
}
    
pub fn load_texture(
    file_name: &str,
    load_resource: impl Fn(&str) -> Option<&&[u8]>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<Texture> {
    let data = load_resource(file_name).unwrap();
    Texture::from_bytes(device, queue, &data, file_name, None)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Descriptor for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
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

impl ToRaw for ModelVertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}