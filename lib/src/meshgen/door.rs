use std::{f32::consts::PI, mem::take};

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemState,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use crate::player::IsPlayer;

const DOOR_MAX_ANGLE: f32 = 90.0 * PI / 180.0;
const DOOR_ANIMATION_SECS: f64 = 2.5;
const DOOR_AUTOCLOSE_SECS: f64 = 4.0;

#[derive(Clone, Copy)]
pub struct DoorwaySpec {
    pub frame: Rect,
    pub frame_depth: f32,
    pub frame_uv_scale: f32,
    pub door: Rect,
    pub door_depth: f32,
    pub door_uv_scale: f32,
}

pub struct DoorMeshes {
    pub frame_mesh: Mesh,
    pub door_meshes: [(Mesh, Vec3); 2],
}

#[derive(Component)]
pub struct Doorway {
    locked: bool,
    open: bool,
    open_inward: bool,
    animation_start_secs: f64,
    animating: bool,
    doors: [Entity; 2], // [left, right]
    sfx_position: Vec3,
}

impl Doorway {
    pub fn set_open(&mut self, open: bool, inward: Option<bool>, time: &Res<Time>) -> bool {
        if self.open == open {
            return false;
        }
        let elapsed = time.elapsed_secs_f64() - self.animation_start_secs;
        if elapsed < DOOR_ANIMATION_SECS {
            return false;
        }

        self.open = open;
        self.animation_start_secs = time.elapsed_secs_f64();
        self.animating = true;

        if let Some(inward) = inward {
            self.open_inward = inward;
        }

        true
    }

    pub fn open(&mut self, inward: bool, time: &Res<Time>) -> bool {
        self.set_open(true, Some(inward), time)
    }

    pub fn close(&mut self, time: &Res<Time>) -> bool {
        self.set_open(false, None, time)
    }
}

#[derive(Component)]
pub struct DoorSensor(pub bool); // front?

#[derive(Resource)]
pub struct DoorAnimationCurves {
    pub open: EasingCurve<f32>,
    pub close: EasingCurve<f32>,
}
impl Default for DoorAnimationCurves {
    fn default() -> Self {
        Self {
            open: EasingCurve::new(0.0, 1.0, EaseFunction::ElasticOut),
            close: EasingCurve::new(0.0, 1.0, EaseFunction::CubicIn),
        }
    }
}

#[derive(Resource)]
pub struct DoorSfx {
    pub open: Handle<AudioSource>,
    pub close_start: Handle<AudioSource>,
    pub close_end: Handle<AudioSource>,
    pub locked: Handle<AudioSource>,
    pub unlock: Handle<AudioSource>,
}

#[derive(Component)]
pub struct Door;

pub fn init_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.init_resource::<DoorAnimationCurves>();
    commands.insert_resource(DoorSfx {
        open: asset_server.load("sfx/door/open.ogg"),
        close_start: asset_server.load("sfx/door/close_start.ogg"),
        close_end: asset_server.load("sfx/door/close_end.ogg"),
        locked: asset_server.load("sfx/door/locked.ogg"),
        unlock: asset_server.load("sfx/door/unlock.ogg"),
    });
}

pub fn open_doors_on_contact(
    time: Res<Time>,
    mut commands: Commands,
    mut collision_event_reader: EventReader<Collision>,
    mut doorways: Query<(&GlobalTransform, &mut Doorway)>,
    sensors: Query<(&Parent, &DoorSensor)>,
    player: Query<&IsPlayer>,
    door_sfx: Res<DoorSfx>,
) {
    for Collision(contacts) in collision_event_reader.read() {
        let (mut doorway, open_inward) = {
            if player.get(contacts.entity1).is_err() && player.get(contacts.entity2).is_err() {
                continue;
            }
            let (sensor_parent, open_inward) = {
                let Ok((sensor_parent, sensor)) = sensors
                    .get(contacts.entity1)
                    .or_else(|_| sensors.get(contacts.entity2))
                else {
                    continue;
                };
                (sensor_parent, sensor.0)
            };
            let Ok(doorway) = doorways.get_mut(**sensor_parent) else {
                continue;
            };

            (doorway, open_inward)
        };

        if doorway.1.locked {
            // TODO make a noise
            continue;
        }

        if doorway.1.open(open_inward, &time) {
            commands.spawn((
                Transform::from_translation(doorway.0.translation() + doorway.1.sfx_position),
                AudioPlayer::new(door_sfx.open.clone()),
                PlaybackSettings::DESPAWN.with_spatial(true),
            ));
        }
    }
}

pub fn animate_doors(
    mut commands: Commands,
    door_sfx: Res<DoorSfx>,
    time: Res<Time>,
    curves: Res<DoorAnimationCurves>,
    mut doorways: Query<(&GlobalTransform, &mut Doorway)>,
    mut doors: Query<&mut Transform, With<Door>>,
) {
    doorways
        .iter_mut()
        .for_each(|(doorway_transform, mut doorway)| {
            if !doorway.animating {
                return;
            }

            let mut elapsed = time.elapsed_secs_f64() - doorway.animation_start_secs;

            if doorway.open && elapsed >= DOOR_AUTOCLOSE_SECS {
                doorway.close(&time);
                elapsed = 0.0;

                commands.spawn((
                    Transform::from_translation(
                        doorway_transform.translation() + doorway.sfx_position,
                    ),
                    AudioPlayer::new(door_sfx.close_start.clone()),
                    PlaybackSettings::DESPAWN.with_spatial(true),
                ));
            }

            let Ok([mut left_door, mut right_door]) = doors.get_many_mut(doorway.doors) else {
                return;
            };

            let curve = if doorway.open {
                &curves.open
            } else {
                &curves.close
            };
            let progress = (elapsed / DOOR_ANIMATION_SECS).clamp(0.0, 1.0);
            let progress = curve.sample(progress as f32).unwrap();
            let direction = if doorway.open_inward { 1.0 } else { -1.0 };
            let angle = if doorway.open {
                progress * DOOR_MAX_ANGLE * direction
            } else {
                (DOOR_MAX_ANGLE - progress * DOOR_MAX_ANGLE) * direction
            };

            left_door.rotation = Quat::from_euler(EulerRot::YXZ, angle, 0.0, 0.0);
            right_door.rotation = Quat::from_euler(EulerRot::YXZ, -angle, 0.0, 0.0);

            if elapsed >= DOOR_ANIMATION_SECS && !doorway.open {
                doorway.animating = false;
                commands.spawn((
                    Transform::from_translation(
                        doorway_transform.translation() + doorway.sfx_position,
                    ),
                    AudioPlayer::new(door_sfx.close_end.clone()),
                    PlaybackSettings::DESPAWN.with_spatial(true),
                ));
            }
        });
}

pub struct AddDoorwayToEntity {
    pub spec: DoorwaySpec,
    pub entity: Entity,
}

impl Command for AddDoorwayToEntity {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
            Res<AssetServer>,
        )> = SystemState::new(world);
        let (mut commands, mut meshes, mut materials, asset_server) = system_state.get_mut(world);

        // Materials
        let door_material = materials.add(StandardMaterial {
            reflectance: 0.0,
            base_color_texture: Some(asset_server.load("textures/wood_cabinet_worn_long.tga")),
            ..default()
        });
        let frame_material = materials.add(StandardMaterial {
            reflectance: 0.0,
            base_color_texture: Some(asset_server.load("textures/weathered_brown_planks.tga")),
            ..default()
        });

        // Doors
        let DoorMeshes {
            frame_mesh,
            door_meshes,
        } = generate_door_meshes(self.spec);
        let door_colliders = generate_door_colliders(self.spec);
        let door_entities = door_meshes
            .into_iter()
            .zip(door_colliders.into_iter())
            .map(|((mesh, translation), collider)| {
                commands
                    .spawn((
                        Door,
                        Transform::from_translation(translation),
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(door_material.clone()),
                        RigidBody::Kinematic,
                        collider,
                    ))
                    .id()
            })
            .collect::<Vec<_>>();

        // Triggers
        let trigger_entities = generate_door_triggers(self.spec)
            .into_iter()
            .map(|(collider, open_inward)| {
                commands
                    .spawn((
                        DoorSensor(open_inward),
                        collider,
                        Sensor,
                        DebugRender::default().with_collider_color(Color::srgb(0.1, 0.9, 0.1)),
                    ))
                    .id()
            })
            .collect::<Vec<_>>();

        // Doorway
        let doorway_entity = {
            let mut doorway_entity = commands.spawn((
                Doorway {
                    locked: false,
                    open: false,
                    open_inward: false,
                    animation_start_secs: -DOOR_ANIMATION_SECS,
                    animating: false,
                    doors: [door_entities[0], door_entities[1]],
                    sfx_position: Vec3::new(
                        self.spec.door.center().x,
                        self.spec.door.center().y,
                        0.0,
                    ),
                },
                Transform::default(),
                RigidBody::Static,
                generate_door_frame_collider(self.spec),
                Mesh3d(meshes.add(frame_mesh)),
                MeshMaterial3d(frame_material),
            ));

            doorway_entity.add_children(&door_entities);
            doorway_entity.add_children(&trigger_entities);

            doorway_entity.id()
        };
        commands.entity(self.entity).add_child(doorway_entity);

        system_state.apply(world);
    }
}

pub fn generate_door_frame_collider(door: DoorwaySpec) -> Collider {
    let DoorwaySpec {
        frame,
        door,
        frame_depth,
        ..
    } = door;

    let left_width = door.min.x - frame.min.x;
    let right_width = frame.max.x - door.max.x;
    let top_height = frame.max.y - door.max.y;
    let bottom_height = door.min.y - frame.min.y;
    let door_width = door.max.x - door.min.x;

    Collider::compound(vec![
        // Left
        (
            Vec3::new(
                -frame.width() / 2.0 + left_width / 2.0,
                frame.height() / 2.0,
                0.0,
            ),
            Rotation::default(),
            Collider::cuboid(left_width, frame.height(), frame_depth),
        ),
        // Right
        (
            Vec3::new(
                frame.width() / 2.0 - right_width / 2.0,
                frame.height() / 2.0,
                0.0,
            ),
            Rotation::default(),
            Collider::cuboid(right_width, frame.height(), frame_depth),
        ),
        // Top
        (
            Vec3::new(
                door.min.x + door_width / 2.0,
                frame.max.y - top_height / 2.0,
                0.0,
            ),
            Rotation::default(),
            Collider::cuboid(door.width(), top_height, frame_depth),
        ),
        // Bottom
        (
            Vec3::new(door.min.x + door_width / 2.0, bottom_height / 2.0, 0.0),
            Rotation::default(),
            Collider::cuboid(door.width(), bottom_height, frame_depth),
        ),
    ])
}

/// Returns (left, right)
pub fn generate_door_colliders(door: DoorwaySpec) -> [Collider; 2] {
    let DoorwaySpec {
        door, door_depth, ..
    } = door;

    let collider = Collider::cuboid(door.width() / 2.0, door.height(), door_depth);
    [
        Collider::compound(vec![(
            Vec3::new(door.width() / 4.0, door.height() / 2.0, 0.0),
            Rotation::default(),
            collider.clone(),
        )]),
        Collider::compound(vec![(
            Vec3::new(-door.width() / 4.0, door.height() / 2.0, 0.0),
            Rotation::default(),
            collider,
        )]),
    ]
}

/// Returns (front, back)
pub fn generate_door_triggers(door: DoorwaySpec) -> [(Collider, bool); 2] {
    const INSET: f32 = 0.1;

    let DoorwaySpec {
        door,
        door_depth,
        frame_depth,
        ..
    } = door;

    let collider = Collider::cuboid(
        door.width(),
        door.height(),
        (frame_depth - door_depth) / 2.0 - INSET,
    );
    let position = Vec3::new(
        door.width() / 2.0 + door.min.x,
        door.height() / 2.0 + door.min.y,
        (frame_depth - door_depth) / 4.0 + door_depth / 2.0,
    );
    [
        (
            Collider::compound(vec![(position, Rotation::default(), collider.clone())]),
            true,
        ),
        (
            Collider::compound(vec![(
                position * Vec3::new(1.0, 1.0, -1.0),
                Rotation::default(),
                collider,
            )]),
            false,
        ),
    ]
}

#[derive(Default)]
struct MeshParts {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    pub curr_idx: u16,
}

pub fn generate_door_meshes(
    DoorwaySpec {
        frame,
        door,
        frame_depth,
        door_depth,
        frame_uv_scale,
        ..
    }: DoorwaySpec,
) -> DoorMeshes {
    let mut mesh_parts = MeshParts::default();
    let door_uv_scale = door.max.x - door.min.x; // TEMP (?)

    // Wall
    fill_rect_difference(
        frame,
        door,
        frame_depth / 2.0,
        Vec3::Z,
        false,
        &mut mesh_parts,
        frame_uv_scale,
    );
    fill_rect_difference(
        frame,
        door,
        -frame_depth / 2.0,
        Vec3::NEG_Z,
        true,
        &mut mesh_parts,
        frame_uv_scale,
    );

    // Frame
    fill_rect_extrusion(door, frame_depth, true, &mut mesh_parts, frame_uv_scale);

    let frame = finish_mesh(&mut mesh_parts);

    // Door mesh
    let left_door_rect = Rect {
        min: Vec2::ZERO,
        max: Vec2::new(door.width() / 2.0, door.height()),
    };

    fill_rect_extrusion(
        left_door_rect,
        door_depth,
        false,
        &mut mesh_parts,
        door_uv_scale,
    );

    fill_rect(
        left_door_rect,
        door_depth / 2.0,
        Vec3::Z,
        false,
        &mut mesh_parts,
        door_uv_scale,
    );
    fill_rect(
        left_door_rect,
        -door_depth / 2.0,
        Vec3::NEG_Z,
        true,
        &mut mesh_parts,
        door_uv_scale,
    );

    let left_door = { finish_mesh(&mut mesh_parts) };

    // Right door
    let right_door_rect = Rect {
        min: Vec2::new(-door.width() / 2.0, 0.0),
        max: Vec2::new(0.0, door.height()),
    };

    fill_rect_extrusion(
        right_door_rect,
        door_depth,
        false,
        &mut mesh_parts,
        door_uv_scale,
    );

    fill_rect(
        right_door_rect,
        door_depth / 2.0,
        Vec3::Z,
        false,
        &mut mesh_parts,
        door_uv_scale,
    );
    fill_rect(
        right_door_rect,
        -door_depth / 2.0,
        Vec3::NEG_Z,
        true,
        &mut mesh_parts,
        door_uv_scale,
    );

    let right_door = { finish_mesh(&mut mesh_parts) };

    DoorMeshes {
        frame_mesh: frame,
        door_meshes: [
            (left_door, Vec3::new(door.min.x, door.min.y, 0.0)),
            (right_door, Vec3::new(door.max.x, door.min.y, 0.0)),
        ],
    }
}

//
// Utility
//

fn finish_mesh(mesh_parts: &mut MeshParts) -> Mesh {
    mesh_parts.curr_idx = 0;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, take(&mut mesh_parts.positions))
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, take(&mut mesh_parts.normals))
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, take(&mut mesh_parts.colors))
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, take(&mut mesh_parts.uvs))
    .with_inserted_indices(Indices::U16(take(&mut mesh_parts.indices)))
}

fn vert(
    position: [f32; 3],
    normal: Vec3,
    color: [f32; 4],
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) -> u16 {
    let idx = mesh_parts.curr_idx;
    mesh_parts.curr_idx += 1;
    mesh_parts.positions.push(position);
    mesh_parts.normals.push(normal.into());
    mesh_parts.colors.push(color);
    mesh_parts
        .uvs
        .push([position[0] / uv_scale, position[1] / uv_scale]);

    idx
}

fn verts<const N: usize>(
    vec: Vec<[f32; 3]>,
    normal: Vec3,
    color: [f32; 4],
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) -> [u16; N] {
    let mut verts = [0; N];
    for (i, v) in vec.into_iter().enumerate() {
        verts[i] = vert(v, normal, color.clone(), mesh_parts, uv_scale);
    }
    verts
}

fn rect_verts(
    rect: Rect,
    depth: f32,
    normal: Vec3,
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) -> [u16; 4] {
    verts(
        vec![
            [rect.min.x, rect.min.y, depth], // [0] bottom left
            [rect.max.x, rect.min.y, depth], // [1] bottom right
            [rect.min.x, rect.max.y, depth], // [2] top left
            [rect.max.x, rect.max.y, depth], // [3] top right
        ],
        normal,
        [1.0, 1.0, 1.0, 1.0],
        mesh_parts,
        uv_scale,
    )
}

fn fill_rect(
    rect: Rect,
    depth: f32,
    normal: Vec3,
    invert: bool,
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) {
    let mut verts = rect_verts(rect, depth, normal, mesh_parts, uv_scale);
    if invert {
        verts.swap(0, 1);
        verts.swap(2, 3);
    }
    mesh_parts.indices.extend(
        [
            [verts[1], verts[3], verts[0]],
            [verts[3], verts[2], verts[0]],
        ]
        .as_flattened(),
    );
}

fn fill_rect_extrusion_edge(mut verts: [u16; 4], invert: bool, mesh_parts: &mut MeshParts) {
    if invert {
        verts.swap(0, 1);
        verts.swap(2, 3);
    }
    mesh_parts.indices.extend(
        [
            [verts[1], verts[0], verts[2]],
            [verts[2], verts[3], verts[1]],
        ]
        .as_flattened(),
    );
}

fn fill_rect_difference(
    outer: Rect,
    inner: Rect,
    depth: f32,
    normal: Vec3,
    invert: bool,
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) {
    let mut outer = rect_verts(outer, depth, normal, mesh_parts, uv_scale);
    let mut inner = rect_verts(inner, depth, normal, mesh_parts, uv_scale);
    if invert {
        outer.swap(0, 1);
        outer.swap(2, 3);
        inner.swap(0, 1);
        inner.swap(2, 3);
    }

    mesh_parts.indices.extend(
        vec![
            // Bottom
            [outer[0], inner[1], inner[0]],
            [outer[0], outer[1], inner[1]],
            // Top
            [inner[2], outer[3], outer[2]],
            [inner[3], outer[3], inner[2]],
            // Left
            [inner[0], inner[2], outer[2]],
            [inner[0], outer[2], outer[0]],
            // Right
            [outer[1], inner[3], inner[1]],
            [outer[1], outer[3], inner[3]],
        ]
        .as_flattened(),
    );
}

fn fill_rect_extrusion(
    rect: Rect,
    depth: f32,
    invert: bool,
    mesh_parts: &mut MeshParts,
    uv_scale: f32,
) {
    const BRIGHTNESS: f32 = 0.125;
    const COLOR: [f32; 4] = [BRIGHTNESS, BRIGHTNESS, BRIGHTNESS, 1.0];

    let invert_mul = if invert { -1.0 } else { 1.0 };

    // Bottom
    fill_rect_extrusion_edge(
        verts(
            vec![
                [rect.min.x, rect.min.y, depth / 2.0],
                [rect.max.x, rect.min.y, depth / 2.0],
                [rect.min.x, rect.min.y, -depth / 2.0],
                [rect.max.x, rect.min.y, -depth / 2.0],
            ],
            Vec3::NEG_Y * invert_mul,
            COLOR.clone(),
            mesh_parts,
            uv_scale,
        ),
        invert,
        mesh_parts,
    );
    // Top
    fill_rect_extrusion_edge(
        verts(
            vec![
                [rect.min.x, rect.max.y, depth / 2.0],
                [rect.max.x, rect.max.y, depth / 2.0],
                [rect.min.x, rect.max.y, -depth / 2.0],
                [rect.max.x, rect.max.y, -depth / 2.0],
            ],
            Vec3::Y * invert_mul,
            COLOR.clone(),
            mesh_parts,
            uv_scale,
        ),
        !invert,
        mesh_parts,
    );
    // Left
    fill_rect_extrusion_edge(
        verts(
            vec![
                [rect.min.x, rect.min.y, depth / 2.0],
                [rect.min.x, rect.max.y, depth / 2.0],
                [rect.min.x, rect.min.y, -depth / 2.0],
                [rect.min.x, rect.max.y, -depth / 2.0],
            ],
            Vec3::NEG_X * invert_mul,
            COLOR.clone(),
            mesh_parts,
            uv_scale,
        ),
        !invert,
        mesh_parts,
    );
    // Right
    fill_rect_extrusion_edge(
        verts(
            vec![
                [rect.max.x, rect.min.y, depth / 2.0],
                [rect.max.x, rect.max.y, depth / 2.0],
                [rect.max.x, rect.min.y, -depth / 2.0],
                [rect.max.x, rect.max.y, -depth / 2.0],
            ],
            Vec3::X * invert_mul,
            COLOR,
            mesh_parts,
            uv_scale,
        ),
        invert,
        mesh_parts,
    );
}
