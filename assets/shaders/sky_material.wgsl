#import bevy_pbr::mesh_functions
#import bevy_pbr::view_transformations::position_world_to_clip

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var sky_texture: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var sky_sampler: sampler;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec3<f32>,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(input.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(input.position, 1.0),
    );
    out.clip_position = position_world_to_clip(world_position.xyz);
    // Use model-rotated direction so entity rotation affects cubemap sampling
    let world_dir = (world_from_local * vec4<f32>(input.position, 0.0)).xyz;
    out.uv = normalize(world_dir);
    return out;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the cubemap
    // We might need to flip Y or Z depending on the texture layout
    let color = textureSample(sky_texture, sky_sampler, normalize(input.uv));
    return color;
}
