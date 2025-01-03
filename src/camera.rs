use bytemuck::{bytes_of, NoUninit};
use cgmath::{Matrix4, Point3, Transform, Vector3, Vector4};

use crate::{binding::{simple_layout_entry, Binding, Resource}, shader::ShaderType};

#[derive(Clone)]
pub struct Camera {
        pub eye: cgmath::Vector3<f32>,
        pub aspect: f32,
        pub fovy: f32,
        pub znear: f32,
        pub zfar: f32,

        pub ground: f32,
        pub sky: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(vec_to_point(self.eye), vec_to_point(self.eye+self.get_forward_vec()), Vector3::unit_y());
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return proj * view;
    }

    pub fn build_view_projection_matrix_raw(&self) -> [[f32; 4]; 4] {
        return self.build_view_projection_matrix().into();
    }

    pub fn build_inverse_matrix(&self) -> Matrix4<f32> {
        return self.build_view_projection_matrix().inverse_transform().unwrap();
    }

    pub fn build_inverse_matrix_raw(&self) -> [[f32; 4]; 4] {
        return self.build_inverse_matrix().into();
    }

    pub fn point_visible(&self, point: Vector3<f32>) -> bool {
        let screen_space_4 = self.build_view_projection_matrix()*Vector4::new(point.x, point.y, point.z, 1.0);
        let screen_space = screen_space_4.truncate()/screen_space_4.w;
        screen_space.x < -1.0 || screen_space.x > 1.0 || screen_space.y < -1.0 || screen_space.y > 1.0
    }

    pub fn get_forward_vec(&self) -> Vector3<f32> {
        cgmath::Vector3::new(self.ground.cos()*self.sky.cos(), self.sky.sin(), self.ground.sin()*self.sky.cos())
    }

    pub fn get_walking_vec(&self) -> Vector3<f32> {
        cgmath::Vector3::new(self.ground.cos(), 0.0, self.ground.sin())
    }

    pub fn get_right_vec(&self) -> Vector3<f32> {
        cgmath::Vector3::new(self.ground.cos(), 0.0, self.ground.sin()).cross(Vector3::unit_y())
    }

    pub fn to_raw(&self) -> CameraRaw {
        // let mut data = bytes_of().to_vec();
        // data.append(&mut bytes_of(&self.build_inverse_matrix_raw()).to_vec());
        // data.append(&mut bytes_of(&Into::<[f32; 3]>::into(self.eye)).to_vec());
        // data.append(&mut bytes_of(&0_f32).to_vec()); //padding
        // data.try_into().unwrap()
        CameraRaw {
            view_proj: self.build_view_projection_matrix_raw(),
            inverse_view_proj: self.build_inverse_matrix_raw(),
            eye: self.eye.into(),
            padding: 0.0,
        }
    }
}

pub fn vec_to_point<T>(vec: Vector3<T>) -> Point3<T> {
    Point3::new(vec.x, vec.y, vec.z)
}

impl Binding for Camera {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![simple_layout_entry(0)]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        let raw = self.to_raw();
        vec![
            Resource::Simple(bytes_of(&raw).to_vec()),
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["<uniform>".into()],
            wgsl_types: vec!["Camera".into()],
        }
    }
}

#[derive(Clone)]
pub struct TargetCamera {
        pub eye: cgmath::Vector3<f32>,
        pub aspect: f32,
        pub fovy: f32,
        pub znear: f32,
        pub zfar: f32,

        pub target: cgmath::Vector3<f32>,
}

impl TargetCamera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(vec_to_point(self.eye), vec_to_point(self.target), Vector3::unit_y());
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return proj * view;
    }

    pub fn build_view_projection_matrix_raw(&self) -> [[f32; 4]; 4] {
        return self.build_view_projection_matrix().into();
    }

    pub fn build_inverse_matrix(&self) -> Matrix4<f32> {
        return self.build_view_projection_matrix().inverse_transform().unwrap();
    }

    pub fn build_inverse_matrix_raw(&self) -> [[f32; 4]; 4] {
        return self.build_inverse_matrix().into();
    }

    pub fn point_visible(&self, point: Vector3<f32>) -> bool {
        let screen_space_4 = self.build_view_projection_matrix()*Vector4::new(point.x, point.y, point.z, 1.0);
        let screen_space = screen_space_4.truncate()/screen_space_4.w;
        screen_space.x < -1.0 || screen_space.x > 1.0 || screen_space.y < -1.0 || screen_space.y > 1.0
    }

    pub fn to_raw(&self) -> CameraRaw {
        CameraRaw {
            view_proj: self.build_view_projection_matrix_raw(),
            inverse_view_proj: self.build_inverse_matrix_raw(),
            eye: self.eye.into(),
            padding: 0.0,
        }
    }
}

impl Binding for TargetCamera {
    fn layout(_ty: Option<wgpu::BindingType>) -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![simple_layout_entry(0)]
    }

    fn create_resources<'a>(&'a self) -> Vec<Resource> {
        let raw = self.to_raw();
        vec![
            Resource::Simple(bytes_of(&raw).to_vec()),
        ]
    }

    fn shader_type() -> ShaderType {
        ShaderType {
            var_types: vec!["<uniform>".into()],
            wgsl_types: vec!["Camera".into()],
        }
    }
}

#[derive(NoUninit, Clone, Copy)]
#[repr(C)]
pub struct CameraRaw {
    pub view_proj: [[f32; 4]; 4],
    pub inverse_view_proj: [[f32; 4]; 4],
    pub eye: [f32; 3],
    pub padding: f32,
}