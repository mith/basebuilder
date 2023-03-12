use bevy::{math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ecs_tilemap::tiles::TilePos;
use bevy_rapier2d::prelude::{Collider, KinematicCharacterController, RigidBody};

use crate::{
    cursor_position::CursorPosition,
    hovered_tile::HoveredTile,
    terrain::{TerrainSet, TileDamageEvent, TileDestroyedEvent},
};

#[derive(Component)]
pub struct Dude;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Dude,
        Name::new("Dude"),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(0., 53. * 16., 1.),
            material: materials.add(Color::GRAY.into()),
            mesh: meshes
                .add(Mesh::from(shape::Quad::new(Vec2::new(12., 36.))))
                .into(),
            ..default()
        },
        RigidBody::KinematicPositionBased,
        KinematicCharacterController::default(),
        Collider::round_cuboid(5., 16.5, 0.1),
    ));
}

fn move_dude_keyboard(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(Entity, &mut KinematicCharacterController), With<Dude>>,
) {
    for (dude_entity, mut controller) in &mut query {
        if keyboard_input.any_just_pressed([
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
        ]) {
            commands.entity(dude_entity).remove::<Target>();
        }

        if keyboard_input.pressed(KeyCode::Left) {
            controller.translation = Some(Vec2::new(-1., 0.));
        }
        if keyboard_input.pressed(KeyCode::Right) {
            controller.translation = Some(Vec2::new(1., 0.));
        }
        if keyboard_input.pressed(KeyCode::Up) {
            controller.translation = Some(Vec2::new(0., 1.));
        }
        if keyboard_input.pressed(KeyCode::Down) {
            controller.translation = Some(Vec2::new(0., -1.));
        }
    }
}

#[derive(Component, Reflect)]
struct Target {
    entity: Option<Entity>,
    position: Vec2,
}

#[derive(Component, Reflect)]
struct Mining {
    timer: Timer,
    target: Entity,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum DudeSet {
    Input,
    Action,
}

fn move_dude_to_target(
    mut commands: Commands,
    mut dude_query: Query<
        (
            Entity,
            &mut KinematicCharacterController,
            &Transform,
            &Target,
        ),
        (With<Dude>, Without<Mining>),
    >,
    hovered_tile_query: Query<Entity, With<TilePos>>,
) {
    for (dude_entity, mut controller, transform, target) in &mut dude_query {
        let distance = target.position - transform.translation.xy();
        if distance.length() < 30. {
            if let Some(target_entity) = target.entity {
                if hovered_tile_query.contains(target_entity) {
                    commands.entity(dude_entity).insert(Mining {
                        timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                        target: target_entity,
                    });
                }
            }
        } else {
            controller.translation = Some(distance.normalize());
        }
    }
}

fn dude_gravity(mut query: Query<&mut KinematicCharacterController, With<Dude>>) {
    for mut controller in &mut query {
        controller.translation = Some(
            controller
                .translation
                .map_or_else(|| Vec2::new(0., -1.), |t| t + Vec2::new(0., -1.)),
        );
    }
}

fn mining_tick(
    mut commands: Commands,
    mut mining_dudes: Query<(Entity, &mut Mining), With<Dude>>,
    mut tile_damage_events: EventWriter<TileDamageEvent>,
    time: Res<Time>,
    mut tiles_destroyed: EventReader<TileDestroyedEvent>,
) {
    let destroyed_tiles: Vec<Entity> = tiles_destroyed.iter().map(|e| e.0).collect();
    for (dude_entity, mut mining) in &mut mining_dudes {
        if destroyed_tiles.contains(&mining.target) {
            commands.entity(dude_entity).remove::<Mining>();
            continue;
        }
        if mining.timer.tick(time.delta()).finished() {
            tile_damage_events.send(TileDamageEvent {
                tile: mining.target,
                damage: 10,
            });
        }
    }
}

fn pick_target(
    mut commands: Commands,
    dude_query: Query<Entity, With<Dude>>,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    hovered_tile_query: Query<Entity, With<HoveredTile>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let target_position = Vec2::new(cursor_position.0.x, cursor_position.0.y);
        for dude_entity in &dude_query {
            commands.entity(dude_entity).insert(Target {
                entity: hovered_tile_query.get_single().ok(),
                position: target_position,
            });
        }
    }
}

pub(crate) struct DudePlugin;

impl Plugin for DudePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Target>()
            .register_type::<Mining>()
            .add_startup_system(setup)
            .add_system(move_dude_keyboard.in_set(DudeSet::Input))
            .add_system(pick_target.in_set(DudeSet::Input))
            .add_system(move_dude_to_target.after(DudeSet::Input))
            .add_system(dude_gravity.after(move_dude_to_target))
            .add_system(
                mining_tick
                    .before(TerrainSet::Update)
                    .in_set(DudeSet::Action),
            );
    }
}
