use avian3d::prelude::*;
use bevy::prelude::*;

pub struct DebugCameraPlugin;

#[derive(Component)]
struct DebugCamera;

const DISTANCE: f32 = 160.0;
const HEIGHT: f32 = 80.0;
const LOOKAT_HEIGHT: f32 = -16.0;
const SPEED: f32 = 0.6;

impl Plugin for DebugCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, update);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        PointLight {
            intensity: 500_000_000.0,
            range: 2048.0,
            color: Color::WHITE.into(),
            ..default()
        },
        Transform::from_xyz(0.0, HEIGHT, 0.0)
            .looking_at(Vec3::new(0.0, LOOKAT_HEIGHT, 0.0), Vec3::Y),
        DebugCamera,
    ));
}

fn update(time: Res<Time>, mut query: Query<&mut Transform, With<DebugCamera>>) {
    for mut transform in query.iter_mut() {
        transform.translation.x = f32::sin(time.elapsed_secs() * SPEED) * DISTANCE;
        transform.translation.z = f32::cos(time.elapsed_secs() * SPEED) * DISTANCE;

        transform.look_at(Vec3::new(0.0, LOOKAT_HEIGHT, 0.0), Vec3::Y);
    }
}
