struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] offset: vec2<f32>;
    [[location(2)]] size: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
};

struct Camera {
    view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> c: Camera;

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = c.view_proj * vec4<f32>(in.position * in.size + in.offset, 0.0, 1.0);
    return output;
}

[[stage(fragment)]]
fn fs_main(input: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.0, 1.0, 1.0);
}
