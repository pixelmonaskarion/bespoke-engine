use wgpu::{BindGroupLayout, Device, PipelineCompilationOptions, PipelineLayout, RenderPass, RenderPipeline, ShaderModule, TextureFormat};

use crate::{binding::Descriptor, texture::DepthTexture, window::BasicVertex};

pub struct Shader {
    pub shader: ShaderModule,
    pub layout: PipelineLayout,
    pub pipeline: RenderPipeline,
}

impl Shader {
    pub fn new(source: &str, device: &Device, format: TextureFormat, bindings: Vec<&BindGroupLayout>, vertex_buffers: &[wgpu::VertexBufferLayout<'_>], config: ShaderConfig) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let depth_stencil = if config.enable_depth_texture {
            Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: config.background,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };
        let targets = &[Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let fragment = if !config.depth_only {
            Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets,
                compilation_options: PipelineCompilationOptions::default(),
            })
        } else {
            None
        };
        let layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bindings,
                push_constant_ranges: &[],
            });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_buffers,
                compilation_options: PipelineCompilationOptions::default(),
            },
            depth_stencil,
            fragment,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: config.line_mode,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });
        Self {
            shader,
            layout,
            pipeline,
        }
    }

    pub fn new_post_process(source: &str, device: &Device, format: TextureFormat, bindings: &[&wgpu::BindGroupLayout]) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Processing Shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bindings,
                push_constant_ranges: &[],
            });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[BasicVertex::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            // depth_stencil: Some(wgpu::DepthStencilState {
            //     format: DepthTexture::DEPTH_FORMAT,
            //     depth_write_enabled: full_config.background,
            //     depth_compare: wgpu::CompareFunction::Less,
            //     stencil: wgpu::StencilState::default(),
            //     bias: wgpu::DepthBiasState::default(),
            // }),
            depth_stencil: None,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });
        Self {
            shader,
            layout,
            pipeline
        }
    }

    pub fn bind<'pass, 's: 'pass>(&'s self, render_pass: &mut RenderPass<'pass>) {
        render_pass.set_pipeline(&self.pipeline);
    }
}

pub struct ShaderConfig {
    pub background: bool,
    pub line_mode: wgpu::PolygonMode,
    pub enable_depth_texture: bool,
    pub depth_only: bool,
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self { background: true, line_mode: wgpu::PolygonMode::Fill, enable_depth_texture: true, depth_only: false }
    }
}