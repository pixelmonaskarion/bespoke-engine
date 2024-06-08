use std::collections::HashMap;

use bytemuck::bytes_of;
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, BindingResource, Buffer, BufferUsages, Device};

pub struct UniformBinding<B: Binding> {
    pub buffers: HashMap<u32, Buffer>,
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
    pub label: &'static str,
    pub value: B,
}

impl <B: Binding> UniformBinding<B> {
    pub fn new(device: &Device, label: &'static str, value: B, ty: Option<wgpu::BindingType>) -> Self {
        let mut buffers = HashMap::new();
        let resources = value.create_resources();
        let mut bind_group_entries = vec![];
        for (i, resource) in resources.into_iter().enumerate() {
            match resource {
                Resource::Simple(bytes) => {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{} Buffer", label)),
                        contents: &bytes,
                        usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE,
                    });
                    buffers.insert(i as u32, buffer);
                    // bind_group_entries.push(wgpu::BindGroupEntry {
                    //     binding: i as u32,
                    //     resource: binding,
                    // });
                }
                Resource::Bespoke(binding) => {
                    bind_group_entries.push(wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: binding,
                    });
                }
            }
        }
        for (i, buffer) in &buffers {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: *i,
                resource: buffer.as_entire_binding(),
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &B::layout(ty),
            label: Some(&format!("{label} Uniform Layout")),
        });
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &bind_group_entries,
            label: Some(&format!("{label} Binding")),
        });
        Self {
            buffers,
            layout,
            binding,
            label,
            value,
        }
    }

    // pub fn layout(label: &'static str, ty: Option<wgpu::BindingType>, device: &Device) -> BindGroupLayout {
    //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //         entries: &[wgpu::BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: wgpu::ShaderStages::all(),
    //             ty: ty.unwrap_or(wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Uniform,
    //                 has_dynamic_offset: false,
    //                 min_binding_size: None,
    //             }),
    //             count: None,
    //         }],
    //         label: Some(&format!("{label} Uniform Layout")),
    //     })
    // }

    pub fn set_data(&mut self, device: &Device, value: B) {
        self.value = value;
        self.buffers = HashMap::new();
        let resources = self.value.create_resources();
        let mut bind_group_entries = vec![];
        for (i, resource) in resources.into_iter().enumerate() {
            match resource {
                Resource::Simple(bytes) => {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{} Buffer", self.label)),
                        contents: &bytes,
                        usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE,
                    });
                    self.buffers.insert(i as u32, buffer);
                    // bind_group_entries.push(wgpu::BindGroupEntry {
                    //     binding: i as u32,
                    //     resource: binding,
                    // });
                }
                Resource::Bespoke(binding) => {
                    bind_group_entries.push(wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: binding,
                    });
                }
            }
        }
        for (i, buffer) in &self.buffers {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: *i,
                resource: buffer.as_entire_binding(),
            });
        }
        self.binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &bind_group_entries,
            label: Some(&format!("{} Binding", self.label)),
        });
    }
}

pub trait Descriptor {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

pub trait Binding {
    fn layout(ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry>;
    fn create_resources<'a>(&'a self) -> Vec<Resource>;
    // fn create_binding<'a>(&self, bindings: Vec<wgpu::BindingResource<'a>>) -> Vec<wgpu::BindGroupEntry<'a>>;
}

pub enum Resource<'a> {
    Simple(Vec<u8>),
    Bespoke(BindingResource<'a>)
}

impl <T: bytemuck::Pod> Binding for T {
    fn layout(ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::all(),
                ty: ty.unwrap_or(wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None })
            }
        ]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        vec![Resource::Simple(bytes_of(self).to_vec())]
    }

    // fn create_binding<'a>(&self, binding: Vec<wgpu::BindingResource<'a>>) -> Vec<wgpu::BindGroupEntry<'a>> {
    //     vec![wgpu::BindGroupEntry {
    //         binding: 0,
    //         resource: binding[0],
    //     }]
    // }
}

pub fn bind_resources<'a, B: Binding>(value: &B, device: &Device) -> BindGroup {
    let resources = value.create_resources();
    let mut buffers = HashMap::new();
    let mut bind_group_entries = vec![];
    for (i, resource) in resources.into_iter().enumerate() {
        match resource {
            Resource::Simple(bytes) => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes,
                    usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE,
                });
                buffers.insert(i as u32, buffer);
                // bind_group_entries.push(wgpu::BindGroupEntry {
                //     binding: i as u32,
                //     resource: binding,
                // });
            }
            Resource::Bespoke(binding) => {
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: binding,
                });
            }
        }
    }
    for (i, buffer) in &buffers {
        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: *i,
            resource: buffer.as_entire_binding(),
        });
    }
    let layout = create_layout::<B>(device);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &bind_group_entries,
        label: None,
    })
}

pub fn create_layout<B: Binding>(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &B::layout(None),
        label: None,
    })
}