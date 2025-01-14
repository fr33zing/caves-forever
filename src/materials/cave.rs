use bevy::{
    pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeDescriptor},
        render_resource::*,
    },
};

/// Specifies what type of voxel the vertex belongs to. The last u8 is
/// not used for anything.
pub const ATTRIBUTE_VOXEL_TYPE: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_VoxelType", 989717230, VertexFormat::Uint8x4);

pub const ATTRIBUTE_VOXEL_RATIO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_VoxelRatio", 989717231, VertexFormat::Float32x3);

const SHADER_VERTEX_PATH: &str = "shaders/CaveMaterialExtension/vertex.wgsl";
const SHADER_FRAGMENT_PATH: &str = "shaders/CaveMaterialExtension/fragment.wgsl";

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct CaveMaterialExtension {
    #[uniform(100)]
    pub render_voxel_size: f32,

    #[uniform(101)]
    pub voxel_type_transition_steps: f32,
}

impl MaterialExtension for CaveMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        SHADER_VERTEX_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_FRAGMENT_PATH.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let prepass = Some("pbr_prepass_pipeline") == descriptor.label.as_deref();
        let mut attrs = Vec::<VertexAttributeDescriptor>::new();

        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_POSITION);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_UV_0);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_UV_1);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_NORMAL);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_TANGENT);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_JOINT_INDEX);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_JOINT_WEIGHT);
        push_if_present(&mut attrs, layout, prepass, Mesh::ATTRIBUTE_COLOR);

        attrs.push(ATTRIBUTE_VOXEL_TYPE.at_shader_location(8));
        attrs.push(ATTRIBUTE_VOXEL_RATIO.at_shader_location(9));

        let vertex_layout = layout.0.get_layout(&attrs)?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

fn push_if_present(
    vec: &mut Vec<VertexAttributeDescriptor>,
    layout: &MeshVertexBufferLayoutRef,
    prepass: bool,
    attribute: MeshVertexAttribute,
) {
    if layout.0.contains(attribute) {
        vec.push(std_material_attr(prepass, attribute));
    }
}

/// Returns the vertex attribute at the same shader location used by StandardMaterial.
fn std_material_attr(prepass: bool, attribute: MeshVertexAttribute) -> VertexAttributeDescriptor {
    let location = if prepass {
        match attribute.name {
            "Vertex_Position" => 0,
            "Vertex_Uv" => 1,
            "Vertex_Uv_1" => 2,
            "Vertex_Normal" => 3,
            "Vertex_Tangent" => 4,
            "Vertex_JointIndex" => 5,
            "Vertex_JointWeight" => 6,
            "Vertex_Color" => 7,
            _ => panic!(),
        }
    } else {
        match attribute.name {
            "Vertex_Position" => 0,
            "Vertex_Uv" => 2,
            "Vertex_Uv_1" => 3,
            "Vertex_Normal" => 1,
            "Vertex_Tangent" => 4,
            "Vertex_JointIndex" => 6,
            "Vertex_JointWeight" => 7,
            "Vertex_Color" => 5,
            _ => panic!(),
        }
    };

    attribute.at_shader_location(location)
}
