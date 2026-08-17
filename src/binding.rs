use std::{collections::HashMap, num::NonZero};

use bytemuck::bytes_of;
use wgpu::{BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingResource, Buffer, BufferBinding, BufferUsages, Device, DynamicOffset, Queue, util::DeviceExt};

use crate::shader::ShaderType;

#[derive(Clone)]
pub struct UniformBinding<B: Binding> {
    pub buffers: HashMap<u32, Buffer>,
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
            entries: &B::layout(value.layout_config(), ty),
            label: Some(&format!("{label} Uniform Layout")),
        });
        let (binding, buffers) = Self::create_bind_group(&value, label, &layout, device);
        let shader_type = B::shader_type(value.layout_config());
        Self {
            buffers,
            layout,
            binding,
            label,
            value,
            shader_type,
        }
    }

    pub fn create_bind_group(value: &B, label: &'static str, layout: &BindGroupLayout, device: &Device) -> (BindGroup, HashMap<u32, Buffer>) {
        let mut buffers = HashMap::new();
        let mut buffer_specifications = HashMap::new();
        let resources = value.create_resources();
        let mut bind_group_entries = vec![];
        for (i, resource) in resources.into_iter().enumerate() {
            match resource {
                Resource::Simple(bytes) => {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{} Buffer", label)),
                        contents: &bytes,
                        usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
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
                },
                Resource::BufferWith { bytes, offset, size } => {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{} Buffer", label)),
                        contents: &bytes,
                        usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    });
                    buffers.insert(i as u32, buffer);
                    buffer_specifications.insert(i as u32, (offset, size));
                }
            }
        }
        for (i, buffer) in &buffers {
            let resource = if let Some((offset, size)) = buffer_specifications.remove(i) {
                BindingResource::Buffer(BufferBinding {
                    buffer: &buffer,
                    offset: offset,
                    size: size
                })
            } else {
                buffer.as_entire_binding()
            };
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: *i,
                resource,
            });
        }
        
        let binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &bind_group_entries,
            label: Some(&format!("{label} Binding")),
        });
        (binding, buffers)
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
    
    pub fn set_data(&mut self, queue: &Queue, value: B) {
        self.value = value;
        let resources = self.value.create_resources();
        for (i, resource) in resources.into_iter().enumerate() {
            match resource {
                Resource::Simple(bytes) => {
                    if let Some(buffer) = self.buffers.get(&(i as u32)) {
                        queue.write_buffer(buffer, 0, &bytes);
                    }
                },
                Resource::BufferWith { bytes, .. } => {
                    if let Some(buffer) = self.buffers.get(&(i as u32)) {
                        queue.write_buffer(buffer, 0, &bytes);
                    }
                },
                Resource::Bespoke(_) => {}
            }
        }
        queue.submit([]);
    }

    pub fn replace_data(&mut self, device: &Device, value: B) {
        self.value = value;
        let resources = self.value.create_resources();
        let mut bind_group_entries = vec![];
        let mut buffer_specifications = HashMap::new();
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
                Resource::BufferWith { bytes, offset, size } => {
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{} Buffer", self.label)),
                        contents: &bytes,
                        usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    });
                    self.buffers.insert(i as u32, buffer);
                    buffer_specifications.insert(i as u32, (offset, size));
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
            let resource = if let Some((offset, size)) = buffer_specifications.remove(i) {
                BindingResource::Buffer(BufferBinding {
                    buffer: &buffer,
                    offset: offset,
                    size: size
                })
            } else {
                buffer.as_entire_binding()
            };
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: *i,
                resource,
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
    type LayoutConfig;
    fn layout_config(&self) -> Self::LayoutConfig;
    fn layout(config: Self::LayoutConfig, ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry>;
    fn create_resources<'a>(&'a self) -> Vec<Resource<'a>>;
    fn shader_type(config: Self::LayoutConfig) -> ShaderType;
    // fn create_binding<'a>(&self, bindings: Vec<wgpu::BindingResource<'a>>) -> Vec<wgpu::BindGroupEntry<'a>>;
}

pub enum Resource<'a> {
    Simple(Vec<u8>),
    Bespoke(BindingResource<'a>),
    BufferWith{ bytes: Vec<u8>, offset: u64, size: Option<NonZero<u64>> },
}

impl <T: bytemuck::Pod + WgslType> Binding for T {
    type LayoutConfig = ();
    fn layout(_config: Self::LayoutConfig, ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::all(),
                ty: ty.unwrap_or(wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None })
            }
        ]
    }

    fn layout_config(&self) -> Self::LayoutConfig {
        ()
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource<'a>> {
        let mut bytes = bytes_of(self).to_vec();
        //I have no idea why it's /4 and *4
        if size_of::<Self>()/4 != 0 {
            bytes.append(&mut vec![0; (4 - ((size_of::<Self>()/4) % 4)) * 4]);
        }
        vec![Resource::Simple(bytes)]
    }

    fn shader_type(_config: ()) -> ShaderType {
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

pub struct DynamicOffsetUniform<T, const N: usize> { pub values: [T; N], pub alignment: usize }

impl <T, const N: usize> UniformBinding<DynamicOffsetUniform<T, N>> where DynamicOffsetUniform<T, N>: Binding {
    pub fn dynamic_offset_for_index(&self, index: usize) -> DynamicOffset {
        (index * size_of::<T>().max(self.value.alignment)) as DynamicOffset
    }
}

impl <T: bytemuck::Pod + WgslType, const N: usize> Binding for DynamicOffsetUniform<T, N> {
    type LayoutConfig = ();
    fn layout(_config: Self::LayoutConfig, ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::all(),
                ty: ty.unwrap_or(wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: NonZero::new(size_of::<T>() as u64) })
            }
        ]
    }

    fn layout_config(&self) -> Self::LayoutConfig {
        ()
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource<'a>> {
        let bytes = self.values.iter().map(|value| {
            let mut bytes = bytes_of(value).to_vec();
            if bytes.len() % self.alignment != 0 {
                bytes.append(&mut vec![0; self.alignment - (bytes.len() % self.alignment)]);
            }
            bytes
        }).flatten().collect::<Vec<u8>>();
        vec![Resource::BufferWith { bytes, offset: 0, size: NonZero::new(size_of::<T>() as u64) }]
    }

    fn shader_type(_config: ()) -> ShaderType {
        T::shader_type(())
    }
}

pub struct DynamicOffsetUniformVec<T, const N: usize> { pub values: Vec<T>, pub alignment: usize }

impl <T, const N: usize> UniformBinding<DynamicOffsetUniformVec<T, N>> where DynamicOffsetUniformVec<T, N>: Binding {
    pub fn dynamic_offset_for_index(&self, index: usize) -> DynamicOffset {
        (index * size_of::<T>().max(self.value.alignment)) as DynamicOffset
    }
}

impl <T: bytemuck::Pod + WgslType, const N: usize> Binding for DynamicOffsetUniformVec<T, N> {
    type LayoutConfig = ();
    fn layout(_config: Self::LayoutConfig, ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::all(),
                ty: ty.unwrap_or(wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: NonZero::new(size_of::<T>() as u64) })
            }
        ]
    }

    fn layout_config(&self) -> Self::LayoutConfig {
        ()
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource<'a>> {
        let bytes = self.values.iter().map(|value| {
            let mut bytes = bytes_of(value).to_vec();
            if bytes.len() % self.alignment != 0 {
                bytes.append(&mut vec![0; self.alignment - (bytes.len() % self.alignment)]);
            }
            bytes
        }).flatten().collect::<Vec<u8>>();
        vec![Resource::BufferWith { bytes, offset: 0, size: NonZero::new(size_of::<T>() as u64) }]
    }

    fn shader_type(_config: ()) -> ShaderType {
        T::shader_type(())
    }
}

pub fn bind_resources<'a, B: Binding>(value: &B, device: &Device) -> BindGroup {
    let resources = value.create_resources();
    let mut buffers = HashMap::new();
    let mut buffer_specifications = HashMap::new();
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
            },
            Resource::BufferWith { bytes, offset, size } => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: &bytes,
                    usage: BufferUsages::UNIFORM | BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
                buffers.insert(i as u32, buffer);
                buffer_specifications.insert(i as u32, (offset, size));
            },
            Resource::Bespoke(binding) => {
                bind_group_entries.push(wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: binding,
                });
            },
        }
    }
    for (i, buffer) in &buffers {
        let resource = if let Some((offset, size)) = buffer_specifications.remove(i) {
            BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: offset,
                size: size
            })
        } else {
            buffer.as_entire_binding()
        };
        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: *i,
            resource,
        });
    }
    let layout = create_layout::<B>(value.layout_config(), device);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &bind_group_entries,
        label: None,
    })
}

pub fn create_layout<B: Binding>(config: B::LayoutConfig, device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &B::layout(config, None),
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