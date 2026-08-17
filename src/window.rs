use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use bytemuck::{bytes_of, NoUninit};
use cgmath::Vector2;
use wgpu::TextureViewDimension::D2;
use wgpu::{Device, Features, InstanceDescriptor, Limits, RenderPass};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, MouseButton, Touch, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersKeyState, PhysicalKey};
use winit::window::{Window, WindowId};
use winit::event_loop::ActiveEventLoop;

use crate::binding::{Binding, Descriptor, UniformBinding, bind_resources, create_layout};
use crate::culling::AABB;
use crate::model::{Model, Render, ToRaw};
use crate::resource_loader::{ResourceType, GLOBAL_PROJECT_RESOURCES};
use crate::shader::{Shader, ShaderConfig, CUSTOM_SHADER_TYPE_SOURCE};
use crate::surface_context::{SurfaceContext, SurfaceCtx};
use crate::texture::{DepthTexture, Texture, TextureLayoutConfig};

pub static MULTISAMPLE_COUNT: Mutex<u32> = Mutex::new(1);

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
        let instance = wgpu::Instance::new(InstanceDescriptor::new_without_display_handle());
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
        let surface_config = H::surface_config();
        *MULTISAMPLE_COUNT.lock().unwrap() = surface_config.multisample_count;
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        let size = window.inner_size();
        if let Some((surface, adapter, device, queue)) = pollster::block_on(async {
            if window.inner_size() == (PhysicalSize { width: 0, height: 0 }) {
                return None;
            }
            let surface = self.instance.create_surface(window.clone()).unwrap();
            let adapter = self.instance.request_adapter(
                &wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                    ..Default::default()
                },
            ).await.unwrap();
            let (device, queue) = adapter.request_device(
                &wgpu::DeviceDescriptor {
                    required_features: H::required_features(),
                    required_limits: H::limits(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    ..Default::default()
                },
            ).await.unwrap();
            return Some((surface, adapter, device, queue));
        }) {
            let mut config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
            if let Some(format) = surface_config.override_format {
                config.format = format;
            }
            surface.configure(&device, &config);
            let depth_texture = DepthTexture::create_depth_texture(&device, config.width, config.height, "Depth Texture", MULTISAMPLE_COUNT.lock().unwrap().clone());
            let depth_texture_binding = UniformBinding::new(&device, "Depth Texture", depth_texture, None);
            let texture_renderer_shader = Shader::new("buildins/screen_renderer.wgsl", &device, vec![config.format], vec![&create_layout::<Texture>(TextureLayoutConfig {dimensions: D2, sample_count: 1 },  &device)], vec![&Texture::shader_type(TextureLayoutConfig {dimensions: D2, sample_count: 1 })], vec![BasicVertex::desc()], ShaderConfig::default());
            let screen_model = BasicVertex::one_face(&device);
            let surface_context = SurfaceContext {
                window_id: window.id(),
                window,
                surface: Arc::new(surface),
                size: size.into(),
                config,
                texture_renderer_shader,
                depth_texture: depth_texture_binding,
                device: Arc::new(device),
                queue: Arc::new(queue),
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
                if self.current_modifiers.lcontrol_state() == ModifiersKeyState::Pressed {
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
            WindowEvent::MouseInput { device_id: _, state, button } => {
                if let Some(surface_context) = &self.surface_context {
                    if let Some(handler) = &mut self.handler {
                        handler.mouse_input(surface_context, state, button);
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
                    let depth_texture = DepthTexture::create_depth_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, "Depth Texture", MULTISAMPLE_COUNT.lock().unwrap().clone());
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
                    let depth_texture = DepthTexture::create_depth_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, "Depth Texture", MULTISAMPLE_COUNT.lock().unwrap().clone());
                    surface_context.depth_texture.replace_data(&surface_context.device, depth_texture);
                    surface_context.surface.configure(&surface_context.device, &surface_context.config);
            }
            }
            WindowEvent::RedrawRequested if self.surface_context.as_ref().map(|ctx| ctx.window_id) == Some(window_id) => {
                if let Some(surface_context) = &self.surface_context {
                    let delta = SystemTime::now().duration_since(self.last_time).unwrap_or(Duration::from_millis(0));
                    self.last_time = SystemTime::now();
                    if let Some(handler) = &mut self.handler {
                        handler.update(surface_context, delta);
                    }
                    let multisample_texture = if MULTISAMPLE_COUNT.lock().unwrap().clone() > 1 {
                        Some(Texture::blank_texture(surface_context.device(), surface_context.config.width, surface_context.config.height, surface_context.config.format, MULTISAMPLE_COUNT.lock().unwrap().clone()))
                    } else {
                        None
                    };
                    let temp_texture = Texture::blank_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, surface_context.config.format, 1);
                    let temp_texture_binding = UniformBinding::new(&surface_context.device, "Temp Texture", temp_texture, None);
                    let output_result = surface_context.surface.get_current_texture();
                    let output = match output_result {
                        wgpu::CurrentSurfaceTexture::Success(texture) => texture,
                        wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                            //TODO: reconfigure surface
                            texture
                        },
                        wgpu::CurrentSurfaceTexture::Occluded => {
                            return
                        },
                        _ => { panic!("bad texture output {output_result:?}") }
                    };
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
                                resolve_target: multisample_texture.as_ref().map(|_| &temp_texture_binding.value.view),
                                view: multisample_texture.as_ref().map(|it| &it.view).unwrap_or(&temp_texture_binding.value.view),
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(self.handler.as_ref().map(|handler| handler.config()).unwrap_or_default().background_color),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            multiview_mask: None,
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
                            handler.render(surface_context, &mut render_pass);
                        }
                    }
                    
                    //create another temporary texture and use it to render post processing effects
                    let post_process_texture = if self.handler.as_ref().map(|handler| handler.config()).unwrap_or_default().enable_post_processing {
                        let post_process_texture = Texture::blank_texture(&surface_context.device, surface_context.config.width, surface_context.config.height, surface_context.config.format, 1);
                        {
                            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Post Processing Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &post_process_texture.view,
                                    resolve_target: None,
                                    depth_slice: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                multiview_mask: None,
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
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            multiview_mask: None,
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

                    surface_context.queue.present(output);
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
pub trait WindowHandler {
    fn resize(&mut self, surface_context: &dyn SurfaceCtx, new_size: Vector2<u32>);
    fn update(&mut self, surface_context: &dyn SurfaceCtx, delta_time: Duration);
    fn render<'a: 'b, 'b>(&'a mut self, surface_context: &'a dyn SurfaceCtx, render_pass: & mut RenderPass<'b>);
    fn config(&self) -> WindowConfig;
    fn surface_config() -> SurfaceConfig;
    fn limits() -> Limits;
    fn required_features() -> Features;
    fn mouse_moved(&mut self, surface_context: &dyn SurfaceCtx, mouse_pos: PhysicalPosition<f64>);
    fn mouse_motion(&mut self, surface_context: &dyn SurfaceCtx, mouse_delta: (f64, f64));
    fn mouse_input(&mut self, surface_context: &dyn SurfaceCtx, element_state: &ElementState, mouse_button: &MouseButton);
    fn input_event(&mut self, surface_context: &dyn SurfaceCtx, input_event: &KeyEvent, current_modifiers: &Modifiers);
    fn touch(&mut self, surface_context: &dyn SurfaceCtx, touch: &Touch);
    fn post_process_render<'a: 'b, 'c: 'b, 'b>(&'a mut self, surface_context: &'c dyn SurfaceCtx, render_pass: & mut RenderPass<'b>, surface_texture: &'c UniformBinding<Texture>);
    fn other_window_event(&mut self, surface_context: &dyn SurfaceCtx, event: &WindowEvent);
    fn custom_shader_type_source() -> String;
    fn resources() -> Option<&'static phf::Map<&'static str, ResourceType>>;
}

pub struct WindowConfig {
    pub background_color: wgpu::Color,
    pub enable_post_processing: bool,
    pub multisample_count: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { 
            background_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }, 
            enable_post_processing: false,
            multisample_count: 1,
        }
    }
}

pub struct SurfaceConfig {
    pub override_format: Option<wgpu::TextureFormat>,
    pub multisample_count: u32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self { override_format: None, multisample_count: 4 }
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