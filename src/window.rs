use std::time::SystemTime;

use bytemuck::{bytes_of, NoUninit};
use cgmath::Vector2;
use wgpu::{Backends, Device, InstanceDescriptor, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, Event, KeyEvent, Touch, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use winit::event_loop::EventLoop;

use crate::binding::Descriptor;
use crate::model::{Model, Render, ToRaw};
use crate::shader::Shader;
use crate::texture::{DepthTexture, Texture};

pub struct SurfaceContext<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: SurfaceConfiguration,
    pub depth_texture: DepthTexture,
    pub device: Device,
    pub queue: Queue,
    pub screen_model: Model,
    pub texture_renderer_shader: Shader,
}

pub struct Surface<'b: 'a, 'a> {
    window_id: WindowId,
    pub window: &'b Window,
    pub instance: wgpu::Instance,
    surface_context: Option<SurfaceContext<'a>>,
    pub size: PhysicalSize<u32>,
    pub mouse_pos: [f64; 2],
    pub last_time: SystemTime,
}

impl <'b: 'a, 'a>Surface<'b, 'a> {
    pub async fn new(window: &'b Window) -> Self {
        let size = window.inner_size();
        let window_id = window.id();
        let instance = wgpu::Instance::new(InstanceDescriptor { 
            backends: Backends::all(),
            ..Default::default()
        });
        return Self {
            window_id,
            window,
            instance,
            surface_context: None,
            mouse_pos: [0.0, 0.0],
            size,
            last_time: SystemTime::now(),
        }
    }

    pub fn run<T, H: WindowHandler>(mut self, event_loop: EventLoop<T>, ready: &dyn Fn(&SurfaceContext) -> H) {
        let mut handler: Option<H> = None;
        event_loop.run(move |event, control_flow| {
            match event {
                Event::DeviceEvent { event,  ..} => {
                    match event {
                        DeviceEvent::MouseMotion { delta } => {
                            self.mouse_pos[0] += delta.0;
                            self.mouse_pos[1] += delta.1;
                            if let Some(surface_context) = &self.surface_context {
                                if let Some(handler) = &mut handler {
                                    handler.mouse_motion(&surface_context.device, delta);
                                    handler.mouse_moved(&surface_context.device, PhysicalPosition { x: self.mouse_pos[0], y: self.mouse_pos[1] });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window_id => {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            control_flow.exit();
                        },
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let Some(surface_context) = &self.surface_context {
                                if let Some(handler) = &mut handler {
                                    handler.input_event(&surface_context.device, event);
                                }
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if let Some(surface_context) = &self.surface_context {
                                if let Some(handler) = &mut handler {
                                    handler.mouse_moved(&surface_context.device, *position);
                                }
                            }
                        }
                        WindowEvent::Touch(touch) => {
                            if let Some(surface_context) = &mut self.surface_context {
                                if let Some(handler) = &mut handler {
                                    handler.touch(&surface_context.device, touch);
                                }
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            if let Some(surface_context) = &mut self.surface_context {
                                surface_context.config.width = physical_size.width;
                                surface_context.config.height = physical_size.height;
                                if let Some(handler) = &mut handler {
                                    handler.resize(&surface_context.device, Vector2::new(surface_context.config.width, surface_context.config.height));
                                }
                                surface_context.surface.configure(&surface_context.device, &surface_context.config);
                                surface_context.depth_texture = DepthTexture::create_depth_texture(&surface_context.device, &surface_context.config, "Depth Texture");
                            }
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            if let Some(surface_context) = &mut self.surface_context {
                                surface_context.config.width = (surface_context.config.width as f64**scale_factor) as u32;
                                surface_context.config.height = (surface_context.config.height as f64**scale_factor) as u32;
                                if let Some(handler) = &mut handler {
                                    handler.resize(&surface_context.device, Vector2::new(surface_context.config.width, surface_context.config.height));
                                }
                                surface_context.depth_texture = DepthTexture::create_depth_texture(&surface_context.device, &surface_context.config, "Depth Texture");
                                surface_context.surface.configure(&surface_context.device, &surface_context.config);
                        }
                        }
                        WindowEvent::RedrawRequested if window_id == self.window_id => {
                            if let Some(surface_context) = &self.surface_context {
                                let delta = SystemTime::now().duration_since(self.last_time).unwrap().as_nanos() as f64 / 1000000.0;
                                self.last_time = SystemTime::now();
                                let temp_texture = Texture::blank_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, surface_context.config.format);
                                let output = surface_context.surface.get_current_texture().unwrap();
                                let view = output
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());
                                let mut encoder = surface_context
                                    .device
                                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                        label: Some("Render Encoder"),
                                    });
                                //render the game to a temporary texture
                                {
                                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Temp Render Pass"),
                                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                            view: &temp_texture.view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(FullWindowConfig::load_defaults(handler.as_ref().map(|handler| handler.config())).background_color),
                                                store: wgpu::StoreOp::Store,
                                            },
                                        })],
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                            view: &surface_context.depth_texture.view,
                                            depth_ops: Some(wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        }),
                                    });
                                    if let Some(handler) = &mut handler {
                                        handler.render(&surface_context.device, &mut render_pass, delta);
                                    }
                                }
                                //create another temporary texture and use it to render post processing effects
                                let post_process_texture = Texture::blank_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, surface_context.config.format);
                                {
                                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Post Processing Render Pass"),
                                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                            view: &post_process_texture.view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                                store: wgpu::StoreOp::Store,
                                            },
                                        })],
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                            view: &surface_context.depth_texture.view,
                                            depth_ops: Some(wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        }),
                                    });
                                    if let Some(handler) = &mut handler {
                                        handler.post_process_render(&surface_context.device, &mut render_pass, &surface_context.screen_model, &temp_texture);
                                    }
                                }
                                //render that texture onto the screen
                                {
                                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Surface Render Pass"),
                                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                            view: &view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                                store: wgpu::StoreOp::Store,
                                            },
                                        })],
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                            view: &surface_context.depth_texture.view,
                                            depth_ops: Some(wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        }),
                                    });
                                    render_pass.set_pipeline(&surface_context.texture_renderer_shader.pipeline);
                                    render_pass.set_bind_group(0, &post_process_texture.binding, &[]);

                                    surface_context.screen_model.render(&mut render_pass);
                                }
                                surface_context.queue.submit([encoder.finish()]);

                                output.present();

                                self.window.request_redraw();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resumed => {
                    pollster::block_on(async {
                        self.size = self.window.inner_size();
                        if self.window.inner_size() == (PhysicalSize { width: 0, height: 0 }) {
                            return;
                        }
                        let surface = self.instance.create_surface(self.window).unwrap();
                        let adapter = self.instance.request_adapter(
                            &wgpu::RequestAdapterOptions {
                                power_preference: wgpu::PowerPreference::default(),
                                compatible_surface: Some(&surface),
                                force_fallback_adapter: false,
                            },
                        ).await.unwrap();
                        let (device, queue) = adapter.request_device(
                            &wgpu::DeviceDescriptor {
                                required_features: wgpu::Features::empty(), //Android doesn't support vertex writable storage, not sure what I'm going to do now :/
                                required_limits: wgpu::Limits {
                                    max_bind_groups: 4,
                                    max_texture_dimension_2d: 16384,
                                    ..Default::default()
                                },
                                label: None 
                            },
                            None,
                        ).await.unwrap();
                        let config = surface.get_default_config(&adapter, self.size.width, self.size.height).unwrap();
                        surface.configure(&device, &config);
                        let depth_texture = DepthTexture::create_depth_texture(&device, &config, "Depth Texture");
                        let texture_renderer_shader = Shader::new(include_str!("screen_renderer.wgsl"), &device, config.format, &[&Texture::layout(&device, None, None)], &[BasicVertex::desc()], None);
                        let screen_model = BasicVertex::one_face(&device);
                        self.surface_context = Some(SurfaceContext {
                            surface,
                            config,
                            depth_texture,
                            device,
                            queue,
                            screen_model,
                            texture_renderer_shader,
                        });
                        handler = Some(ready(self.surface_context.as_ref().unwrap()));
                    });
                }
                Event::Suspended => {
                    log::error!("help");
                    println!("suspended!");
                }
                _ => {}
            }
        }).unwrap();
    }
}

pub trait WindowHandler {
    fn resize(&mut self, device: &Device, new_size: Vector2<u32>);
    fn render<'a: 'b, 'b>(&'a mut self, device: &Device, render_pass: & mut RenderPass<'b>, delta: f64);
    fn config(&self) -> WindowConfig;
    fn mouse_moved(&mut self, device: &Device, mouse_pos: PhysicalPosition<f64>);
    fn mouse_motion(&mut self, device: &Device, mouse_delta: (f64, f64));
    fn input_event(&mut self, device: &Device, input_event: &KeyEvent);
    fn touch(&mut self, device: &Device, touch: &Touch);
    fn post_process_render<'a: 'b, 'c: 'b, 'b>(&'a mut self, device: &Device, render_pass: & mut RenderPass<'b>, screen_model: &'c Model, surface_texture: &'c Texture);
}

pub struct WindowConfig {
    pub background_color: Option<wgpu::Color>,
}

struct FullWindowConfig {
    background_color: wgpu::Color,
}

impl FullWindowConfig {
    fn load_defaults(config: Option<WindowConfig>) -> Self {
        Self {
            background_color: config.map(|c| c.background_color).flatten().unwrap_or(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }),
        }
    }
}

#[repr(C)]
#[derive(NoUninit, Copy, Clone)]
pub struct BasicVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl BasicVertex {
    pub fn one_face(device: &Device) -> Model {
        Model::new(vec![
            Self { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
            Self { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0] },
            Self { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] },
            Self { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], device)
    }
}

impl Descriptor for BasicVertex {
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
            ],
        }
    }
}

impl ToRaw for BasicVertex {
    fn to_raw(&self) -> Vec<u8> {
        bytes_of(self).to_vec()
    }
}