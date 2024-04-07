use wgpu::{BindGroup, Buffer, ComputePipeline, Device, Queue};

pub struct ComputeShader {
    pipeline: ComputePipeline,
}

impl ComputeShader {
    pub fn new(source: &str, bindings: &[&wgpu::BindGroupLayout], device: &Device) -> Self {
        let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)).into(),
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
            entry_point: "main",
        });
        Self {
            pipeline,
        }
    }

    pub fn run(&self, bind_groups: &[&BindGroup], groups: [u32; 3], device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&self.pipeline);
            for (i, bind_group) in bind_groups.into_iter().enumerate() {
                cpass.set_bind_group(i as u32, bind_group, &[]);
            }
            cpass.dispatch_workgroups(groups[0], groups[1], groups[2]);
        }
        queue.submit(Some(encoder.finish()));
    }
}

pub struct ComputeOutput {
    pub buffer: Buffer,
}

impl ComputeOutput {
    pub fn new(size: u64, device: &Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        Self {
            buffer
        }
    }

    pub fn read(&self) -> Vec<u8> {
        self.buffer.slice(..).get_mapped_range().to_vec()
    }
}