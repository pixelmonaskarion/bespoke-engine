use wgpu::{BindGroupLayout, Device, PipelineCompilationOptions, PipelineLayout, RenderPass, RenderPipeline, ShaderModule, TextureFormat};

use crate::{binding::Descriptor, texture::DepthTexture, window::BasicVertex};

pub struct Shader {
    pub shader: ShaderModule,
    pub layout: PipelineLayout,
    pub pipeline: RenderPipeline,
}

impl Shader {
    pub fn new(source: &str, device: &Device, format: TextureFormat, bindings: Vec<&BindGroupLayout>, vertex_buffers: &[wgpu::VertexBufferLayout<'_>], config: Option<ShaderConfig>) -> Self {
        let full_config = FullShaderConfig::load_defaults(config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let depth_stencil = if full_config.enable_depth_texture {
            Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: full_config.background,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
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
                polygon_mode: full_config.line_mode,
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
        let _full_config = FullShaderConfig::load_defaults(None);
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
    pub background: Option<bool>,
    pub line_mode: Option<wgpu::PolygonMode>,
    pub enable_depth_texture: Option<bool>
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self { background: None, line_mode: None, enable_depth_texture: None }
    }
}

struct FullShaderConfig {
    background: bool,
    line_mode: wgpu::PolygonMode,
    enable_depth_texture: bool,
}

impl FullShaderConfig {
    fn load_defaults(config: Option<ShaderConfig>) -> Self {
        Self {
            background: config.as_ref().map(|c| c.background).flatten().unwrap_or(true),
            line_mode: config.as_ref().map(|c| c.line_mode).flatten().unwrap_or(wgpu::PolygonMode::Fill),
            enable_depth_texture: config.as_ref().map(|c| c.enable_depth_texture).flatten().unwrap_or(true),
        }
    }
}