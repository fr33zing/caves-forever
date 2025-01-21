use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use meshtext::{MeshGenerator, MeshText, TextSection};

pub fn mesh_text(text: &str, flat: bool) -> Mesh {
    let font_data = include_bytes!("../../../assets/fonts/Urbanist-Regular.ttf");
    let mut generator = MeshGenerator::new(font_data);
    let transform = Mat4::IDENTITY.to_cols_array();
    let text_mesh: MeshText = generator
        .generate_section(&text.to_string(), flat, Some(&transform))
        .unwrap();
    let positions: Vec<[f32; 3]> = text_mesh
        .vertices
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

    mesh
}
