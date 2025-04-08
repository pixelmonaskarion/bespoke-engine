use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytemuck::{bytes_of, NoUninit};
use cgmath::Vector2;
use wgpu::{Backends, Device, Features, InstanceDescriptor, Limits, RenderPass};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, Touch, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersKeyState, PhysicalKey};
use winit::window::{Window, WindowId};
use winit::event_loop::ActiveEventLoop;

use crate::binding::{bind_resources, create_layout, Binding, Descriptor, UniformBinding};
use crate::culling::AABB;
use crate::model::{Model, Render, ToRaw};
use crate::resource_loader::{ResourceType, GLOBAL_PROJECT_RESOURCES};
use crate::shader::{Shader, ShaderConfig, CUSTOM_SHADER_TYPE_SOURCE};
use crate::surface_context::{SurfaceContext, SurfaceCtx};
use crate::texture::{DepthTexture, Texture};

pub struct Surface<'b: 'a, 'a, H: WindowHandler> {
    pub instance: wgpu::Instance,
    pub surface_context: Option<SurfaceContext<'a>>,
    pub mouse_pos: [f64; 2],
    pub current_modifiers: Modifiers,
    pub last_time: SystemTime,
    pub handler: Option<H>,
    pub ready: &'b dyn Fn(&dyn SurfaceCtx) -> H,
}

impl <'b: 'a, 'a, H: WindowHandler> Surface<'b, 'a, H> {
    pub async fn new(ready: &'b dyn Fn(&dyn SurfaceCtx) -> H) -> Self {
        let instance = wgpu::Instance::new(&InstanceDescriptor { 
            backends: Backends::all(),
            ..Default::default()
        });
        *GLOBAL_PROJECT_RESOURCES.lock().unwrap() = H::resources();
        *CUSTOM_SHADER_TYPE_SOURCE.lock().unwrap() = H::custom_shader_type_source();
        return Self {
            // window: None,
            instance,
            surface_context: None,
            current_modifiers: Modifiers::default(),
            mouse_pos: [0.0, 0.0],
            last_time: SystemTime::now(),
            handler: None,
            ready,
        }
    }
}

impl<'b: 'a, 'a, H: WindowHandler> ApplicationHandler for Surface<'b, 'a, H> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let surface_config = FullSurfaceConfig::load_defaults(H::surface_config());
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        let size = window.inner_size();
        if let Some((surface, adapter, device, queue)) = pollster::block_on(async {
            if window.inner_size() == (PhysicalSize { width: 0, height: 0 }) {
                return None;
            }
            let surface = self.instance.create_surface(window.clone()).unwrap();
            let adapter = self.instance.request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                },
            ).await.unwrap();
            let (device, queue) = adapter.request_device(
                &wgpu::DeviceDescriptor {
                    required_features: H::required_features(),
                    required_limits: H::limits(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            ).await.unwrap();
            return Some((surface, adapter, device, queue));
        }) {
            let mut config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
            if let Some(format) = surface_config.override_format {
                config.format = format;
            }
            surface.configure(&device, &config);
            let depth_texture = DepthTexture::create_depth_texture(&device, config.width, config.height, "Depth Texture");
            let depth_texture_binding = UniformBinding::new(&device, "Depth Texture", depth_texture, None);
            let texture_renderer_shader = Shader::new("buildins/screen_renderer.wgsl", &device, vec![config.format], vec![&create_layout::<Texture>(&device)], vec![&Texture::shader_type()], vec![BasicVertex::desc()], ShaderConfig::default());
            let screen_model = BasicVertex::one_face(&device);
            let surface_context = SurfaceContext {
                window_id: window.id(),
                window,
                surface: Arc::new(surface),
                size: size.into(),
                config,
                texture_renderer_shader,
                depth_texture: depth_texture_binding,
                device,
                queue,
                screen_model,
            };
            self.surface_context = Some(surface_context);
            self.handler = Some((self.ready)(self.surface_context.as_ref().unwrap()));
        }
    }

    fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: DeviceEvent,
        ) {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    self.mouse_pos[0] += delta.0;
                    self.mouse_pos[1] += delta.1;
                    if let Some(surface_context) = &self.surface_context {
                        if let Some(handler) = &mut self.handler {
                            handler.mouse_motion(surface_context, delta);
                            // handler.mouse_moved(surface_context, PhysicalPosition { x: self.mouse_pos[0], y: self.mouse_pos[1] });
                        }
                    }
                }
                _ => {}
            }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.surface_context.as_ref().map(|ctx| ctx.window_id) == Some(window_id) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                if self.current_modifiers.lsuper_state() == ModifiersKeyState::Pressed {
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(surface_context) = &self.surface_context {
                    if let Some(handler) = &mut self.handler {
                        handler.input_event(surface_context, &event, &self.current_modifiers);
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.current_modifiers = *modifiers;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(surface_context) = &self.surface_context {
                    if let Some(handler) = &mut self.handler {
                        handler.mouse_moved(surface_context, *position);
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                if let Some(surface_context) = &mut self.surface_context {
                    if let Some(handler) = &mut self.handler {
                        handler.touch(surface_context, &touch);
                    }
                }
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(surface_context) = &mut self.surface_context {
                    surface_context.config.width = physical_size.width;
                    surface_context.config.height = physical_size.height;
                    if let Some(handler) = &mut self.handler {
                        handler.resize(surface_context, Vector2::new(surface_context.config.width, surface_context.config.height));
                    }
                    surface_context.surface.configure(&surface_context.device, &surface_context.config);
                    let depth_texture = DepthTexture::create_depth_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, "Depth Texture");
                    surface_context.depth_texture.replace_data(&surface_context.device, depth_texture);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(surface_context) = &mut self.surface_context {
                    surface_context.config.width = (surface_context.config.width as f64*scale_factor) as u32;
                    surface_context.config.height = (surface_context.config.height as f64*scale_factor) as u32;
                    if let Some(handler) = &mut self.handler {
                        handler.resize(surface_context, Vector2::new(surface_context.config.width, surface_context.config.height));
                    }
                    let depth_texture = DepthTexture::create_depth_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, "Depth Texture");
                    surface_context.depth_texture.replace_data(&surface_context.device, depth_texture);
                    surface_context.surface.configure(&surface_context.device, &surface_context.config);
            }
            }
            WindowEvent::RedrawRequested if self.surface_context.as_ref().map(|ctx| ctx.window_id) == Some(window_id) => {
                if let Some(surface_context) = &self.surface_context {
                    let delta = SystemTime::now().duration_since(self.last_time).unwrap_or(Duration::from_millis(0)).as_nanos() as f64 / 1000000.0;
                    self.last_time = SystemTime::now();
                    let temp_texture = Texture::blank_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, surface_context.config.format);
                    let temp_texture_binding = UniformBinding::new(&surface_context.device, "Temp Texture", temp_texture, None);
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
                                view: &temp_texture_binding.value.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(FullWindowConfig::load_defaults(self.handler.as_ref().map(|handler| handler.config()).flatten()).background_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: &surface_context.depth_texture.value.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                        });
                        if let Some(handler) = &mut self.handler {
                            handler.render(surface_context, &mut render_pass, delta);
                        }
                    }
                    
                    //create another temporary texture and use it to render post processing effects
                    let post_process_texture = if FullWindowConfig::load_defaults(self.handler.as_ref().map(|handler| handler.config()).flatten()).enable_post_processing {
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
                                // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                //     view: &surface_context.depth_texture.view,
                                //     depth_ops: Some(wgpu::Operations {
                                //         load: wgpu::LoadOp::Clear(1.0),
                                //         store: wgpu::StoreOp::Store,
                                //     }),
                                //     stencil_ops: None,
                                // }),
                                depth_stencil_attachment: None,
                            });
                            if let Some(handler) = &mut self.handler {
                                handler.post_process_render(surface_context, &mut render_pass, &temp_texture_binding);
                            }
                        }
                        post_process_texture
                    } else {
                        temp_texture_binding.value
                    };
                    let post_process_texture_binding = bind_resources(&post_process_texture, &surface_context.device);
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
                                view: &surface_context.depth_texture.value.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                        });
                        surface_context.texture_renderer_shader.bind(&mut render_pass);
                        render_pass.set_bind_group(0, &post_process_texture_binding, &[]);

                        surface_context.screen_model.render(&mut render_pass);
                    }
                    surface_context.queue.submit([encoder.finish()]);

                    output.present();
                }
            }
            _ => {}
        }
        if let Some(handler) = &mut self.handler {
            if let Some(surface_context) = &self.surface_context {
                handler.other_window_event(surface_context, &event);
            }
        }
    }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.surface_context.as_ref().map(|ctx| &ctx.window) {
            window.request_redraw();
        }
    }
}

pub fn render_texture(surface_context: &dyn SurfaceCtx, render: &dyn Fn(&mut RenderPass)) {
    let render_texture = Texture::blank_texture(surface_context.device(), surface_context.config().width, surface_context.config().height, surface_context.config().format);
    let mut encoder = surface_context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Surface Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            timestamp_writes: None,
            occlusion_query_set: None,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &surface_context.depth_texture().value.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
        });
        render(&mut render_pass);
    }
    surface_context.queue().submit([encoder.finish()]);
}

pub trait WindowHandler {
    fn resize(&mut self, surface_context: &dyn SurfaceCtx, new_size: Vector2<u32>);
    fn render<'a: 'b, 'b>(&'a mut self, surface_context: &'a dyn SurfaceCtx, render_pass: & mut RenderPass<'b>, delta: f64);
    fn config(&self) -> Option<WindowConfig>;
    fn surface_config() -> Option<SurfaceConfig>;
    fn limits() -> Limits;
    fn required_features() -> Features;
    fn mouse_moved(&mut self, surface_context: &dyn SurfaceCtx, mouse_pos: PhysicalPosition<f64>);
    fn mouse_motion(&mut self, surface_context: &dyn SurfaceCtx, mouse_delta: (f64, f64));
    fn input_event(&mut self, surface_context: &dyn SurfaceCtx, input_event: &KeyEvent, current_modifiers: &Modifiers);
    fn touch(&mut self, surface_context: &dyn SurfaceCtx, touch: &Touch);
    fn post_process_render<'a: 'b, 'c: 'b, 'b>(&'a mut self, surface_context: &'c dyn SurfaceCtx, render_pass: & mut RenderPass<'b>, surface_texture: &'c UniformBinding<Texture>);
    fn other_window_event(&mut self, surface_context: &dyn SurfaceCtx, event: &WindowEvent);
    fn custom_shader_type_source() -> String;
    fn resources() -> Option<&'static phf::Map<&'static str, ResourceType>>;
}

pub struct WindowConfig {
    pub background_color: Option<wgpu::Color>,
    pub enable_post_processing: Option<bool>,
}

struct FullWindowConfig {
    background_color: wgpu::Color,
    enable_post_processing: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { background_color: None, enable_post_processing: None }
    }
}

impl FullWindowConfig {
    fn load_defaults(config: Option<WindowConfig>) -> Self {
        Self {
            background_color: config.as_ref().map(|c| c.background_color).flatten().unwrap_or(wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }),
            enable_post_processing: config.as_ref().map(|c| c.enable_post_processing).flatten().unwrap_or(false),
        }
    }
}

pub struct SurfaceConfig {
    pub override_format: Option<wgpu::TextureFormat>,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self { override_format: None }
    }
}

struct FullSurfaceConfig {
    override_format: Option<wgpu::TextureFormat>,
}

impl FullSurfaceConfig {
    fn load_defaults(config: Option<SurfaceConfig>) -> Self {
        Self {
            override_format: config.as_ref().map(|c| c.override_format).flatten(),
        }
    }
}


#[repr(C)]
#[derive(NoUninit, Copy, Clone, Default, Debug)]
pub struct BasicVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl BasicVertex {
    pub fn one_face(device: &Device) -> Model {
        Model::new(vec![
            Self { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
            Self { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0] },
            Self { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] },
            Self { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0] },
        ], &[0_u16, 2, 1, 2, 3, 1], AABB { dimensions: [1.0, 1.0, 0.0] }, device)
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