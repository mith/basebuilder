use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, ExternalImpulse, RayIntersection, RigidBody};

use crate::health::{Health, HealthDamageEvent, HealthSet};

pub(crate) struct HitEvent {
    pub(crate) entity: Entity,
    pub(crate) intersection: RayIntersection,
    pub(crate) damage: u32,
}

#[derive(Component)]
pub(crate) struct Wound;

#[derive(Component)]
pub(crate) struct BloodParticle {
    pub(crate) life_timer: Timer,
}

fn hits(
    mut commands: Commands,
    mut hit_events: EventReader<HitEvent>,
    health_query: Query<&Health>,
    global_transform_query: Query<&GlobalTransform>,
    mut health_damage_events: EventWriter<HealthDamageEvent>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for hit_event in hit_events.iter() {
        if health_query.contains(hit_event.entity) {
            health_damage_events.send(HealthDamageEvent {
                entity: hit_event.entity,
                damage: hit_event.damage,
            });

            // spawn blood particle
            let particle_size = 1.;
            commands.spawn((
                BloodParticle {
                    life_timer: Timer::from_seconds(0.5, TimerMode::Once),
                },
                MaterialMesh2dBundle {
                    transform: Transform::from_xyz(
                        hit_event.intersection.point.x,
                        hit_event.intersection.point.y,
                        0.,
                    ),
                    material: materials.add(Color::RED.into()),
                    mesh: meshes
                        .add(Mesh::from(shape::Quad::new(Vec2::new(
                            particle_size,
                            particle_size,
                        ))))
                        .into(),
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::round_cuboid(particle_size / 2., particle_size / 2., 0.01),
                ExternalImpulse {
                    impulse: hit_event.intersection.normal * 0.01,
                    torque_impulse: 0.,
                },
            ));

            let target_global_transform = global_transform_query.get(hit_event.entity).unwrap();

            // spawn wound on target
            commands.entity(hit_event.entity).with_children(|target| {
                target.spawn((
                    Wound,
                    MaterialMesh2dBundle {
                        transform: Transform::from_xyz(
                            hit_event.intersection.point.x
                                - target_global_transform.translation().x,
                            hit_event.intersection.point.y
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

fn blood_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut BloodParticle)>,
    time: Res<Time>,
) {
    for (entity, mut blood_particle) in &mut query {
        if blood_particle.life_timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HitSet;

pub(crate) struct HitPlugin;

impl Plugin for HitPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HitEvent>()
            .add_systems((hits.before(HealthSet), blood_particles).in_set(HitSet));
    }
}
