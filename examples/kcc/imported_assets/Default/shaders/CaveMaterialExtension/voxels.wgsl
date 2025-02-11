#import bevy_render::{color_operations::hsv_to_rgb, maths::PI_2}
#import bevy_pbr::mesh_view_bindings::globals
#import noisy_bevy::simplex_noise_3d
#import noisy_bevy::simplex_noise_2d

#import "shaders/CaveMaterialExtension/types.wgsl"::{
    VoxelMaterialOutput,
    VoxelMaterialOutput_default
}
#import "shaders/CaveMaterialExtension/utility.wgsl"::{
    clamp_scaled,
    quantize,
    quantize_3d
}


// Register voxel types here.
fn voxel_function_by_type(voxel_type: u32, pos: vec3<f32>) -> VoxelMaterialOutput {
    switch voxel_type {
        case 0u: { return voxel_0(pos); }
        case 1u: { return voxel_1(pos); }
        case 2u: { return voxel_2(pos); }

        case 255u: { return fallback(pos, vec3(0.0, 1.0, 0.0)); } // Unset
        case 254u: { return fallback(pos, vec3(1.0, 0.0, 0.0)); } // Invalid
        case 253u: { return boundary(pos); } // Boundary
        case 252u: { return boundary(pos); } // FakeBoundary
        default: { return fallback(pos, vec3(1.0, 1.0, 0.0)); }
    }
}

fn fallback(
    pos: vec3<f32>,
    color: vec3<f32>
) -> VoxelMaterialOutput {
    var out = VoxelMaterialOutput_default();
    var quantized_pos = quantize_3d(pos, 32.0);
    out.base_color = color;

    if quantized_pos.x % 2.0 == 0.0
       || quantized_pos.y % 2.0 == 0.0
       || quantized_pos.z % 2.0 == 0.0
    {
        out.base_color = vec3(0.0, 0.0, 0.0);
    }

    return out;
}

fn boundary(pos: vec3<f32>) -> VoxelMaterialOutput {
    const vertical_noise_scale = vec3(1.0 / 64.0, 1.0 / 2.0, 1.0 / 64.0);
    const color1 = vec3(0.05, 0.05, 0.05);
    const color2 = vec3(0.1, 0.1, 0.1);

    // Standard layered noise
    var noise = 0.5;
    noise = mix(noise, simplex_noise_3d(pos / 2.0), 0.5);
    noise = mix(noise, simplex_noise_3d(pos / 16.0), 0.5);
    noise = clamp_scaled(noise, 0.25, 1.0);
    noise = quantize(noise, 7.0);

    // Vertical striations
    var vertical_noise = abs(simplex_noise_3d(pos * vertical_noise_scale));
    vertical_noise = quantize(vertical_noise, 4.0);
    let color = mix(color1, color2, vertical_noise);

    var out = VoxelMaterialOutput_default();
    out.base_color = mix(vec3(0.0, 0.0, 0.0), color, noise);
    out.reflectance = vertical_noise;
    return out;
}

fn voxel_0(pos: vec3<f32>) -> VoxelMaterialOutput {
    const vertical_noise_scale = vec3(1.0 / 64.0, 1.0 / 6.0, 1.0 / 64.0);
    const color1 = vec3(0.32, 0.16, 0.08);
    const color2 = vec3(0.12, 0.00, 0.00);

    // Standard layered noise
    var noise = 0.5;
    noise = mix(noise, simplex_noise_3d(pos / 2.0), 0.3);
    noise = mix(noise, simplex_noise_3d(pos / 4.0), 0.3);
    noise = mix(noise, simplex_noise_3d(pos / 8.0), 0.3);
    noise = clamp_scaled(noise, 0.35, 1.0);
    noise = quantize(noise, 7.0);

    // Vertical striations
    var vertical_noise = abs(simplex_noise_3d(pos * vertical_noise_scale));
    vertical_noise = quantize(vertical_noise, 4.0);
    let color = mix(color1, color2, vertical_noise);

    var out = VoxelMaterialOutput_default();
    out.base_color = mix(vec3(0.0, 0.0, 0.0), color, noise);
    return out;
}

fn voxel_1(pos: vec3<f32>) -> VoxelMaterialOutput {
    const vertical_noise_scale = vec3(1.0 / 64.0, 1.0 / 4.0, 1.0 / 64.0);
    const color1 = vec3(0.35, 0.35, 0.15);
    const color2 = vec3(0.65, 0.65, 0.15);

    // Standard layered noise
    var noise = 0.5;
    noise = mix(noise, simplex_noise_3d(pos), 0.5);
    noise = mix(noise, simplex_noise_3d(pos / 5.0), 0.3);
    noise = clamp_scaled(noise, 0.65, 1.0);
    noise = quantize(noise, 7.0);

    // Vertical striations
    var vertical_noise = abs(simplex_noise_3d(pos * vertical_noise_scale));
    vertical_noise = quantize(vertical_noise, 4.0);
    let color = mix(color1, color2, vertical_noise);

    var out = VoxelMaterialOutput_default();
    out.base_color = mix(vec3(0.0, 0.0, 0.0), color, noise);
    return out;
}

fn voxel_2(pos: vec3<f32>) -> VoxelMaterialOutput {
    const vertical_noise_scale = vec3(1.0 / 64.0, 1.0 / 2.0, 1.0 / 64.0);
    const color1 = vec3(0.05, 0.05, 0.05);
    let color2 = hsv_to_rgb(vec3(globals.time / 16.0, 1.0, 0.85));

    // Standard layered noise
    var noise = 0.5;
    noise = mix(noise, simplex_noise_3d(pos / 2.0), 0.5);
    noise = mix(noise, simplex_noise_3d(pos / 16.0), 0.5);
    noise = clamp_scaled(noise, 0.25, 1.0);
    noise = quantize(noise, 7.0);

    // Vertical striations
    var vertical_noise = abs(simplex_noise_3d(pos * vertical_noise_scale));
    vertical_noise = quantize(vertical_noise, 4.0);
    let color = mix(color1, color2, vertical_noise);

    var out = VoxelMaterialOutput_default();
    out.base_color = mix(vec3(0.0, 0.0, 0.0), color, noise);
    out.reflectance = vertical_noise;
    return out;
}