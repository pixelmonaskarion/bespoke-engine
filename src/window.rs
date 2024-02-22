use cgmath::Vector2;
use wgpu::{Backends, Device, InstanceDescriptor, Queue, RenderPass, SurfaceConfiguration};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use winit::event_loop::EventLoop;

use crate::texture::DepthTexture;

pub struct Surface<'b: 'a, 'a> {
    window_id: WindowId,
    pub window: &'b Window,
    pub config: SurfaceConfiguration,
    surface: wgpu::Surface<'a>,
    pub depth_texture: DepthTexture,
    pub device: Device,
    pub queue: Queue,
    pub size: PhysicalSize<u32>,
}

impl <'b: 'a, 'a>Surface<'b, 'a> {
    pub async fn new(window: &'b Window) -> Self {
        let size = window.inner_size();
        let window_id = window.id();
        let instance = wgpu::Instance::new(InstanceDescriptor { 
            backends: Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::POLYGON_MODE_LINE,
                required_limits: wgpu::Limits::default(),
                label: None 
            },
            None,
        ).await.unwrap();
        let config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
        surface.configure(&device, &config);
        let depth_texture = DepthTexture::create_depth_texture(&device, &config, "Depth Texture");
        return Self {
            window_id,
            window,
            config,
            surface,
            device,
            queue,
            depth_texture,
            size,
        }
    }

    pub fn run(mut self, mut handler: impl WindowHandler, event_loop: EventLoop<()>) {
        let config = FullWindowConfig::load_defaults(handler.config());
        event_loop.run(move |event, control_flow| {
            match event {
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
                        WindowEvent::CursorMoved { position, .. } => {
                            handler.mouse_moved(&self.device, *position);
                        }
                        WindowEvent::Resized(physical_size) => {
                            self.config.width = physical_size.width;
                            self.config.height = physical_size.height;
                            handler.resize(&self.device, Vector2::new(self.config.width, self.config.height));
                            self.surface.configure(&self.device, &self.config);
                            self.depth_texture = DepthTexture::create_depth_texture(&self.device, &self.config, "Depth Texture");
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            self.config.width = (self.config.width as f64**scale_factor) as u32;
                            self.config.height = (self.config.height as f64**scale_factor) as u32;
                            handler.resize(&self.device, Vector2::new(self.config.width, self.config.height));
                            self.depth_texture = DepthTexture::create_depth_texture(&self.device, &self.config, "Depth Texture");
                            self.surface.configure(&self.device, &self.config);
                        }
                        WindowEvent::RedrawRequested if window_id == self.window_id => {
                            let output = self.surface.get_current_texture().unwrap();
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder = self
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                });
                            {
                                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Render Pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(config.background_color),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                        view: &self.depth_texture.view,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(1.0),
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    }),
                                });
                                handler.render(&self.device, &mut render_pass);
                            }
                            self.queue.submit([encoder.finish()]);

                            output.present();

                            self.window.request_redraw();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }).unwrap();
    }
}

pub trait WindowHandler {
    fn resize(&mut self, device: &Device, new_size: Vector2<u32>);
    fn render<'a: 'b, 'b>(&'a mut self, device: &Device, render_pass: & mut RenderPass<'b>);
    fn config(&self) -> WindowConfig;
    fn mouse_moved(&mut self, device: &Device, mouse_pos: PhysicalPosition<f64>);
}

pub struct WindowConfig {
    pub background_color: Option<wgpu::Color>,
}

struct FullWindowConfig {
    background_color: wgpu::Color,
}

impl FullWindowConfig {
    fn load_defaults(config: WindowConfig) -> Self {
        Self {
            background_color: config.background_color.unwrap_or(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }),
        }
    }
}