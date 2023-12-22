// #import bevy_pbr::mesh_vertex_output MeshVertexOutput
// #import bevy_pbr::mesh_view_bindings globals
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct SpaceMaterial {
    time : f32,
};

@group(1) @binding(0)
var<uniform> material: SpaceMaterial;

@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_color_sampler: sampler;

@group(1) @binding(3)
var noise_texture: texture_2d<f32>;
@group(1) @binding(4)
var noise_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    return textureSample(base_color_texture, base_color_sampler, 
        mesh.uv + textureSample(noise_texture, noise_sampler, mesh.uv).r * sin(globals.time * 2.0) * 0.01
    );
}


