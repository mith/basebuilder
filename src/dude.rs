use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, KinematicCharacterController, RigidBody};

use crate::{
    app_state::AppState,
    gravity::Gravity,
    movement::{Aim, AimingLaser, Jumper, Walker},
    player_controller::PlayerControlled,
    shoot::Shooter,
};

#[derive(Component, Default)]
pub(crate) struct Dude;
fn spawn_dude(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn((
            Dude::default(),
            Walker::default(),
            Jumper::default(),
            Aim::default(),
            Shooter::default(),
            PlayerControlled,
            Name::new("Dude"),
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(0., 16., 0.),
                material: materials.add(Color::GRAY.into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(12., 36.))))
                    .into(),
                ..default()
            },
            RigidBody::KinematicVelocityBased,
            KinematicCharacterController::default(),
            Collider::round_cuboid(5., 16.5, 0.01),
            Gravity,
        ))
        .with_children(|parent| {
            parent.spawn((
                AimingLaser,
                MaterialMesh2dBundle {
                    material: materials.add(Color::RED.into()),
                    mesh: meshes
                        .add(Mesh::from(shape::Quad::new(Vec2::new(1., 0.2))))
                        .into(),
                    transform: Transform::from_xyz(50., 0., 1.),
                    ..default()
                },
            ));
            parent.spawn(Camera2dBundle {
                projection: OrthographicProjection {
                    scale: 0.4,
                    ..default()
                },
                ..default()
            });
        });
}
pub(crate) struct DudePlugin;

impl Plugin for DudePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_dude.in_schedule(OnEnter(AppState::Game)));
    }
}
