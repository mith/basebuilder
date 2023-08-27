use bevy::{
    ecs::system::SystemParam,
    math::Vec3Swizzles,
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform,
        IntoSystemConfigs, Plugin, PreUpdate, Query, Res, Update, Vec2, With, World,
    },
    reflect::Reflect,
    time::{Time, Timer, TimerMode},
};
use bevy_ecs_tilemap::tiles::TilePos;
use big_brain::{
    actions::StepsBuilder,
    prelude::{ActionBuilder, ActionState, Steps},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::{debug, error, info, trace};

use crate::{
    actions::action_area::ActionArea,
    actions::move_to::follow_path,
    movement::Walker,
    pathfinding::Pathfinding,
    terrain::{TerrainParams, TerrainSet, TileDamageEvent, TileDestroyedEvent},
    util::get_entity_position,
};

use super::action_area::{HasActionArea, HasActionPosition};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Dig>()
            .register_type::<DigTimer>()
            .add_systems(PreUpdate, (move_to_tile, dig).in_set(BigBrainSet::Actions))
            .add_systems(Update, dig_timer.before(TerrainSet));
    }
}

#[derive(Component, Debug, Reflect)]
pub struct Dig(pub Entity);

#[derive(Debug, Clone)]
pub struct DigActionBuilder;

impl ActionBuilder for DigActionBuilder {
    fn build(&self, cmd: &mut Commands, action: Entity, actor: Entity) {
        cmd.entity(actor).add(move |id: Entity, world: &mut World| {
            let DigTarget(tile) = world
                .get::<DigTarget>(id)
                .expect("Actor should have a fell target")
                .to_owned();
            world.entity_mut(action).insert(Dig(tile));
        });
    }
}

impl HasActionArea for Dig {
    fn action_area(&self, action_pos_query: &Self::PositionQuery<'_, '_>) -> Option<ActionArea> {
        let action_pos = self.action_pos(action_pos_query)?;
        let x = action_pos.x;
        let y = action_pos.y;
        Some(ActionArea(vec![
            // West
            Vec2::new(x - 16., y),
            // East
            Vec2::new(x + 16., y),
            // South
            Vec2::new(x, y - 16.),
            // Northwest
            Vec2::new(x - 16., y + 16.),
            // Southwest
            Vec2::new(x - 16., y - 16.),
            // Southeast
            Vec2::new(x + 16., y - 16.),
            // Northeast
            Vec2::new(16., 16.),
        ]))
    }
}

#[derive(SystemParam)]
pub struct DigActionSystemParam<'w, 's> {
    tile_pos_query: Query<'w, 's, &'static TilePos>,
    terrain: TerrainParams<'w, 's>,
}

impl HasActionPosition for Dig {
    type PositionQuery<'w, 's> = DigActionSystemParam<'w, 's>;

    fn action_pos(&self, dig_action_params: &DigActionSystemParam) -> Option<Vec2> {
        dig_action_params
            .tile_pos_query
            .get(self.0)
            .ok()
            .map(|tile_pos| dig_action_params.terrain.tile_to_global_pos(*tile_pos))
    }
}
#[derive(Component, Debug, Clone, Reflect)]
pub struct DigTarget(pub Entity);

#[derive(Component, Clone, Debug, ActionBuilder)]
struct MoveTotile;

fn dig_work_area(tile_pos: TilePos, terrain: &TerrainParams) -> ActionArea {
    let tile_global_pos = terrain.tile_to_global_pos(tile_pos);
    // All sites around the target tile
    // NW, NE, E, SE, S, SW, W
    ActionArea(vec![
        tile_global_pos + Vec2::new(-1., 1.) * 16.,
        tile_global_pos + Vec2::new(1., 1.) * 16.,
        tile_global_pos + Vec2::new(1., 0.) * 16.,
        tile_global_pos + Vec2::new(1., -1.) * 16.,
        tile_global_pos + Vec2::new(0., -1.) * 16.,
        tile_global_pos + Vec2::new(-1., -1.) * 16.,
        tile_global_pos + Vec2::new(-1., 0.) * 16.,
    ])
}

fn at_work_area(actor_position: Vec2, work_area: &ActionArea) -> bool {
    // if we're close to a job site, we're done
    work_area
        .0
        .iter()
        .any(|&site| Vec2::new(site.x, 0.).distance(Vec2::new(actor_position.x, 0.)) < 5.)
}

fn move_to_tile(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<MoveTotile>>,
    global_transform_query: Query<&GlobalTransform>,
    dig_target_query: Query<&DigTarget>,
    pathfinding: Pathfinding,
    tile_query: Query<&TilePos>,
    terrain: TerrainParams,
    mut walker_query: Query<&mut Walker>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting move to tile");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let actor_position = global_transform_query
                    .get(actor.0)
                    .unwrap()
                    .translation()
                    .xy();
                let Some(dig_target) = dig_target_query.get(actor.0).ok() else {
                    error!("No dig target");
                    *action_state = ActionState::Failure;
                    continue;
                };
                if tile_query.get(dig_target.0).is_err() {
                    info!("tile no longer exists");
                    *action_state = ActionState::Cancelled;
                    continue;
                }
                let Some(tile_pos) = terrain.get_entity_tile_pos(dig_target.0) else {
                    error!("Tile has no position");
                    *action_state = ActionState::Failure;
                    continue;
                };
                let work_area = dig_work_area(tile_pos, &terrain);
                // if we're close to a job site, we're done
                if at_work_area(actor_position, &work_area) {
                    info!("Arrived at tile");
                    let mut walker = walker_query
                        .get_mut(actor.0)
                        .expect("Actor should have a walker");

                    walker.move_direction = None;
                    *action_state = ActionState::Success;
                } else {
                    debug!("Moving to tile");
                    let path = work_area
                        .0
                        .iter()
                        .find_map(|&site| pathfinding.find_path(actor_position, site));
                    if let Some(path) = path {
                        let mut walker = walker_query
                            .get_mut(actor.0)
                            .expect("Actor should have a walker");

                        debug!("Following path to tile");
                        follow_path(path, &mut walker, actor_position, &terrain);
                    } else {
                        error!(actor_position=?actor_position, work_area=?work_area, "No path found to tile");
                        *action_state = ActionState::Failure;
                    }
                }
            }
            ActionState::Cancelled => {
                info!("Cancelling move to tile");
                let mut walker = walker_query
                    .get_mut(actor.0)
                    .expect("Actor should have a walker");

                walker.move_direction = None;

                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub struct DigTimer {
    pub tile_entity: Entity,
    pub timer: Timer,
}

fn dig(
    mut commands: Commands,
    mut dig_action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<Dig>>,
    global_transform_query: Query<&GlobalTransform>,
    dig_target_query: Query<&DigTarget>,
    tile_query: Query<&TilePos>,
    terrain: TerrainParams,
    mut dig_timer_query: Query<&mut DigTimer>,
    mut tile_destroyed_event_reader: EventReader<TileDestroyedEvent>,
) {
    for (actor, mut action_state, span) in &mut dig_action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting digging");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                trace!("Digging");
                let Ok(dig_target) = dig_target_query.get(actor.0) else {
                    info!("No dig target");
                    *action_state = ActionState::Cancelled;
                    continue;
                };
                let actor_position = get_entity_position(&global_transform_query, actor.0);
                let Some(dig_target) = dig_target_query.get(actor.0).ok() else {
                    error!("No dig target");
                    *action_state = ActionState::Failure;
                    continue;
                };
                if tile_query.get(dig_target.0).is_err() {
                    info!("tile no longer exists");
                    *action_state = ActionState::Cancelled;
                    continue;
                }
                let Some(tile_pos) = terrain.get_entity_tile_pos(dig_target.0) else {
                    error!("Tile has no position");
                    *action_state = ActionState::Failure;
                    continue;
                };
                let work_area = dig_work_area(tile_pos, &terrain);
                // if we're close to a job site, we're done
                if at_work_area(actor_position, &work_area) {
                    if let Ok(dig_timer) = dig_timer_query.get_mut(actor.0) {
                        if let Some(_tile_destroyed_event) = tile_destroyed_event_reader
                            .iter()
                            .find(|event| event.entity == dig_timer.tile_entity)
                        {
                            info!("Digging finished");
                            commands
                                .entity(actor.0)
                                .remove::<DigTimer>()
                                .remove::<DigTarget>();
                            *action_state = ActionState::Success;
                        }
                    } else {
                        info!("Digging started");
                        commands.entity(actor.0).insert(DigTimer {
                            tile_entity: dig_target.0,
                            timer: Timer::from_seconds(1., TimerMode::Repeating),
                        });
                    }
                } else {
                    info!("Too far away to dig");
                    commands.entity(actor.0).remove::<DigTimer>();
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Digging cancelled");
                commands.entity(actor.0).remove::<DigTimer>();
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn dig_timer(
    time: Res<Time>,
    mut dig_action_query: Query<&mut DigTimer>,
    mut tile_damage_event_writer: EventWriter<TileDamageEvent>,
) {
    for mut dig_timer in &mut dig_action_query {
        if dig_timer.timer.tick(time.delta()).just_finished() {
            info!(tile_entity = ?dig_timer.tile_entity, "Digging tick");
            tile_damage_event_writer.send(TileDamageEvent {
                tile: dig_timer.tile_entity,
                damage: 20,
            });
        }
    }
}

pub fn dig_tile() -> StepsBuilder {
    Steps::build()
        .label("digger")
        .step(MoveTotile)
        .step(DigActionBuilder)
}
