use bevy::math::Vec3A;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_trackball::TrackballCamera;
use egui::{menu, Align, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui};
use mines::worldgen::asset::TunnelMeshInfo;
use nalgebra::Point2;
use strum::IntoEnumIterator;

use mines::{
    materials::LineMaterial,
    tnua::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::{
        asset::{Environment, Rarity, Tunnel},
        consts::CHUNK_SIZE_F,
    },
};

use super::{EditorHandleGizmos, ModeSpecific};
use crate::state::FilePayload;
use crate::{
    state::{EditorMode, EditorState, EditorViewMode},
    ui::CursorOverEditSelectionPanel,
    util::mesh_text,
};

#[derive(Component)]
pub struct TunnelInfo(Tunnel, TunnelMeshInfo);

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
pub fn draw_size_references(mut gizmos: Gizmos, info: Option<Single<&TunnelInfo>>) {
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

    // Tunnel AABB
    if let Some(info) = info {
        let TunnelMeshInfo { center, size } = info.1;
        let color = Color::srgba(1.0, 1.0, 1.0, 0.03);

        gizmos.rounded_cuboid(
            Vec3::new(center.x, 0.0, center.y),
            Vec3::new(size.x, 0.0, size.y),
            color,
        );
        gizmos.line(
            Vec3::new(center.x, 0.0, center.y - size.y / 2.0),
            Vec3::new(center.x, 0.0, center.y + size.y / 2.0),
            color,
        );
        gizmos.line(
            Vec3::new(center.x - size.x / 2.0, 0.0, center.y),
            Vec3::new(center.x + size.x / 2.0, 0.0, center.y),
            color,
        );
    }
}

/// Hook: update
pub fn pick_profile_point(
    mut gizmos: Gizmos<EditorHandleGizmos>,
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

    let Some(current) = state.files.current_data() else {
        return;
    };
    let FilePayload::Tunnel(data) = current else {
        panic!("pick_profile_point ran in the wrong mode");
    };

    data.points.iter().enumerate().for_each(|(i, p)| {
        let isometry = Isometry3d {
            rotation: Quat::from_euler(EulerRot::XYZ, -90.0_f32.to_radians(), 0.0, 0.0),
            translation: Vec3A::new(p.position.x, 0.0, p.position.y),
        };

        let mut picked_this = false;
        if let Some(cursor) = cursor {
            if !state.tunnels_mode.dragging()
                && picked.is_none()
                && cursor.distance(Vec2::new(p.position.x, p.position.y)) <= radius
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
                state.tunnels_mode.drag_start = Some((data.points[picked].position, cursor));
                state.tunnels_mode.selected_point = Some(picked);
            }
        } else if !cursor_over_edit_selection_panel.0 {
            state.tunnels_mode.selected_point = None;
        }
    } else if mouse.just_released(MouseButton::Left) {
        state.tunnels_mode.drag_start = None;
    }
}

// Hook: update
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
    let data = state.files.current_data_mut();

    let Some(data) = data else {
        return;
    };
    let FilePayload::Tunnel(data) = data else {
        todo!();
    };

    let cursor_diff = cursor - cursor_start;
    let point_new_pos = Point2::new(point_start.x + cursor_diff.x, point_start.y + cursor_diff.y);

    'change: {
        data.points[drag_point].position = point_new_pos;
        let len = data.points.len();

        if !mirror {
            break 'change;
        }

        let mut point_new_pos = Point2::new(
            -point_start.x - cursor_diff.x,
            point_start.y + cursor_diff.y,
        );
        if drag_point == 0 || drag_point == len / 2 {
            point_new_pos.x = 0.0;
        }

        let mirror_point = (len - drag_point) % len;
        data.points[mirror_point].position = point_new_pos;
    }

    state.files.current_file_mut().unwrap().changed = true;
}

// Hook: update
pub fn update_tunnel_info(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    state: Res<EditorState>,
    info: Option<Single<(Entity, &mut TunnelInfo)>>,
) {
    let data = state.files.current_data();

    let Some(data) = data else {
        return;
    };
    let FilePayload::Tunnel(data) = data else {
        todo!()
    };

    let Some(info) = info else {
        let tunnel = data.clone();
        let mesh = tunnel.to_mesh();
        let mesh_info = TunnelMeshInfo::from_mesh(&mesh);
        let model = TunnelInfo(tunnel, mesh_info);

        commands.spawn((
            ModeSpecific(EditorMode::Tunnels, None),
            model,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(LineMaterial {
                color: Color::srgb(1.0, 1.0, 1.0),
                ..default()
            })),
        ));

        return;
    };

    let (entity, mut info) = info.into_inner();

    if *data == info.0 {
        return;
    }

    info.0 = data.clone();
    let mesh = info.0.to_mesh();
    info.1 = TunnelMeshInfo::from_mesh(&mesh);

    let mut commands = commands.entity(entity);
    commands.insert(Mesh3d(meshes.add(mesh)));
}

//
// UI
//

pub fn topbar(state: &mut EditorState, ui: &mut Ui) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Tunnel(data) = data else {
        todo!();
    };

    match state.view {
        EditorViewMode::Editor => {
            Frame::none().show(ui, |ui| {
                ui.shrink_width_to_current();
                menu::bar(ui, |ui| {
                    ui.menu_button("Operations", |ui| {
                        if ui
                            .selectable_label(false, "Center on world origin")
                            .clicked()
                        {
                            ui.close_menu();
                            data.center();
                        };
                    });
                });
            });

            ui.checkbox(&mut state.tunnels_mode.mirror, "Mirror");
        }
        EditorViewMode::Preview => {}
    }
}

pub fn sidebar(state: &mut EditorState, ui: &mut Ui) {
    let picker = &mut state.files;
    let Some(file) = picker.current_file_mut() else {
        return;
    };
    let Some(ref mut data) = file.data else {
        return;
    };
    let FilePayload::Tunnel(data) = data else {
        todo!();
    };

    ui.style_mut().spacing.item_spacing.y = 8.0;

    ui.add(Label::new(RichText::new("Tunnel").heading()).selectable(false));

    // Environment
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Environment").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("tunnel_environment")
                .selected_text(format!("{}", data.environment))
                .show_ui(right, |ui| {
                    Environment::iter().for_each(|env| {
                        ui.selectable_value(&mut data.environment, env, format!("{env}"));
                    });
                });
        });
    });

    // Knot style
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Rarity").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("tunnel_rarity")
                .selected_text(format!("{}", data.rarity))
                .show_ui(right, |ui| {
                    Rarity::iter().for_each(|rarity| {
                        ui.selectable_value(&mut data.rarity, rarity, format!("{rarity}"));
                    });
                });
        });
    });

    ui.separator();

    // Point
    ScrollArea::vertical().show(ui, |ui| {
        if let Some(selection_index) = state.tunnels_mode.selected_point {
            ui.add(
                Label::new(RichText::new(format!("Point {selection_index}")).heading())
                    .selectable(false),
            );

            let selection = &data.points[selection_index];
            ui.add(
                Label::new(format!(
                    "{selection_index}: ({}, {})",
                    selection.position.x, selection.position.y
                ))
                .selectable(false),
            );
        } else {
            ui.add(Label::new(RichText::new("Point").heading()).selectable(false));
            ui.add(Label::new("No point selected.").selectable(false));
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
