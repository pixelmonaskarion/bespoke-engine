use std::collections::HashMap;

use bytemuck::bytes_of;
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BufferUsages, Device};

use crate::shader::ShaderType;

pub struct UniformBinding<B: Binding> {
    // pub buffers: HashMap<u32, Buffer>,
    pub layout: BindGroupLayout,
    pub binding: BindGroup,
    pub label: &'static str,
    pub shader_type: ShaderType,
    pub value: B,
}

pub trait Uniform {
    fn layout(&self) -> &BindGroupLayout;
    fn binding(&self) -> &BindGroup;
    fn label(&self) -> &'static str;
    fn shader_type(&self) -> &ShaderType;
}

impl <B: Binding> Uniform for UniformBinding<B> {
    fn layout(&self) -> &BindGroupLayout {
        &self.layout
    }

    fn binding(&self) -> &BindGroup {
        &self.binding
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn shader_type(&self) -> &ShaderType {
        &self.shader_type
    }
}

impl <B: Binding> UniformBinding<B> {
    pub fn new(device: &Device, label: &'static str, value: B, ty: Option<wgpu::BindingType>) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &B::layout(ty),
            label: Some(&format!("{label} Uniform Layout")),
        });
        let binding = Self::create_bind_group(&value, label, &layout, device);
        let shader_type = B::shader_type();
        Self {
            // buffers,
            layout,
            binding,
            label,
            value,
            shader_type,
        }
    }

    pub fn create_bind_group(value: &B, label: &'static str, layout: &BindGroupLayout, device: &Device) -> BindGroup {
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
        
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &bind_group_entries,
            label: Some(&format!("{label} Binding")),
        });
        binding
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
        let mut buffers = HashMap::new();
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
    fn shader_type() -> ShaderType;
    // fn create_binding<'a>(&self, bindings: Vec<wgpu::BindingResource<'a>>) -> Vec<wgpu::BindGroupEntry<'a>>;
}

pub enum Resource<'a> {
    Simple(Vec<u8>),
    Bespoke(BindingResource<'a>)
}

impl <T: bytemuck::Pod + WgslType> Binding for T {
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
        let mut bytes = bytes_of(self).to_vec();
        if size_of::<Self>()/4 != 0 {
            bytes.append(&mut vec![0; (4 - ((size_of::<Self>()/4) % 4)) * 4]);
        }
        vec![Resource::Simple(bytes)]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["<uniform>".into()],
            wgsl_types: vec![T::wgsl_name()],
        }
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

pub fn create_layout_from_entries(entries: &[wgpu::BindGroupLayoutEntry], device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries,
        label: None,
    })
}

pub trait WgslType {
    fn wgsl_name() -> String;
}

pub fn simple_layout_entry(binding: u32) -> BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}

impl WgslType for f32 {fn wgsl_name() -> String {"f32".into()}}
impl WgslType for f64 {fn wgsl_name() -> String {"f64".into()}}
impl WgslType for i32 {fn wgsl_name() -> String {"i32".into()}}
impl WgslType for i64 {fn wgsl_name() -> String {"i64".into()}}
impl WgslType for u32 {fn wgsl_name() -> String {"u32".into()}}
impl WgslType for u64 {fn wgsl_name() -> String {"u64".into()}}
impl WgslType for [f32; 2] {fn wgsl_name() -> String {"vec2f".into()}}
impl WgslType for [f32; 3] {fn wgsl_name() -> String {"vec3f".into()}}
impl WgslType for [f32; 4] {fn wgsl_name() -> String {"vec4f".into()}}
impl WgslType for [i32; 2] {fn wgsl_name() -> String {"vec2i".into()}}
impl WgslType for [i32; 3] {fn wgsl_name() -> String {"vec3i".into()}}
impl WgslType for [i32; 4] {fn wgsl_name() -> String {"vec4i".into()}}
impl WgslType for [u32; 2] {fn wgsl_name() -> String {"vec2u".into()}}
impl WgslType for [u32; 3] {fn wgsl_name() -> String {"vec3u".into()}}
impl WgslType for [u32; 4] {fn wgsl_name() -> String {"vec4u".into()}}
impl WgslType for [[f32; 4]; 4] {fn wgsl_name() -> String {"mat4x4f".into()}}