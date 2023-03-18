use std::f32::consts::PI;

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
    shooter_query: Query<&AimingAt>,
    parents_query: Query<&Parent>,
    mut gun_query: Query<(Entity, &mut Gun, &Children)>,
    muzzle_query: Query<&GlobalTransform, With<Muzzle>>,
    mut hit_events: EventWriter<HitEvent>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (gun_entity, mut gun, gun_children) in &mut gun_query {
        if let Some(reload_timer) = gun.reload_timer.as_mut() {
            if reload_timer.tick(time.delta()).just_finished() {
                gun.reload_timer = None;
            }
        } else if gun.fire {
            gun.reload_timer = Some(Timer::from_seconds(0.2, TimerMode::Once));

            let maybe_shooter = parents_query
                .iter_ancestors(gun_entity)
                .find_map(|ancestor| shooter_query.get(ancestor).ok());
            if let Some(aiming_at) = maybe_shooter
            {
                hit_events.send(HitEvent {
                    entity: aiming_at.target,
                    intersection: aiming_at.intersection,
                    damage: 25,
                });
            }

            spawn_muzzle_flash(
                &muzzle_query,
                gun_children,
                &mut commands,
                &mut materials,
                &mut meshes,
            );
        }
    }
}

fn spawn_muzzle_flash(
    muzzle_query: &Query<&GlobalTransform, With<Muzzle>>,
    gun_children: &Children,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    let muzzle_global_transform = muzzle_query.get(gun_children[0]).unwrap();
    commands.spawn((
        MaterialMesh2dBundle {
            material: materials.add(Color::rgb(1., 0.5, 0.).into()),
            transform: muzzle_global_transform
                .mul_transform(Transform::from_rotation(Quat::from_rotation_z(-PI * 0.5)))
                .compute_transform(),
            mesh: meshes
                .add(Mesh::from(shape::RegularPolygon::new(2., 3)))
                .into(),
            ..Default::default()
        },
        MuzzleFlash {
            life_timer: Timer::from_seconds(0.05, TimerMode::Once),
        },
    ));
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
