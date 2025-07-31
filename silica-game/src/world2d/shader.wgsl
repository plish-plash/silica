struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) transform0: vec3f,
    @location(1) transform1: vec3f,
    @location(2) uv: vec4f,
    @location(3) color: vec4f,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
}

struct Params {
    view_matrix: mat3x2f,
    screen_resolution: vec2f,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var tex: texture_2d<f32>;

@group(1) @binding(1)
var sam: sampler;

@vertex
fn vs_main(in_vert: VertexInput) -> VertexOutput {
    let corner_position = vec2<f32>(vec2<u32>(
        in_vert.vertex_idx & 1u,
        (in_vert.vertex_idx >> 1u) & 1u,
    ));
    let view_transform = mat3x3f(vec3f(params.view_matrix[0], 0.0), vec3f(params.view_matrix[1], 0.0), vec3f(params.view_matrix[2], 1.0));
    let model_transform = mat3x3f(vec3f(in_vert.transform0.xy, 0.0), vec3f(in_vert.transform0.z, in_vert.transform1.x, 0.0), vec3f(in_vert.transform1.yz, 1.0));
    let pos = view_transform * model_transform * vec3f(corner_position, 1.0);
    let uv = mix(in_vert.uv.xy, in_vert.uv.zw, corner_position);

    var out_vert: VertexOutput;
    out_vert.position = vec4f(2.0 * pos.xy / params.screen_resolution - 1.0, 0.0, 1.0);
    out_vert.position.y *= -1.0;
    out_vert.uv = uv;
    out_vert.color = in_vert.color;
    return out_vert;
}


@fragment
fn fs_main(in_frag: VertexOutput) -> @location(0) vec4f {
    return in_frag.color * textureSampleLevel(tex, sam, in_frag.uv, 0.0);
}
