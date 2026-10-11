struct ViewportUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    base_color: vec4<f32>,
    surface: vec4<f32>,
    pigment_detail: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ViewportUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return u.view_projection * vec4<f32>(input.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.20, 0.22, 0.25, 1.0);
}
