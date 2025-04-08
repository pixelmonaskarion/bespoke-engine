use crate::{binding::{create_layout, Binding, UniformBinding, WgslType}, camera::Camera, compute::{ComputeOutput, ComputeShader}, model::Model, shader::ShaderType};
use bytemuck::{Pod, Zeroable};
use cgmath::{vec3, Matrix4, Vector3};
use wgpu::{BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, Buffer, Device, Queue};

pub struct CullingCompute {
    shader: ComputeShader,
    buffers_layout: BindGroupLayout,
    num_instances_uniform: UniformBinding<u32>,
    bounding_box_uniform: UniformBinding<AABB>,
}

impl CullingCompute {
    pub fn new(instance_struct_definition: &str, instance_matrix_identifier: &str, device: &Device) -> Self {
        let buffers_layout = 
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }, wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }]
            });
        let output_i_buffer = ComputeOutput::new(4, device);
        let num_instances_uniform = UniformBinding::new(device, "Num Instances", 0, None);
        let bounding_box_uniform = UniformBinding::new(device, "Num Instances", AABB { dimensions: [0.0; 3] }, None);
        let shader = ComputeShader::new(&format!("{instance_struct_definition}\n{}", include_str!("culling.wgsl")).replace("***INSTANCE_MATRIX***", instance_matrix_identifier), &[&buffers_layout, &output_i_buffer.layout, &create_layout::<Camera>(device), &create_layout::<u32>(device), &bounding_box_uniform.layout], vec![&ShaderType::multi_buffer_type(vec![false, true], vec!["Instance".into(); 2]), &u32::shader_type(), &Camera::shader_type(), &u32::shader_type(), &bounding_box_uniform.shader_type], device);
        Self {
            shader,
            buffers_layout,
            num_instances_uniform,
            bounding_box_uniform,
        }
    }

    pub fn run(&mut self, input_buffer: &Buffer, num_instances: u32, bounding_box: &AABB, camera: &UniformBinding<Camera>, device: &Device, queue: &Queue) -> (Buffer, u32) {
        let output_buffer =
            device.create_buffer(&wgpu::BufferDescriptor {
                size: input_buffer.size(),
                label: Some(&format!("Culled Output Instance Buffer")),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            });
        let buffer_binding = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.buffers_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            }, BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            }]
        });
        self.bounding_box_uniform.set_data(queue, *bounding_box);
        self.num_instances_uniform.set_data(queue, num_instances);
        let output_i_buffer = ComputeOutput::new(4, device);
        let groups = [65535, num_instances / 65535 + 1, 1];
        self.shader.run_once(vec![&buffer_binding, &output_i_buffer.binding, &camera.binding, &self.num_instances_uniform.binding, &self.bounding_box_uniform.binding], groups, device, queue);
        let output_i = output_i_buffer.read(device, queue);
        (output_buffer, *bytemuck::from_bytes(&output_i))
    }
}

pub fn culled(model: &Model, instance_transform: Matrix4<f32>, camera: &Camera) -> bool {
    let dimensions: Vector3<f32> = model.bounding_box.dimensions.into();
    let box_corners: [Vector3<f32>; 8] = [
        multiply_vec3f(dimensions, vec3(1.0, 1.0, 1.0)),
        multiply_vec3f(dimensions, vec3(1.0, 1.0, -1.0)),
        multiply_vec3f(dimensions, vec3(1.0, -1.0, 1.0)),
        multiply_vec3f(dimensions, vec3(1.0, -1.0, -1.0)),
        multiply_vec3f(dimensions, vec3(-1.0, 1.0, 1.0)),
        multiply_vec3f(dimensions, vec3(-1.0, 1.0, -1.0)),
        multiply_vec3f(dimensions, vec3(-1.0, -1.0, 1.0)),
        multiply_vec3f(dimensions, vec3(-1.0, -1.0, -1.0)),
    ];
    for corner in box_corners {
        let camera_space_point = camera.build_view_projection_matrix() * instance_transform * corner.extend(1.0);
        let screen_pos = camera_space_point.truncate() / camera_space_point.w;
        if screen_pos.x >= -1.0 && screen_pos.x <= 1.0 && screen_pos.y >= -1.0 && screen_pos.y <= 1.0 && camera_space_point.z >= camera.znear && camera_space_point.z <= camera.zfar {
            return false;
        }
    }
    true
}

fn multiply_vec3f(x: Vector3<f32>, y: Vector3<f32>) -> Vector3<f32> {
    vec3(x.x * y.x, x.y * y.y, x.z * y.z)
}

#[derive(Debug, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct AABB {
    pub dimensions: [f32; 3],
}

impl AABB {
    pub fn zero() -> Self {
        AABB { dimensions: [0.; 3] }
    }
}

impl WgslType for AABB {
    fn wgsl_name() -> String {
        "AABB".into()
    }
}