use cgmath::Vector3;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera {
        pub eye: cgmath::Point3<f32>,
        pub aspect: f32,
        pub fovy: f32,
        pub znear: f32,
        pub zfar: f32,

        pub ground: f32,
        pub sky: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> [[f32; 4]; 4] {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.eye+self.get_forward_vec(), Vector3::unit_y());
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return (OPENGL_TO_WGPU_MATRIX * proj * view).into();
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
}