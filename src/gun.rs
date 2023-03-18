use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

use crate::{
    hit::{HitEvent, HitSet},
    movement::{AimingAt, MovementSet},
};

#[derive(Component, Default)]
pub(crate) struct Gun {
    pub(crate) fire: bool,
    reload_timer: Option<Timer>,
}

#[derive(Component)]
pub(crate) struct Muzzle;

#[derive(Component)]
pub(crate) struct MuzzleFlash {
    pub(crate) life_timer: Timer,
}

fn fire(
    mut commands: Commands,
    shooter_query: Query<(&AimingAt, Entity)>,
    children_query: Query<&Children>,
    mut gun_query: Query<(&mut Gun, &Children)>,
    muzzle_query: Query<&GlobalTransform, With<Muzzle>>,
    mut hit_events: EventWriter<HitEvent>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (aiming_at, shooter_entity) in &shooter_query {
        for descendant in children_query.iter_descendants(shooter_entity) {
            if let Ok((gun, gun_children)) = gun_query.get_mut(descendant).as_mut() {
                if let Some(reload_timer) = gun.reload_timer.as_mut() {
                    if reload_timer.tick(time.delta()).just_finished() {
                        gun.reload_timer = None;
                    }
                } else if gun.fire {
                    gun.reload_timer = Some(Timer::from_seconds(0.2, TimerMode::Once));
                    hit_events.send(HitEvent {
                        entity: aiming_at.target,
                        intersection: aiming_at.intersection,
                        damage: 25,
                    });

                    // spawn muzzle flash
                    let muzzle_global_transform = muzzle_query.get(gun_children[0]).unwrap();
                    commands.spawn((
                        MaterialMesh2dBundle {
                            material: materials.add(Color::rgb(1., 0.5, 0.).into()),
                            transform: Transform::from_translation(
                                muzzle_global_transform.translation(),
                            ),
                            mesh: meshes
                                .add(Mesh::from(shape::Quad::new(Vec2::new(2., 2.))))
                                .into(),
                            ..Default::default()
                        },
                        MuzzleFlash {
                            life_timer: Timer::from_seconds(0.1, TimerMode::Once),
                        },
                    ));
                }
            }
        }
    }
}

fn muzzle_flash(
    mut commands: Commands,
    mut muzzle_flash_query: Query<(Entity, &mut MuzzleFlash)>,
    time: Res<Time>,
) {
    for (entity, mut muzzle_flash) in &mut muzzle_flash_query {
        if muzzle_flash.life_timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct GunSet;

pub(crate) struct GunPlugin;

impl Plugin for GunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((fire.after(MovementSet).before(HitSet), muzzle_flash).in_set(GunSet));
    }
}
