#import bevy_pbr::{
    pbr_types::PbrInput,
    pbr_fragment::pbr_input_from_standard_material
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

#import "shaders/CaveMaterialExtension/types.wgsl"::VoxelMaterialOutput
#import "shaders/CaveMaterialExtension/voxels.wgsl"::voxel_function_by_type
#import "shaders/CaveMaterialExtension/utility.wgsl"::{
    ease_in_out_sine_3d,
    mix_two_voxels,
    mix_three_voxels,
    reconstruct_pbr_vertex,
    quantize_3d
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

@group(2) @binding(100)
var<uniform> render_voxel_size: f32;

@group(2) @binding(101)
var<uniform> voxel_type_transition_steps: f32;

fn easeInOutSine(x: f32) -> f32 {
    return -(cos(3.14 * x) - 1.0) / 2.0;
}

fn easeInOutCubic(x: f32) -> f32 {
    if x < 0.5 {
        return 4.0 * x * x * x;
    } else {
        return 1.0 - pow(-2.0 * x + 2.0, 3.0) / 2.0;
    }
}

fn easeInOutCirc(x: f32) -> f32 {
    if x < 0.5 {
        return (1.0 - sqrt(1.0 - pow(2.0 * x, 2.0))) / 2.0;
    }
    return (sqrt(1.0 - pow(-2.0 * x + 2.0, 2.0)) + 1.0) / 2.0;
}

@fragment
fn fragment(
    in: CaveVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    let quantized_pos = quantize_3d(in.world_position.xyz, render_voxel_size);
    let fac = quantize_3d(ease_in_out_sine_3d(in.voxel_ratio), voxel_type_transition_steps);
    var voxel: VoxelMaterialOutput;

    if in.voxel_type[0] == in.voxel_type[1] {
        let a = voxel_function_by_type(in.voxel_type[0], quantized_pos);

        if in.voxel_type[0] == in.voxel_type[2] {
            // Same voxel type for all vertices
            voxel = a;
        } else {
            let b = voxel_function_by_type(in.voxel_type[2], quantized_pos);
            voxel = mix_two_voxels(a, b, fac.z);
        }
    } else if in.voxel_type[0] == in.voxel_type[2] {
        let a = voxel_function_by_type(in.voxel_type[0], quantized_pos);
        let b = voxel_function_by_type(in.voxel_type[1], quantized_pos);
        voxel = mix_two_voxels(a, b, fac.y);
    } else if in.voxel_type[1] == in.voxel_type[2] {
        let a = voxel_function_by_type(in.voxel_type[1], quantized_pos);
        let b = voxel_function_by_type(in.voxel_type[0], quantized_pos);
        voxel = mix_two_voxels(a, b, fac.x);
    } else {
        // Different voxel type for all vertices
        let a = voxel_function_by_type(in.voxel_type[0], quantized_pos);
        let b = voxel_function_by_type(in.voxel_type[1], quantized_pos);
        let c = voxel_function_by_type(in.voxel_type[2], quantized_pos);
        voxel = mix_three_voxels(a, b, c, fac);
    }

    let pbr_vertex = reconstruct_pbr_vertex(in);
    var pbr_input = pbr_input_from_standard_material(pbr_vertex, is_front);
    pbr_input.material.base_color = vec4(voxel.base_color, 1.0);
    pbr_input.material.reflectance = voxel.reflectance;
    pbr_input.material.emissive = voxel.emissive;
    pbr_input.material.perceptual_roughness = 0.0;
    //pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    out.color = apply_pbr_lighting(pbr_input);

    // (optional) modify the lit color before post-processing is applied here

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    // (optional) modify the final result here
#endif

    return out;
}
