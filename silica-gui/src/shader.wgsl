struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) rect: vec4i,
    @location(1) uv: vec4f,
    @location(2) color: vec4f,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
    @location(1) color: vec4f,
}

struct Params {
    screen_resolution: vec2u,
    _pad: vec2u,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(1) @binding(0)
var tex: texture_2d<f32>;

@group(1) @binding(1)
var sam: sampler;

@vertex
fn vs_main(in_vert: VertexInput) -> VertexOutput {
    let corner_position = vec2f(vec2u(
        in_vert.vertex_idx & 1u,
        (in_vert.vertex_idx >> 1u) & 1u,
    ));
    let pos = mix(vec2f(in_vert.rect.xy), vec2f(in_vert.rect.zw), corner_position);
    let uv = mix(in_vert.uv.xy, in_vert.uv.zw, corner_position);

    var out_vert: VertexOutput;
    out_vert.position = vec4f(2.0 * pos / vec2f(params.screen_resolution) - 1.0, 0.0, 1.0);
    out_vert.position.y *= -1.0;
    out_vert.uv = uv;
    out_vert.color = in_vert.color;
    return out_vert;
}

@fragment
fn fs_main(in_frag: VertexOutput) -> @location(0) vec4f {
    if in_frag.uv.x < -1.0 {
        return in_frag.color;
    } else {
        return in_frag.color * textureSampleLevel(tex, sam, in_frag.uv, 0.0);
    }
}
