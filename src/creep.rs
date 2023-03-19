use bevy::{math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, KinematicCharacterController, RigidBody};

use crate::{
    ai_controller::{AiControlled, Target},
    app_state::AppState,
    gravity::Gravity,
    health::Health,
    movement::Walker,
    player::Player,
};

#[derive(Component)]
pub(crate) struct Creep;

#[derive(Resource)]
struct CreepSpawnTimer(Timer);

impl Default for CreepSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2., TimerMode::Repeating))
    }
}

fn spawn_creep(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    dude_query: Query<(Entity, &Transform), With<Player>>,
    mut timer: ResMut<CreepSpawnTimer>,
    time: Res<Time>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let (dude_entity, dude_transform) = dude_query.single();
        commands.spawn((
            Creep,
            Walker::default(),
            AiControlled,
            Name::new("Creep"),
            Health { value: 100 },
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(160., 16., 0.),
                material: materials.add(Color::MAROON.into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(12., 12.))))
                    .into(),
                ..default()
            },
            RigidBody::KinematicVelocityBased,
            KinematicCharacterController::default(),
            Collider::round_cuboid(5., 5., 0.01),
            Gravity,
            Target {
                entity: Some(dude_entity),
                position: dude_transform.translation.xy(),
            },
        ));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct CreepSet;

pub(crate) struct CreepPlugin;

impl Plugin for CreepPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreepSpawnTimer>().add_system(
            spawn_creep
                .in_schedule(OnEnter(AppState::Game))
                .in_set(CreepSet),
        );
    }
}
