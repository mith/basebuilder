use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, ExternalForce, ExternalImpulse, RigidBody};

use crate::{
    health::{Health, HealthDamageEvent, HealthSet},
    movement::{AimingAt, MovementSet},
};

#[derive(Component, Default)]
pub(crate) struct Shooter {
    pub(crate) shoot: bool,
}

#[derive(Resource)]
struct ShootTimer(Timer);

impl Default for ShootTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.2, TimerMode::Repeating))
    }
}

#[derive(Component)]
pub(crate) struct BloodParticle {
    pub(crate) lifetime: f32,
}

#[derive(Component)]
pub(crate) struct Wound;

fn shoot(
    mut commands: Commands,
    dude_query: Query<(&Shooter, &AimingAt)>,
    health_query: Query<&GlobalTransform, With<Health>>,
    mut health_damage_events: EventWriter<HealthDamageEvent>,
    mut timer: ResMut<ShootTimer>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (dude_input, aiming_at) in &dude_query {
        if dude_input.shoot && health_query.contains(aiming_at.target) {
            if timer.0.tick(time.delta()).just_finished() {
                health_damage_events.send(HealthDamageEvent {
                    entity: aiming_at.target,
                    damage: 25,
                });

                // spawn blood particle
                let particle_size = 1.;
                commands.spawn((
                    BloodParticle { lifetime: 0.5 },
                    MaterialMesh2dBundle {
                        transform: Transform::from_xyz(
                            aiming_at.intersection.point.x,
                            aiming_at.intersection.point.y,
                            0.,
                        ),
                        material: materials.add(Color::RED.into()),
                        mesh: meshes
                            .add(Mesh::from(shape::Quad::new(Vec2::new(particle_size, particle_size))))
                            .into(),
                        ..default()
                    },
                    RigidBody::Dynamic,
                    Collider::round_cuboid(particle_size / 2., particle_size / 2., 0.01),
                    ExternalImpulse {
                        impulse: aiming_at.intersection.normal * 0.01,
                        torque_impulse: 0.,
                    },
                ));

                let target_global_transform = health_query.get(aiming_at.target).unwrap();

                // spawn wound on target
                commands.entity(aiming_at.target).with_children(|target| {
                    target.spawn((
                        Wound,
                        MaterialMesh2dBundle {
                            transform: Transform::from_xyz(
                                aiming_at.intersection.point.x
                                    - target_global_transform.translation().x,
                                aiming_at.intersection.point.y
                                    - target_global_transform.translation().y,
                                1.,
                            ),
                            material: materials.add(Color::RED.into()),
                            mesh: meshes
                                .add(Mesh::from(shape::Quad::new(Vec2::new(2., 2.))))
                                .into(),
                            ..default()
                        },
                    ));
                });
            }
        }
    }
}

fn blood_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut BloodParticle)>,
    time: Res<Time>,
) {
    for (entity, mut blood_particle) in &mut query {
        blood_particle.lifetime -= time.delta_seconds();
        if blood_particle.lifetime <= 0. {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct ShootSet;

pub(crate) struct ShootPlugin;

impl Plugin for ShootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShootTimer>()
            .add_system(shoot.in_set(ShootSet).after(MovementSet).before(HealthSet))
            .add_system(blood_particles.in_set(ShootSet).after(shoot));
    }
}
