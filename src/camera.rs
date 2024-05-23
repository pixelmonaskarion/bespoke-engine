use cgmath::{Matrix4, Point3, Transform, Vector3, Vector4};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

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
        return OPENGL_TO_WGPU_MATRIX * proj * view;
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
}

pub fn vec_to_point<T>(vec: Vector3<T>) -> Point3<T> {
    Point3::new(vec.x, vec.y, vec.z)
}