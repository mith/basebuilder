use bevy::{math::Vec3Swizzles, prelude::*, transform::commands};
use bevy_ecs_tilemap::prelude::TilemapSize;
use bevy_rapier2d::prelude::{CollisionGroups, Group, QueryFilter, RapierContext};

use crate::{
    ai_controller::{MoveTo, Path},
    climbable::{self, ClimbableMap},
    cursor_position::CursorPosition,
    designation_layer::Designated,
    dig::JobTimer,
    job::{Accessible, AssignedTo, HasJob, Job, Worker},
    pathfinding::{can_stand_or_climb, find_path},
    terrain::{Terrain, TerrainParams, TerrainSet, TERRAIN_COLLISION_GROUP},
    tree::{TreeDamageEvent, TreeDestroyedEvent, TREE_COLLISION_GROUP},
};

pub struct FellingPlugin;

impl Plugin for FellingPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<FellingToolState>().add_systems((
            mark_trees.run_if(state_exists_and_equals(FellingToolState::Designating)),
            fell,
            felling_timer.before(TerrainSet),
            finish_felling,
        ));
    }
}

#[derive(Component)]
pub struct FellTarget;

#[derive(States, Default, Reflect, Clone, Eq, PartialEq, Hash, Debug)]
pub enum FellingToolState {
    #[default]
    Inactive,
    Designating,
}

pub const PICKER_COLLISION_GROUP: Group = Group::GROUP_4;

fn mark_trees(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    rapier_context: Res<RapierContext>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        rapier_context.intersections_with_point(
            Vec2::new(cursor_position.0.x as f32, cursor_position.0.y as f32),
            CollisionGroups::new(PICKER_COLLISION_GROUP, TREE_COLLISION_GROUP).into(),
            |entity| {
                commands
                    .entity(entity)
                    .insert((Job, FellTarget, Designated, Accessible));
                false
            },
        );
    }
}

#[derive(Component)]
pub struct Felling(Entity);

#[derive(Component)]
pub struct FellingTimer(pub Timer);

fn fell(
    mut commands: Commands,
    assigned_felling_job_query: Query<
        (Entity, &GlobalTransform, &AssignedTo, &FellTarget),
        With<Job>,
    >,
    feller_query: Query<
        (&GlobalTransform, Option<(&MoveTo, &Path)>),
        (With<Worker>, Without<Felling>),
    >,
    terrain: TerrainParams,
    rapier_context: Res<RapierContext>,
    climbable_map_query: Query<&ClimbableMap, With<Terrain>>,
) {
    for (felling_job_entity, felling_job_transform, assigned_to, felling_target) in
        &assigned_felling_job_query
    {
        if let Ok((worker_transform, opt_move)) = feller_query.get(assigned_to.entity) {
            let climbable_map = climbable_map_query.single();
            let terrain_data = terrain.terrain_data_query.single();

            if let Some((move_to, path)) = opt_move {
                if path.0.is_empty() {
                    commands
                        .entity(assigned_to.entity)
                        .remove::<MoveTo>()
                        .remove::<Path>()
                        .insert((
                            Felling(felling_job_entity),
                            JobTimer(Timer::from_seconds(1.0, TimerMode::Repeating)),
                        ));
                }
                if move_to
                    .position
                    .distance(worker_transform.translation().xy())
                    < 26.
                {
                    commands
                        .entity(assigned_to.entity)
                        .remove::<MoveTo>()
                        .remove::<Path>()
                        .insert((
                            Felling(felling_job_entity),
                            JobTimer(Timer::from_seconds(1.0, TimerMode::Repeating)),
                        ));
                }
            } else {
                let worker_tile_pos = terrain
                    .global_to_tile_pos(worker_transform.translation().xy())
                    .unwrap();

                // check if the worker can stand on either of the tiles next to the tree
                let mut accessible_tiles = Vec::new();
                for &x in [-16., 16.].iter() {
                    let x = felling_job_transform.translation().x + x;
                    let y = felling_job_transform.translation().y;
                    let ray_dir = Vec2::new(0., -1.);
                    let max_toi = 90.;
                    let filter: QueryFilter =
                        CollisionGroups::new(PICKER_COLLISION_GROUP, TERRAIN_COLLISION_GROUP)
                            .into();

                    if let Some(intersection) = rapier_context.cast_ray_and_get_normal(
                        Vec2::new(x, y),
                        ray_dir,
                        max_toi,
                        true,
                        filter,
                    ) {
                        let hit_tile_pos =
                            terrain.global_to_tile_pos(intersection.1.point).unwrap();

                        // check if the worker can stand in the tile above the hit tile
                        let possible_tile = UVec2::new(hit_tile_pos.x, hit_tile_pos.y + 1);
                        if can_stand_or_climb(
                            terrain_data,
                            Some(climbable_map),
                            possible_tile.into(),
                        ) {
                            accessible_tiles.push(possible_tile);
                        }
                    }
                }
                if let Some((closest_accessible_tile, path)) = accessible_tiles
                    .iter()
                    .map(|&tile| {
                        (
                            tile,
                            find_path(
                                terrain_data,
                                Some(climbable_map),
                                worker_tile_pos.into(),
                                tile,
                            ),
                        )
                    })
                    .filter(|(_, path)| path.len() > 0)
                    .min_by_key(|tile| tile.1.len())
                {
                    let tile_global_position =
                        terrain.tile_to_global_pos(closest_accessible_tile.into());
                    commands.entity(assigned_to.entity).insert((
                        MoveTo {
                            entity: Some(felling_job_entity),
                            position: tile_global_position,
                        },
                        Path(path),
                    ));
                }
            }
        }
    }
}

fn felling_timer(
    time: Res<Time>,
    mut feller_query: Query<(&mut JobTimer, &mut Felling)>,
    mut tree_damage_event_writer: EventWriter<TreeDamageEvent>,
) {
    for (mut timer, felling) in &mut feller_query {
        if timer.0.tick(time.delta()).just_finished() {
            tree_damage_event_writer.send(TreeDamageEvent {
                tree: felling.0,
                damage: 10,
            });
        }
    }
}

fn finish_felling(
    mut commands: Commands,
    mut tree_destroyed_event_reader: EventReader<TreeDestroyedEvent>,
    feller_query: Query<(Entity, &Felling), With<Worker>>,
    mut unassigned_workers: RemovedComponents<HasJob>,
) {
    for unassigned_worker in &mut unassigned_workers.iter() {
        if feller_query.get(unassigned_worker).is_ok() {
            commands.entity(unassigned_worker).insert(Job);
        }
    }

    for tree_destroyed_event in tree_destroyed_event_reader.iter() {
        for (feller, felling) in &mut feller_query.iter() {
            if felling.0 == tree_destroyed_event.tree {
                remove_feller_job(&mut commands, feller);
            }
        }
    }
}

fn remove_feller_job(commands: &mut Commands, feller: Entity) {
    commands
        .entity(feller)
        .remove::<Felling>()
        .remove::<JobTimer>()
        .remove::<HasJob>()
        .remove::<MoveTo>()
        .remove::<Path>();
}
