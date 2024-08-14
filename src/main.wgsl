struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tex_id: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tex_id: u32,
};

struct FrameData {
    projview: mat4x4<f32>,
};
 // 1.
@group(0) @binding(0) var<uniform> frame_data: FrameData;

@group(1) @binding(0) var textures: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = frame_data.projview*vec4<f32>(model.position, 1.0);
    out.uv = model.uv;
    out.tex_id = model.tex_id;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //return vec4<f32>(in.uv, 1.0, 1.0);
    return textureSample(textures, texture_sampler, in.uv, 0);
}