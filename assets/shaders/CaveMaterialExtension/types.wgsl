// Register PBR attributes to make available to voxel functions here.
// Blending logic must be defined in utility.wgsl to take effect.
struct VoxelMaterialOutput {
    // Note that this base_color is a vec3, unlike the standard PBR base_color.
    base_color: vec3<f32>,
    reflectance: f32,
    emissive: vec4<f32>,
}

fn VoxelMaterialOutput_default() -> VoxelMaterialOutput {
    return VoxelMaterialOutput(
        vec3(0.0, 0.0, 0.0),
        0.0,
        vec4(0.0, 0.0, 0.0, 1.0),
    );
}