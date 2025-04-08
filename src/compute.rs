use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, Buffer, CommandEncoder, ComputePipeline, Device, PipelineCompilationOptions, Queue};

use crate::shader::{parse_shader, ShaderType};

pub struct ComputeShader {
    pub pipeline: ComputePipeline,
}

impl ComputeShader {
    pub fn new(source: &str, bindings: &[&wgpu::BindGroupLayout], shader_types: Vec<&ShaderType>, device: &Device) -> Self {
        let parsed_source = parse_shader(source, &shader_types.clone().into_iter().map(|it| it.clone()).collect());
        let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(parsed_source.into()).into(),
        });
        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: bindings,
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&compute_pipeline_layout),
            module: &cs_module,
            entry_point: Some("main"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            pipeline,
        }
    }

    pub fn run_once(&self, bind_groups: Vec<&BindGroup>, groups: [u32; 3], device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&self.pipeline);
            for (i, bind_group) in bind_groups.into_iter().enumerate() {
                cpass.set_bind_group(i as u32, Some(bind_group), &[]);
            }
            cpass.dispatch_workgroups(groups[0], groups[1], groups[2]);
        }
        queue.submit(Some(encoder.finish()));
    }
}

pub struct ComputeOutput {
    pub buffer: Buffer,
    pub binding: BindGroup,
    pub layout: BindGroupLayout,
}

impl ComputeOutput {
    pub fn new(size: u64, device: &Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let layout = 
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }]
            });
            let binding = device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }]
            });
        Self {
            buffer,
            binding,
            layout,
        }
    }

    pub fn read(&self, device: &Device, queue: &Queue) -> Vec<u8> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Output Map Buffer"),
            size: self.buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &map_buffer, 0, self.buffer.size());
        queue.submit([encoder.finish()]);
        map_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| {
                result.unwrap();
            });
        device.poll(wgpu::Maintain::Wait);
        let bytes = map_buffer.slice(..).get_mapped_range().to_vec();
        map_buffer.unmap();
        bytes
    }
    pub fn read_encoder(self, encoder: &mut CommandEncoder, device: &Device) -> Vec<u8> {
        let map_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Output Map Buffer"),
            size: self.buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &map_buffer, 0, self.buffer.size());
        map_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| {
                result.unwrap();
            });
        device.poll(wgpu::Maintain::Wait);
        let bytes = map_buffer.slice(..).get_mapped_range().to_vec();
        map_buffer.unmap();
        bytes
    }
}