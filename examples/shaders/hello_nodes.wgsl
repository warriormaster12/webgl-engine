struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct Camera {
    vp_matrix: mat4x4<f32>,
}
@group(0)
@binding(0)
var<uniform> camera: Camera;

struct Mesh {
    model_matrix: mat4x4<f32>,
}
@group(0)
@binding(1)
var<storage> meshes: Mesh;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = camera.vp_matrix * meshes.model_matrix * vec4<f32>(position.x, position.y, position.z, 1.0);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

