use bevy::{
    asset::RenderAssetUsages,
    math::Vec3A,
    prelude::*,
    render::mesh::{PrimitiveTopology, VertexAttributeValues},
    window::PrimaryWindow,
};

use bevy_trackball::TrackballCamera;
use curvo::prelude::*;
use egui::{Label, ScrollArea, Ui};
use mines::{
    materials::LineMaterial,
    tnua::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::consts::CHUNK_SIZE_F,
};
use nalgebra::{Const, OPoint, Point2, Point3};

use crate::{
    state::{EditorMode, EditorState, EditorViewMode},
    ui::CursorOverEditSelectionPanel,
    util::mesh_text,
};

use super::ModeSpecific;

#[derive(Component)]
pub struct ProfileMesh(Vec<Point2<f32>>);

impl ProfileMesh {
    pub fn to_3d(&self) -> Vec<OPoint<f32, Const<3>>> {
        let mut points = self
            .0
            .iter()
            .map(|p| Point3::new(p.x, 0.0, p.y))
            .collect::<Vec<_>>();
        points.push(points[0]);

        points
    }

    pub fn to_curve_3d(&self) -> NurbsCurve<f32, Const<4>> {
        let points = self.to_3d();

        NurbsCurve3D::<f32>::try_interpolate(&points, 3).unwrap()
    }

    pub fn to_mesh(&self) -> Mesh {
        let curve = self.to_curve_3d();
        let samples = curve.tessellate(Some(1e-8));
        let vertices = samples
            .iter()
            .map(|p| p.cast::<f32>())
            .map(|p| [p.x, p.y, p.z])
            .collect();

        Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::all()).with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(vertices),
        )
    }
}

/// Hook: enter
pub fn spawn_size_reference_labels(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // "Player"
    commands.spawn((
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -90.0_f32.to_radians(),
            0.0,
            0.0,
        ))
        .with_translation(Vec3::new(
            -PLAYER_RADIUS + 0.017,
            0.0,
            -PLAYER_HEIGHT / 2.0 + 0.14,
        ))
        .with_scale(Vec3::splat(0.2)),
        Mesh3d(meshes.add(mesh_text("Player", true))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            unlit: true,
            ..default()
        })),
    ));

    // "Chunk"
    commands.spawn((
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -90.0_f32.to_radians(),
            0.0,
            0.0,
        ))
        .with_translation(Vec3::new(
            -CHUNK_SIZE_F / 2.0 + 0.2,
            0.0,
            -CHUNK_SIZE_F / 2.0 + 1.6,
        ))
        .with_scale(Vec3::splat(2.25)),
        Mesh3d(meshes.add(mesh_text("Chunk", true))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        })),
    ));
}

/// Hook: update
pub fn draw_size_references(mut gizmos: Gizmos) {
    // Player
    gizmos.rounded_cuboid(
        Vec3::ZERO,
        Vec3::new(PLAYER_RADIUS * 2.0, 0.0, PLAYER_HEIGHT),
        Color::srgb(0.243, 0.757, 0.176),
    );

    // Chunk
    gizmos.rounded_cuboid(
        Vec3::ZERO,
        Vec3::new(CHUNK_SIZE_F, 0.0, CHUNK_SIZE_F),
        Color::srgb(0.776, 0.294, 0.769),
    );
}

/// Hook: update
pub fn pick_profile_point(
    mut gizmos: Gizmos,
    mut state: ResMut<EditorState>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<TrackballCamera>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_over_edit_selection_panel: Res<CursorOverEditSelectionPanel>,
) {
    if state.view != EditorViewMode::Editor {
        return;
    }

    let cursor = cursor_to_ground_plane(&window, *camera);
    let radius = 0.25;
    let mut picked: Option<usize> = None;

    let Some(current) = state.tunnels_mode.files.current_data() else {
        return;
    };

    current.points.iter().enumerate().for_each(|(i, p)| {
        let isometry = Isometry3d {
            rotation: Quat::from_euler(EulerRot::XYZ, -90.0_f32.to_radians(), 0.0, 0.0),
            translation: Vec3A::new(p.x, 0.0, p.y),
        };

        let mut picked_this = false;
        if let Some(cursor) = cursor {
            if !state.tunnels_mode.dragging()
                && picked.is_none()
                && cursor.distance(Vec2::new(p.x, p.y)) <= radius
            {
                picked_this = true;
            }
        }

        if picked_this {
            picked = Some(i);
        }

        let mut color = Color::srgba(1.0, 1.0, 1.0, 0.35);

        if picked_this {
            color = Color::srgb(1.0, 1.0, 1.0);
        }

        if let Some(drag_point) = state.tunnels_mode.selected_point {
            if drag_point == i {
                color = Color::srgb(0.0, 1.0, 1.0);
            }
        }
        gizmos.circle(isometry, radius, color);
        gizmos.circle(isometry, radius * 0.2, color);
    });

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(picked) = picked {
            if let Some(cursor) = cursor {
                state.tunnels_mode.drag_start = Some((current.points[picked], cursor));
                state.tunnels_mode.selected_point = Some(picked);
            }
        } else if !cursor_over_edit_selection_panel.0 {
            state.tunnels_mode.selected_point = None;
        }
    } else if mouse.just_released(MouseButton::Left) {
        state.tunnels_mode.drag_start = None;
    }
}

pub fn drag_profile_point(
    mut state: ResMut<EditorState>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<TrackballCamera>>,
) {
    let Some(drag_point) = state.tunnels_mode.selected_point else {
        return;
    };
    let Some((point_start, cursor_start)) = state.tunnels_mode.drag_start else {
        return;
    };
    let Some(cursor) = cursor_to_ground_plane(&window, *camera) else {
        return;
    };

    let mirror = state.tunnels_mode.mirror;
    let current = state.tunnels_mode.files.current_data_mut();

    let Some(current) = current else {
        return;
    };

    let cursor_diff = cursor - cursor_start;
    let point_new_pos = Point2::new(point_start.x + cursor_diff.x, point_start.y + cursor_diff.y);
    current.points[drag_point] = point_new_pos;

    let len = current.points.len();
    if !mirror {
        return;
    }

    let mut point_new_pos = Point2::new(
        -point_start.x - cursor_diff.x,
        point_start.y + cursor_diff.y,
    );
    if drag_point == 0 || drag_point == len / 2 {
        point_new_pos.x = 0.0;
    }

    let mirror_point = (len - drag_point) % len;
    current.points[mirror_point] = point_new_pos;
}

pub fn update_profile_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    state: Res<EditorState>,
    mesh: Option<Single<(Entity, &mut ProfileMesh)>>,
) {
    let current = state.tunnels_mode.files.current_data();

    let Some(current) = current else {
        return;
    };

    let Some(profile) = mesh else {
        let profile = ProfileMesh(current.points.clone());
        let mesh = profile.to_mesh();
        commands.spawn((
            ModeSpecific(EditorMode::Tunnels, None),
            profile,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(LineMaterial {
                color: Color::srgb(1.0, 1.0, 1.0),
                ..default()
            })),
        ));

        return;
    };

    let (entity, mut profile) = profile.into_inner();

    if current.points == profile.0 {
        return;
    }

    profile.0 = current.points.clone();

    let mut commands = commands.entity(entity);
    commands.insert(Mesh3d(meshes.add(profile.to_mesh())));
}

//
// Sidebar
//

pub fn sidebar(state: &mut EditorState, ui: &mut Ui) {
    let picker = &mut state.tunnels_mode.files;
    let Some(file) = picker.files.get_mut(&picker.current) else {
        return;
    };

    let Some(ref mut data) = file.data else {
        return;
    };

    ScrollArea::vertical().show(ui, |ui| {
        if let Some(selection_index) = state.tunnels_mode.selected_point {
            let selection = data.points[selection_index];
            ui.horizontal(|ui| {
                ui.add(
                    Label::new(format!(
                        "{selection_index}: ({}, {})",
                        selection.x, selection.y
                    ))
                    .selectable(false),
                )
            });
        }
    });
}

//
// Utility
//

/// Adapted from: https://bevy-cheatbook.github.io/cookbook/cursor2world.html
pub fn cursor_to_ground_plane(
    window: &Window,
    (camera, camera_transform): (&Camera, &GlobalTransform),
) -> Option<Vec2> {
    let Some(cursor_position) = window.cursor_position() else {
        return None;
    };
    let plane_origin = Vec3::ZERO;
    let plane = InfinitePlane3d::new(Vec3::Y);
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return None;
    };
    let Some(distance) = ray.intersect_plane(plane_origin, plane) else {
        return None;
    };
    let global_cursor = ray.get_point(distance);

    Some(global_cursor.xz())
}
