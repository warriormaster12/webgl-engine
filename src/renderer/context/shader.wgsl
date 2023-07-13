struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}

struct Camera {
    transform: mat4x4<f32>
}
@group(0) 
@binding(0)
var<uniform> camera: Camera;

struct Mesh {
    model_mx: mat4x4<f32>
}

@group(2)
@binding(0)
var<uniform> mesh: Mesh;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var result: VertexOutput;
    result.position = camera.transform * mesh.model_mx * in.position;
    result.tex_coord = in.tex_coord;
    return result;
}

struct Material {
    color: vec4<f32>
}
@group(1) 
@binding(0)
var<uniform> material_data: Material;

// @group(1)
// @binding(1)
// var albedo_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //let a_tex = textureLoad(albedo_texture, vec2<i32>(in.tex_coord * 256.0), 0);
    return vec4<f32>(material_data.color);
}