struct AABB {
    dimensions: vec3f,
};

input_instances: $0,0;
output_instances: $0,1;
@group(1) @binding(0)
var<storage,read_write> output_i: atomic<u32>;
camera: $2;
num_instances: $3;
bounding_box: $4;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x + global_id.y * 65535 + global_id.z * 65535 * 65535;
    if i < num_instances {
        var instance = input_instances[i];
        let instance_matrix = instance.***INSTANCE_MATRIX***;
        var any_shown = false;
        let dimensions = bounding_box.dimensions;
        let bounding_box_corners = array(
            multiply_vec3f(dimensions, vec3(1.0, 1.0, 1.0)),
            multiply_vec3f(dimensions, vec3(1.0, 1.0, -1.0)),
            multiply_vec3f(dimensions, vec3(1.0, -1.0, 1.0)),
            multiply_vec3f(dimensions, vec3(1.0, -1.0, -1.0)),
            multiply_vec3f(dimensions, vec3(-1.0, 1.0, 1.0)),
            multiply_vec3f(dimensions, vec3(-1.0, 1.0, -1.0)),
            multiply_vec3f(dimensions, vec3(-1.0, -1.0, 1.0)),
            multiply_vec3f(dimensions, vec3(-1.0, -1.0, -1.0))
        );
        for (var i = 0; i < 8; i++) {
            let clip_position = camera.view_proj * instance_matrix * vec4f(bounding_box_corners[i], 1.0);
            let screen_pos = clip_position.xyz / clip_position.w;
            if screen_pos.x >= -1.0 && screen_pos.x <= 1.0 && screen_pos.y >= -1.0 && screen_pos.y <= 1.0 && clip_position.z >= 0.1 && clip_position.z <= 100.0 /*&& distance(instance_pos.xyz-camera.position, vec3f(0.0)) < 100.0*/ {
                any_shown = true;
                break;
            }
        }
        if any_shown {
            let loaded_output_i = atomicAdd(&output_i, 1u);
            output_instances[loaded_output_i] = instance;
        }
    }
}

fn multiply_vec3f(x: vec3f, y: vec3f) -> vec3f {
    return vec3f(x.x * y.x, x.y * y.y, x.z * y.z);
}