#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    view_transformations::position_world_to_clip,
}

#ifdef PREPASS_PIPELINE
#import "shaders/CaveMaterialExtension/types_prepass.wgsl"::{
    CaveVertex,
    CaveVertexOutput
}
#else
#import "shaders/CaveMaterialExtension/types_forward.wgsl"::{
    CaveVertex,
    CaveVertexOutput
}
#endif

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/mesh.wgsl
@vertex
fn vertex(vertex_no_morph: CaveVertex) -> CaveVertexOutput {
    var out: CaveVertexOutput;
#ifdef MORPH_TARGETS
    //var vertex = morph_vertex(vertex_no_morph);
    var vertex = vertex_no_morph;
#else
    var vertex = vertex_no_morph;
#endif
    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);
#endif

    // My changes
    out.voxel_type = vertex.voxel_type;
    out.voxel_ratio = vertex.voxel_ratio;

    return out;
}