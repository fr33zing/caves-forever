use std::hash::{Hash, Hasher};

use bevy::{
    math::Vec3A,
    prelude::{Sphere, *},
    render::{mesh::PrimitiveTopology, view::RenderLayers},
    window::PrimaryWindow,
};
use bevy_trackball::TrackballCamera;
use curvo::prelude::{NurbsCurve3D, Tessellation};
use egui::{menu, Align, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui};
use nalgebra::{Point2, Point3};
use pathfinding::prelude::dfs;
use strum::IntoEnumIterator;
use transform_gizmo_bevy::{enum_set, GizmoMode, GizmoOrientation};

use mines::{
    materials::LineMaterial,
    tnua::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::{
        asset::{Environment, Rarity, Tunnel, TunnelMeshInfo},
        brush::{curve::mesh_curve, sweep::ProfileRamp, TerrainBrush, TerrainBrushRequest},
        consts::CHUNK_SIZE_F,
        voxel::VoxelMaterial,
    },
};
use uuid::Uuid;

use super::{EditorHandleGizmos, ModeSpecific};
use crate::{
    gizmos::{ConnectedPath, ConnectionPlane, ConnectionPoint, Pickable},
    state::{EditorMode, EditorState, EditorViewMode, FilePayload},
    ui::CursorOverEditSelectionPanel,
    util::mesh_text,
};

#[derive(Component)]
pub struct TunnelInfo(Tunnel, TunnelMeshInfo);

#[derive(Component)]
pub struct UpdatePreviewBrush {
    time: f64,
    rail: Vec<Point3<f32>>,
    profile: ProfileRamp,
}

/// Hook: enter
pub fn spawn_size_reference_labels(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // "Player"
    commands.spawn((
        RenderLayers::from_layers(&[1]),
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZXY,
            180.0_f32.to_radians(),
            90.0_f32.to_radians(),
            0.0,
        ))
        .with_translation(Vec3::new(
            PLAYER_RADIUS - 0.017,
            0.0,
            PLAYER_HEIGHT / 2.0 - 0.14,
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
        RenderLayers::from_layers(&[1]),
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZXY,
            180.0_f32.to_radians(),
            90.0_f32.to_radians(),
            0.0,
        ))
        .with_translation(Vec3::new(
            CHUNK_SIZE_F / 2.0 - 0.2,
            0.0,
            CHUNK_SIZE_F / 2.0 - 1.6,
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

fn spawn_doorway(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    color: &Color,
    transform: Transform,
) {
    commands
        .spawn((
            RenderLayers::from_layers(&[1]),
            ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
            ConnectionPlane,
            RayCastBackfaces,
            transform,
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(1.0, 0.125, 1.0)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color.with_alpha(0.1),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            })),
            Pickable(
                Some(
                    enum_set!(
                        GizmoMode::RotateZ
                            | GizmoMode::ScaleX
                            | GizmoMode::ScaleZ
                            | GizmoMode::ScaleXZ
                    )
                    .union(GizmoMode::all_translate()),
                ),
                Some(GizmoOrientation::Local),
            ),
        ))
        .with_child((
            ConnectionPoint,
            Transform::from_translation(Vec3::NEG_Y * 4.0).with_scale(Vec3::new(
                1.0 / transform.scale.x,
                1.0,
                1.0 / transform.scale.z,
            )),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color.with_alpha(0.8),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            })),
        ));

    commands.spawn((
        RenderLayers::from_layers(&[1]),
        ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
        ConnectionPoint,
        Transform::from_translation(transform.translation * Vec3::new(0.4, 1.0, 0.0)),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color.with_alpha(0.8),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        })),
        Pickable(Some(GizmoMode::all_translate()), None),
    ));
}

/// Hook: enter_view
pub fn enter_preview(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let door_scale = Vec3::new(10.0, 1.0, 10.0);
    let y = door_scale.z / 2.0 + 2.0;
    let color = Color::srgba(0.0, 1.0, 1.0, 0.1);

    commands.spawn((
        RenderLayers::from_layers(&[1]),
        ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
        ConnectionPoint,
        Transform::from_translation(Vec3::Y * y),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color.with_alpha(0.8),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        })),
        Pickable(Some(GizmoMode::all_translate()), None),
    ));

    spawn_doorway(
        &mut commands,
        &mut materials,
        &mut meshes,
        &color,
        Transform::default()
            .with_translation(Vec3::new(CHUNK_SIZE_F / 2.0, y, 0.0))
            .with_scale(door_scale)
            .with_rotation(Quat::from_euler(
                EulerRot::YXZ,
                -90.0_f32.to_radians(),
                -90.0_f32.to_radians(),
                0.0,
            )),
    );

    spawn_doorway(
        &mut commands,
        &mut materials,
        &mut meshes,
        &color,
        Transform::default()
            .with_translation(Vec3::new(-CHUNK_SIZE_F / 2.0, y, 0.0))
            .with_scale(door_scale)
            .with_rotation(Quat::from_euler(
                EulerRot::YXZ,
                90.0_f32.to_radians(),
                -90.0_f32.to_radians(),
                0.0,
            )),
    );
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

    let len = data.points.len();
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
        if i == 0 || i == len / 2 {
            gizmos.circle(isometry, radius * 0.4, color);
        }
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

    data.points[drag_point].position = point_new_pos;
    let len = data.points.len();

    if !mirror || drag_point == 0 || drag_point == len / 2 {
        return;
    }

    let point_new_pos = Point2::new(
        -point_start.x - cursor_diff.x,
        point_start.y + cursor_diff.y,
    );

    let mirror_point = (len - drag_point) % len;
    data.points[mirror_point].position = point_new_pos;
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

// Hook: update
pub fn remesh_preview_path(
    state: Res<EditorState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    update_preview_brush: Query<(Entity, &UpdatePreviewBrush)>,
    time: Res<Time>,
    any_pickable_changed: Query<&Pickable, Changed<Transform>>,
    path: Option<Single<Entity, With<ConnectedPath>>>,
    planes: Query<&GlobalTransform, With<ConnectionPlane>>,
    points: Query<&GlobalTransform, With<ConnectionPoint>>,
    info: Option<Single<&mut TunnelInfo>>,
) {
    let dirty = !any_pickable_changed.is_empty() || path.is_none();
    if !dirty || state.view != EditorViewMode::Preview {
        return;
    }

    let Some(info) = info else {
        return;
    };

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Point(i8, Vec3);
    impl Eq for Point {}
    impl Hash for Point {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    let planes = planes.iter().collect::<Vec<_>>();

    let Some(start_plane) = planes.first() else {
        return;
    };
    let Some(end_plane) = planes.last() else {
        return;
    };

    let (start_point, end_point) = (
        Point(i8::MIN, start_plane.translation()),
        Point(i8::MAX, end_plane.translation()),
    );

    let mut points = points
        .iter()
        .enumerate()
        .map(|(i, p)| Point(i as i8, p.translation()))
        .collect::<Vec<_>>();
    points.push(end_point);

    let Some(result) = dfs(
        start_point,
        |p| {
            let mut ps = points.clone();
            ps.sort_unstable_by_key(|q| q.1.distance_squared(p.1) as u32);
            ps
        },
        |p| p.0 == end_point.0,
    ) else {
        return;
    };

    let points = result.into_iter().map(|p| p.1).collect::<Vec<_>>();
    let line_mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points.clone());

    let rail = points
        .into_iter()
        .map(|p| Point3::from(p))
        .collect::<Vec<_>>();
    let Ok(curve) = NurbsCurve3D::<f32>::try_interpolate(&rail, 3) else {
        return;
    };
    let samples = curve.tessellate(Some(1e-8));
    let curve_mesh = mesh_curve(&samples);

    if let Some(path) = path {
        commands.entity(*path).despawn_recursive();
    }

    commands
        .spawn((
            ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
            ConnectedPath,
            Mesh3d(meshes.add(line_mesh)),
            MeshMaterial3d(materials.add(LineMaterial {
                color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                opacity: 0.2,
                alpha_mode: AlphaMode::Blend,
            })),
        ))
        .with_children(|parent| {
            parent.spawn((
                ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
                Mesh3d(meshes.add(curve_mesh)),
                MeshMaterial3d(materials.add(LineMaterial {
                    color: Color::WHITE,
                    ..default()
                })),
            ));
        });

    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Tunnel(tunnel) = data else {
        return;
    };
    update_preview_brush.iter().for_each(|(entity, _)| {
        commands.entity(entity).despawn();
    });

    let size = info.1.size;
    let start_scale = start_plane.scale().xz() / size * 1.01;
    let end_scale = end_plane.scale().xz() / size * 1.01;
    let profile = ProfileRamp::start(tunnel.to_3d_xy_scaled(start_scale))
        .end(tunnel.to_3d_xy_scaled(end_scale));

    commands.spawn(UpdatePreviewBrush {
        time: time.elapsed_secs_f64(),
        rail,
        profile,
    });
}

pub fn update_preview_brush(
    mut commands: Commands,
    time: Res<Time>,
    update_preview_brush: Option<Single<(Entity, &UpdatePreviewBrush)>>,
    terrain_brushes: Query<Entity, With<TerrainBrush>>,
) {
    const TIMER_SECS: f64 = 0.5;

    let Some(brush) = update_preview_brush else {
        return;
    };
    if time.elapsed_secs_f64() - brush.1.time < TIMER_SECS {
        return;
    }

    let (entity, upb) = brush.into_inner();

    commands.entity(entity).despawn();
    terrain_brushes.iter().for_each(|entity| {
        commands.entity(entity).despawn();
    });

    commands.spawn(TerrainBrushRequest::Sweep {
        uuid: Uuid::new_v4().into(),
        material: VoxelMaterial::BrownRock,
        rail: upb.rail.clone(),
        profile: upb.profile.clone(),
    });
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
                        if ui
                            .selectable_value(&mut data.environment, env, format!("{env}"))
                            .clicked()
                        {
                            //file.changed = true;
                        };
                    });
                });
        });
    });

    // Rarity
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Rarity").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("tunnel_rarity")
                .selected_text(format!("{}", data.rarity))
                .show_ui(right, |ui| {
                    Rarity::iter().for_each(|rarity| {
                        if ui
                            .selectable_value(&mut data.rarity, rarity, format!("{rarity}"))
                            .clicked()
                        {
                            //file.changed = true;
                        };
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
