pub mod model;
pub mod binding;
pub mod shader;
pub mod texture;
pub mod window;
pub mod camera;
pub mod compute;
pub mod mesh;
pub mod resource_loader;
pub mod billboard;
pub mod instance;
pub mod surface_context;
pub mod culling;

pub trait VertexTrait {
    fn pos(&self) -> cgmath::Vector3<f32>;
}

pub trait InstanceTrait {
    fn instance_transform(&self) -> cgmath::Matrix4<f32>;
}