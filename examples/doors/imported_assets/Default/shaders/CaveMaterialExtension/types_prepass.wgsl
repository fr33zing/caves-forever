// https://docs.rs/crate/bevy_pbr/0.14.0/source/src/prepass/prepass_io.wgsl

struct CaveVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(2) uv_b: vec2<f32>,
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(3) normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif

    // My changes
    @location(8) voxel_type: vec3u,
    @location(9) voxel_ratio: vec3f,
}

struct CaveVertexOutput {
    @builtin(position) position: vec4<f32>,
#ifdef VERTEX_UVS_A
    @location(0) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(1) uv_b: vec2<f32>,
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(2) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
#endif
#endif
    @location(4) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(5) previous_world_position: vec4<f32>,
#endif
#ifdef DEPTH_CLAMP_ORTHO
    @location(6) clip_position_unclamped: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(7) instance_index: u32,
#endif
#ifdef VERTEX_COLORS
    @location(8) color: vec4<f32>,
#endif

    // My changes
    @location(9) voxel_type: vec3u,
    @location(10) voxel_ratio: vec3f,
}