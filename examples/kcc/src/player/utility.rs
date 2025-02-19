use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    input::{PlayerInput, PlayerYaw},
    PlayerInputConfig,
};

#[derive(Clone, Copy)]
pub enum SectionShape {
    Capsule,
    Cylinder,
    SquarePrism,
}

#[derive(Component, Clone, Copy)]
pub struct Section {
    pub shape: SectionShape,
    pub offset: f32,
    pub height: f32,
    pub radius: f32,
}

impl Section {
    pub fn inflated(&self, inflate: f32) -> Self {
        Self {
            shape: self.shape,
            offset: self.offset + inflate,
            height: self.height + inflate * 2.0,
            radius: self.radius + inflate,
        }
    }

    pub fn collider_centered(&self) -> Collider {
        match self.shape {
            SectionShape::Capsule => {
                Collider::capsule(self.radius, self.height - self.radius * 2.0)
            }
            SectionShape::Cylinder => Collider::cylinder(self.radius, self.height),
            SectionShape::SquarePrism => {
                Collider::cuboid(self.radius * 2.0, self.height, self.radius * 2.0)
            }
        }
    }

    pub fn collider(&self) -> Collider {
        Collider::compound(vec![(
            Position::new(Vec3::new(0.0, self.height / 2.0 + self.offset, 0.0)),
            Rotation::default(),
            self.collider_centered(),
        )])
    }

    pub fn mesh(&self) -> Mesh {
        match self.shape {
            SectionShape::Capsule => Capsule3d::new(self.radius, self.height - self.radius * 2.0)
                .mesh()
                .build(),
            SectionShape::Cylinder => Cylinder::new(self.radius, self.height).mesh().build(),
            SectionShape::SquarePrism => {
                Cuboid::new(self.radius * 2.0, self.height, self.radius * 2.0)
                    .mesh()
                    .build()
            }
        }
        .transformed_by(Transform::from_translation(Vec3::new(
            0.0,
            self.height / 2.0 + self.offset,
            0.0,
        )))
    }

    pub fn top(&self, position: Vec3) -> Vec3 {
        position + Vec3::Y * (self.height + self.offset)
    }

    pub fn center(&self, position: Vec3) -> Vec3 {
        position + Vec3::Y * (self.height / 2.0 + self.offset)
    }

    pub fn gizmo(&self, position: Vec3, color: Color, gizmos: &mut Gizmos) {
        match self.shape {
            SectionShape::Capsule => {
                cylinder_gizmo(
                    position + Vec3::Y * self.radius,
                    self.radius,
                    self.height - self.radius * 2.0,
                    color,
                    gizmos,
                );

                // Bottom
                gizmos.arc_3d(
                    PI,
                    self.radius,
                    Isometry3d {
                        rotation: Quat::from_euler(EulerRot::YXZ, 0.0, -FRAC_PI_2, 0.0),
                        translation: (position + Vec3::Y * self.radius).into(),
                    },
                    color,
                );
                gizmos.arc_3d(
                    PI,
                    self.radius,
                    Isometry3d {
                        rotation: Quat::from_euler(EulerRot::YXZ, FRAC_PI_2, -FRAC_PI_2, 0.0),
                        translation: (position + Vec3::Y * self.radius).into(),
                    },
                    color,
                );

                // Top
                gizmos.arc_3d(
                    PI,
                    self.radius,
                    Isometry3d {
                        rotation: Quat::from_euler(EulerRot::YXZ, 0.0, FRAC_PI_2, 0.0),
                        translation: (position + Vec3::Y * (self.height - self.radius)).into(),
                    },
                    color,
                );
                gizmos.arc_3d(
                    PI,
                    self.radius,
                    Isometry3d {
                        rotation: Quat::from_euler(EulerRot::YXZ, FRAC_PI_2, FRAC_PI_2, 0.0),
                        translation: (position + Vec3::Y * (self.height - self.radius)).into(),
                    },
                    color,
                );
            }
            SectionShape::Cylinder => {
                cylinder_gizmo(position, self.radius, self.height, color, gizmos);
            }
            SectionShape::SquarePrism => todo!(),
        }
    }

    pub fn gizmo_centered(&self, position: Vec3, color: Color, gizmos: &mut Gizmos) {
        self.gizmo(position - Vec3::Y * self.height / 2.0, color, gizmos);
    }
}

fn cylinder_gizmo(position: Vec3, radius: f32, height: f32, color: Color, gizmos: &mut Gizmos) {
    let rotation = Quat::from_euler(EulerRot::XYZ, FRAC_PI_2, 0.0, 0.0);

    gizmos.circle(
        Isometry3d {
            rotation,
            translation: position.into(),
        },
        radius,
        color,
    );
    gizmos.circle(
        Isometry3d {
            rotation,
            translation: (position + Vec3::Y * height).into(),
        },
        radius,
        color,
    );

    vec![(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
        .into_iter()
        .for_each(|(x, z)| {
            gizmos.line(
                position + Vec3::new(x * radius, 0.0, z * radius),
                position + Vec3::new(x * radius, height, z * radius),
                color,
            );
        });
}

pub fn wish_dir(yaw: &PlayerYaw, input: &PlayerInput) -> Dir3 {
    let mut wishdir = Vec3::new(input.direction.x, 0.0, input.direction.y);
    wishdir = Quat::from_euler(EulerRot::YXZ, yaw.0, 0.0, 0.0).mul_vec3(wishdir);

    Dir3::new_unchecked(wishdir)
}

pub fn running(input: &PlayerInput, input_config: &PlayerInputConfig) -> bool {
    input.walk_mod != input_config.always_run
}
