#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::VertexOutput
#import "shaders/CaveMaterialExtension/types_prepass.wgsl"::CaveVertexOutput
#else
#import bevy_pbr::forward_io::VertexOutput
#import "shaders/CaveMaterialExtension/types_forward.wgsl"::CaveVertexOutput
#endif

#import "shaders/CaveMaterialExtension/types.wgsl"::VoxelMaterialOutput

fn ease_in_out_sine(x: f32) -> f32 {
    return -(cos(3.1415927 * x) - 1.0) / 2.0;
}

fn ease_in_out_sine_3d(v: vec3f) -> vec3f {
    return vec3(
        ease_in_out_sine(v.x),
        ease_in_out_sine(v.y),
        ease_in_out_sine(v.z),
    );
}

fn quantize(v: f32, steps: f32) -> f32 {
    return round(v * steps) / steps;
}

fn quantize_3d(v: vec3<f32>, steps: f32) -> vec3<f32> {
    return round(v * steps) / steps;
}

fn clamp_scaled(v: f32, min: f32, max: f32) -> f32 {
    return v * (max - min) + min;
}

fn mix_three(a: f32, b: f32, c: f32, fac: vec3f) -> f32 {
    return a * fac.x + b * fac.y + c * fac.z;
}

fn mix_three_3d(a: vec3f, b: vec3f, c: vec3f, fac: vec3f) -> vec3f {
    return vec3(
        a.r * fac.x + b.r * fac.y + c.r * fac.z,
        a.g * fac.x + b.g * fac.y + c.g * fac.z,
        a.b * fac.x + b.b * fac.y + c.b * fac.z,
    );
}

fn mix_three_4d(a: vec4f, b: vec4f, c: vec4f, fac: vec3f) -> vec4f {
    return vec4(
        a.r * fac.x + b.r * fac.y + c.r * fac.z,
        a.g * fac.x + b.g * fac.y + c.g * fac.z,
        a.b * fac.x + b.b * fac.y + c.b * fac.z,
        a.a * fac.x + b.a * fac.y + c.a * fac.z,
    );
}

fn mix_two_voxels(
    a: VoxelMaterialOutput,
    b: VoxelMaterialOutput,
    fac: f32,
) -> VoxelMaterialOutput {
    var out: VoxelMaterialOutput;
    out.base_color = mix(a.base_color, b.base_color, fac);
    out.reflectance = mix(a.reflectance, b.reflectance, fac);
    out.emissive = mix(a.emissive, b.emissive, fac);
    return out;
}

fn mix_three_voxels(
    a: VoxelMaterialOutput,
    b: VoxelMaterialOutput,
    c: VoxelMaterialOutput,
    fac: vec3f,
) -> VoxelMaterialOutput {
    var out: VoxelMaterialOutput;
    out.base_color = mix_three_3d(a.base_color, b.base_color, c.base_color, fac);
    out.reflectance = mix_three(a.reflectance, b.reflectance, c.reflectance, fac);
    out.emissive = mix_three_4d(a.emissive, b.emissive, c.emissive, fac);
    return out;
}

fn reconstruct_pbr_vertex(in: CaveVertexOutput) -> VertexOutput {
#ifdef PREPASS_PIPELINE
    return VertexOutput(
        in.position,
#ifdef VERTEX_UVS_A
        in.uv,
#endif
#ifdef VERTEX_UVS_B
        in.uv_b,
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
        in.world_normal,
#ifdef VERTEX_TANGENTS
        in.world_tangent,
#endif
#endif
        in.world_position,
#ifdef MOTION_VECTOR_PREPASS
        in.previous_world_position,
#endif
#ifdef DEPTH_CLAMP_ORTHO
        in.clip_position_unclamped,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        in.instance_index,
#endif
#ifdef VERTEX_COLORS
        in.color,
#endif
    );
#else
    return VertexOutput(
        in.position,
        in.world_position,
        in.world_normal,
#ifdef VERTEX_UVS_A
        in.uv,
#endif
#ifdef VERTEX_UVS_B
        in.uv_b,
#endif
#ifdef VERTEX_TANGENTS
        in.world_tangent,
#endif
#ifdef VERTEX_COLORS
        in.color,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        in.instance_index,
#endif
#ifdef VISIBILITY_RANGE_DITHER
        in.visibility_range_dither,
#endif
    );
#endif
}