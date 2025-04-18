use std::sync::Mutex;

use wgpu::{BindGroupLayout, Device, FrontFace, PipelineCompilationOptions, PipelineLayout, RenderPass, RenderPipeline, ShaderModule, TextureFormat};

use crate::{binding::{Descriptor, Uniform}, resource_loader::load_resource_string, texture::DepthTexture, window::BasicVertex};

pub static CUSTOM_SHADER_TYPE_SOURCE: Mutex<String> = Mutex::new(String::new());

pub struct Shader<'a> {
    pub shader: ShaderModule,
    pub layout: PipelineLayout,
    pub pipeline: RenderPipeline,
    pub resource_path: String,
    pub config: ShaderConfig,
    pub vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    pub shader_types: Vec<ShaderType>,
    pub formats: Vec<TextureFormat>,
}

impl <'a> Shader<'a> {
    pub fn new(resource_path: &str, device: &Device, formats: Vec<TextureFormat>, bindings: Vec<&BindGroupLayout>, shader_types: Vec<&ShaderType>, vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>, config: ShaderConfig) -> Self {
        let source = &load_resource_string(resource_path);
        let shader_types_owned = shader_types.clone().into_iter().map(|it| it.clone()).collect();
        let parsed_source = parse_shader(source, &shader_types_owned);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(parsed_source.clone().into()),
        });
        let targets = &formats.iter().map(|format| {
            Some(wgpu::ColorTargetState {
                format: *format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })
        }).collect::<Vec<Option<wgpu::ColorTargetState>>>();
        let fragment = if !config.depth_only {
            Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: PipelineCompilationOptions::default(),
            },
            depth_stencil: config.depth_stencil(),
            fragment,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: config.face_cull.unwrap_or(FrontFace::Ccw),
                cull_mode: config.face_cull.map(|_| wgpu::Face::Back),
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
            cache: None,
        });
        Self {
            shader,
            layout,
            pipeline,
            resource_path: resource_path.into(),
            config,
            vertex_buffers,
            shader_types: shader_types_owned,
            formats,
        }
    }

    pub fn reload_source(&mut self, device: &Device) {
        let source = &load_resource_string(&self.resource_path);
        let parsed_source = parse_shader(source, &self.shader_types);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(parsed_source.clone().into()),
        });
        let targets = &self.formats.iter().map(|format| {
            Some(wgpu::ColorTargetState {
                format: *format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })
        }).collect::<Vec<Option<wgpu::ColorTargetState>>>();
        let fragment = if !self.config.depth_only {
            Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets,
                compilation_options: PipelineCompilationOptions::default(),
            })
        } else {
            None
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &self.vertex_buffers,
                compilation_options: PipelineCompilationOptions::default(),
            },
            depth_stencil: self.config.depth_stencil(),
            fragment,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: self.config.face_cull.unwrap_or(FrontFace::Ccw),
                cull_mode: self.config.face_cull.map(|_| wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: self.config.line_mode,
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
            cache: None,
        });
        self.pipeline = pipeline;
    }

    pub fn new_uniform(resource_path: &str, device: &Device, formats: Vec<TextureFormat>, uniforms: Vec<&dyn Uniform>, vertex_buffers: Vec<wgpu::VertexBufferLayout<'a>>, config: ShaderConfig) -> Self {
        Self::new(resource_path, device, formats, uniforms.iter().map(|it| it.layout()).collect(), uniforms.iter().map(|it| it.shader_type()).collect(), vertex_buffers, config)
    }

    pub fn new_post_process(resource_path: &str, device: &Device, format: TextureFormat, bindings: Vec<&wgpu::BindGroupLayout>, binding_types: Vec<&ShaderType>) -> Self {
        let source = &load_resource_string(resource_path);
        let shader_types_owned = binding_types.clone().into_iter().map(|it| it.clone()).collect();
        let parsed_source = parse_shader(source, &shader_types_owned);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Processing Shader"),
            source: wgpu::ShaderSource::Wgsl(parsed_source.into()),
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
                entry_point: Some("vs_main"),
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
                entry_point: Some("fs_main"),
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
            cache: None,
        });
        Self {
            shader,
            layout,
            pipeline,
            resource_path: resource_path.into(),
            config: ShaderConfig { enable_depth_texture: false, ..Default::default() },
            vertex_buffers: vec![BasicVertex::desc()],
            shader_types: shader_types_owned,
            formats: vec![format],
        }
    }

    pub fn new_post_process_uniforms(source: &str, device: &Device, format: TextureFormat, uniforms: Vec<&dyn Uniform>) -> Self {
        Self::new_post_process(source, device, format, uniforms.iter().map(|it| it.layout()).collect(), uniforms.into_iter().map(|it| it.shader_type()).collect())
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
    pub face_cull: Option<FrontFace>,
    pub depth_compare: wgpu::CompareFunction,
}

impl ShaderConfig {
    fn depth_stencil(&self) -> Option<wgpu::DepthStencilState> {
        if self.enable_depth_texture {
            return Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: self.background,
                depth_compare: self.depth_compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            });
        }
        None
    }
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self { background: true, line_mode: wgpu::PolygonMode::Fill, enable_depth_texture: true, depth_only: false, face_cull: Some(FrontFace::Ccw), depth_compare: wgpu::CompareFunction::Less }
    }
}

pub fn parse_shader(shader_source: &str, binding_types: &Vec<ShaderType>) -> String {
    let global_types = load_resource_string("buildins/global_shader_types.wgsl");
    let custom_types = CUSTOM_SHADER_TYPE_SOURCE.lock().unwrap().clone();
    let mut parsed = format!(
"//GLOBAL TYPES
{global_types}
//CUSTOM TYPES
{custom_types}
//SHADER DEFINITION
{shader_source}
"
    );
    let mut i = 0;
    while let Some(dollar_i) = parsed.find("$") {
        i += 1;
        if i > 1000 {
            break;
        }
        if let Some(mut end_i) = parsed[dollar_i..].find(";") {
            end_i += dollar_i;
            let binding_id = &parsed[dollar_i+1..end_i].split(",").map(|it| it.parse::<u32>().unwrap()).collect::<Vec<u32>>();
            let line_start = dollar_i-parsed[0..dollar_i].chars().rev().collect::<String>().find("\n").unwrap_or(0);
            let variable_name = &parsed[line_start..dollar_i];
            if let Some(binding_type) = binding_types.get(binding_id[0] as usize) {
                let binding_i = *binding_id.get(1).unwrap_or(&0) as usize;
                parsed.replace_range(
                    line_start..end_i, 
                    &format!(
                        "@group({}) @binding({}) var{} {variable_name} {}", 
                        binding_id[0], 
                        binding_i, 
                        binding_type.var_types[binding_i], 
                        binding_type.wgsl_types[binding_i]
                    ));
            }
        }
    }
    println!("{parsed}");
    parsed
}

#[derive(Clone)]
pub struct ShaderType {
    pub var_types: Vec<String>,
    pub wgsl_types: Vec<String>,
}

impl ShaderType {
    pub fn buffer_type(writable: bool, inner_type: String) -> ShaderType {
        let var_type = if writable {
            "<storage, read_write>"
        } else {
            "<storage, read>"
        };
        ShaderType { var_types: vec![var_type.into()], wgsl_types: vec![format!("array<{inner_type}>")] }
    }

    pub fn multi_buffer_type(writable: Vec<bool>, inner_type: Vec<String>) -> ShaderType {
        let var_types = writable.into_iter().map(|it| {
            if it {
                "<storage, read_write>".to_string()
            } else {
                "<storage, read>".into()
            }
        }).collect();
        let wgsl_types = inner_type.into_iter().map(|it| {
            format!("array<{it}>")
        }).collect();
        ShaderType { 
            var_types, 
            wgsl_types,
        }
    }
}