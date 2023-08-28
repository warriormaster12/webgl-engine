struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct Camera {
    vp_matrix: mat4x4<f32>,
}
@group(0)
@binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.position = camera.vp_matrix * position;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

