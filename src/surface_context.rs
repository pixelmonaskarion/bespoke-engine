use std::sync::Arc;

use wgpu::{Device, Queue, SurfaceConfiguration};
use winit::window::{Window, WindowId};

use crate::{binding::UniformBinding, model::Model, shader::Shader, texture::DepthTexture};

pub struct SurfaceContext<'a> {
    pub surface: Arc<wgpu::Surface<'a>>,
    pub config: SurfaceConfiguration,
    // pub depth_texture: Texture,
    pub depth_texture: UniformBinding<DepthTexture>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub screen_model: Model,
    pub texture_renderer_shader: Shader<'a>,
    pub window_id: WindowId,
    pub size: (u32, u32),
    pub window: Arc<Window>,
}

impl SurfaceCtx for SurfaceContext<'_> {
    fn surface(&self) -> &wgpu::Surface<'_> {
        &self.surface
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    fn depth_texture(&self) -> &UniformBinding<DepthTexture> {
        &self.depth_texture
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn queue(&self) -> &Queue {
        &self.queue
    }

    fn device_arc(&self) -> Arc<Device> {
        self.device.clone()
    }

    fn queue_arc(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    fn screen_model(&self) -> &Model {
        &self.screen_model
    }

    fn texture_renderer_shader(&self) -> &Shader<'_> {
        &self.texture_renderer_shader
    }

    fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn window(&self) -> &Window {
        &self.window
    }
} 

pub trait SurfaceCtx {
    fn surface(&'_ self) -> &'_ wgpu::Surface<'_>;
    fn config(&self) -> &SurfaceConfiguration;
    fn depth_texture(&self) -> &UniformBinding<DepthTexture>;
    fn device(&self) -> &Device;
    fn queue(&self) -> &Queue;
    fn device_arc(&self) -> Arc<Device>;
    fn queue_arc(&self) -> Arc<Queue>;
    fn screen_model(&self) -> &Model;
    fn texture_renderer_shader(&self) -> &Shader<'_>;
    fn window_id(&self) -> &WindowId;
    fn size(&self) -> (u32, u32);
    fn window(&self) -> &Window;
}