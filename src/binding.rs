use bytemuck::{bytes_of, NoUninit};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Buffer, BufferUsages, Device};

pub struct UniformBinding {
    pub buffer: Buffer,
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
    pub label: &'static str,
}

impl UniformBinding {
    pub fn new<A: NoUninit>(device: &Device, label: &'static str, data: A, ty: Option<wgpu::BindingType>) -> Self {
        Self::new_bytes(device, label, bytes_of(&data), ty)
    }
    pub fn new_bytes(device: &Device, label: &'static str, data: &[u8], ty: Option<wgpu::BindingType>) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label} Buffer")),
            contents: data,
            usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE,
        });
        let layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: ty.unwrap_or(wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }),
                    count: None,
                }],
                label: Some(&format!("{label} Uniform Layout")),
            });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some(&format!("{label} Binding")),
        });
        Self {
            buffer,
            layout,
            binding,
            label,
        }
    }

    pub fn set_data<A: NoUninit>(&mut self, device: &Device, data: A) {
        self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Buffer", self.label)),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        self.binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.buffer.as_entire_binding(),
            }],
            label: Some(&format!("{} Binding", self.label)),
        });
    }
}

pub trait Descriptor {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}