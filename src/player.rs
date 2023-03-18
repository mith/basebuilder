use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, KinematicCharacterController, RigidBody};

use crate::{
    app_state::AppState,
    gravity::Gravity,
    gun::{Gun, Muzzle},
    movement::{Aim, Hands, Jumper, Walker},
    player_controller::PlayerControlled,
};

#[derive(Component)]
pub(crate) struct Player;

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn((
            Player,
            Walker::default(),
            Jumper::default(),
            Aim::default(),
            PlayerControlled,
            Name::new("Player"),
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
            parent
                .spawn((
                    Hands,
                    Name::new("Hands"),
                    TransformBundle {
                        local: Transform::from_xyz(0., 5., 1.),
                        ..default()
                    },
                    VisibilityBundle::default(),
                ))
                .with_children(|hands| {
                    spawn_gun(hands, materials, meshes);
                });
            parent.spawn(Camera2dBundle {
                projection: OrthographicProjection {
                    scale: 0.4,
                    ..default()
                },
                ..default()
            });
        });
}

fn spawn_gun(
    hands: &mut ChildBuilder,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let gun_size = 15.;
    hands
        .spawn((
            Gun::default(),
            Name::new("Gun"),
            MaterialMesh2dBundle {
                material: materials.add(Color::DARK_GRAY.into()),
                transform: Transform::from_xyz(gun_size * 0.5, 0., 1.),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(gun_size, 4.0))))
                    .into(),
                ..default()
            },
        ))
        .with_children(|gun| {
            gun.spawn((
                Muzzle,
                TransformBundle {
                    local: Transform::from_xyz(gun_size * 0.5, 0., 1.),
                    ..default()
                },
            ));
        });
}

pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_player.in_schedule(OnEnter(AppState::Game)));
    }
}
