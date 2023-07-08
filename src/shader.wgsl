struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>
}

struct Camera {
    transform: mat4x4<f32>
}
@group(0) 
@binding(0)
var<uniform> camera: Camera;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.position = camera.transform * in.position;
    return result;
}

struct TriangleUniform {
    color: vec4<f32>
}
@group(0) 
@binding(1)
var<uniform> triangle_data: TriangleUniform;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(triangle_data.color);
}