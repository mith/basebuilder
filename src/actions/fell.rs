use bevy::{
    math::Vec3Swizzles,
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform,
        IntoSystemConfigs, Plugin, PreUpdate, Query, Res, Update, Vec2, With,
    },
    reflect::Reflect,
    time::{Time, Timer, TimerMode},
};
use big_brain::{
    actions::StepsBuilder,
    prelude::{ActionBuilder, ActionState, Steps},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::{debug, error, info};

use crate::{
    actions::move_to::follow_path,
    health::HealthDamageEvent,
    labor::job::{AssignedJob, JobSite},
    movement::Walker,
    pathfinding::Pathfinding,
    terrain::TerrainParams,
    tree::TreeDestroyedEvent,
    util::get_entity_position,
};

pub struct FellPlugin;

impl Plugin for FellPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Fell>()
            .register_type::<FellingTimer>()
            .register_type::<FellTarget>()
            .add_systems(PreUpdate, (fell, move_to_tree).in_set(BigBrainSet::Actions))
            .add_systems(Update, felling_timer);
    }
}

#[derive(Component, Clone, Debug, Reflect, ActionBuilder)]
pub struct Fell;

#[derive(Component, Debug, Clone, Reflect)]
pub struct FellTarget(pub Entity);

#[derive(Component, Debug, Reflect)]
pub struct FellingTimer {
    pub tree_entity: Entity,
    pub timer: Timer,
}

fn fell(
    mut commands: Commands,
    mut fell_action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<Fell>>,
    global_transform_query: Query<&GlobalTransform>,
    fell_target_query: Query<&FellTarget>,
    assigned_job_query: Query<&AssignedJob>,
    job_site_query: Query<&JobSite>,
    mut fell_timer_query: Query<&mut FellingTimer>,
    mut tree_destroyed_event_reader: EventReader<TreeDestroyedEvent>,
) {
    for (actor, mut action_state, span) in &mut fell_action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting felling");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                debug!("Felling");
                let actor_position = get_entity_position(&global_transform_query, actor.0);
                let Some(fell_target) = fell_target_query.get(actor.0).ok() else {
                    error!("No fell target");
                    *action_state = ActionState::Failure;
                    continue;
                };

                let job_site = assigned_job_query
                    .get(actor.0)
                    .and_then(|job| job_site_query.get(job.0))
                    .expect("Actor should have an assigned job with a job site");

                if at_job_site(actor_position, job_site) {
                    if let Some(fell_timer) = fell_timer_query.get_mut(actor.0).ok() {
                        if let Some(tree_destroyed_event) = tree_destroyed_event_reader
                            .iter()
                            .find(|event| event.tree == fell_timer.tree_entity)
                        {
                            info!("Felling finished");
                            commands.entity(actor.0).remove::<FellingTimer>();
                            *action_state = ActionState::Success;
                        }
                    } else {
                        info!("Felling started");
                        commands.entity(actor.0).insert(FellingTimer {
                            tree_entity: fell_target.0,
                            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                        });
                    }
                } else {
                    info!("Too far away to fell");
                    commands.entity(actor.0).remove::<FellingTimer>();
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Felling cancelled");
                commands.entity(actor.0).remove::<FellingTimer>();
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn felling_timer(
    time: Res<Time>,
    mut felling_action_query: Query<(Entity, &FellTarget, &mut FellingTimer)>,
    mut tree_damage_event_writer: EventWriter<HealthDamageEvent>,
) {
    for (action_entity, FellTarget(tree_entity), mut felling_timer) in &mut felling_action_query {
        if felling_timer.timer.tick(time.delta()).just_finished() {
            info!(action=?action_entity, tree=?tree_entity, "Felling tick");
            tree_damage_event_writer.send(HealthDamageEvent {
                entity: *tree_entity,
                damage: 20,
            });
        }
    }
}

fn at_job_site(actor_position: Vec2, job_site: &JobSite) -> bool {
    // if we're close to a job site, we're done
    if job_site
        .0
        .iter()
        .any(|&site| site.distance(actor_position) < 6.)
    {
        true
    } else {
        false
    }
}

#[derive(Component, Clone, Debug, ActionBuilder)]
struct MoveToTree;

fn move_to_tree(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<MoveToTree>>,
    global_transform_query: Query<&GlobalTransform>,
    assigned_job_query: Query<&AssignedJob>,

    fell_jobs_query: Query<&JobSite>,
    pathfinding: Pathfinding,
    terrain: TerrainParams,
    mut walker_query: Query<&mut Walker>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting move to tree");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let actor_position = global_transform_query
                    .get(actor.0)
                    .unwrap()
                    .translation()
                    .xy();

                let job_site = assigned_job_query
                    .get(actor.0)
                    .and_then(|assigned_job| fell_jobs_query.get(assigned_job.0))
                    .expect("Actor should have a FellJob assigned");

                // if we're close to a job site, we're done
                if at_job_site(actor_position, &job_site) {
                    info!("Arrived at tree");
                    let mut walker = walker_query
                        .get_mut(actor.0)
                        .expect("Actor should have a walker");

                    walker.move_direction = None;
                    *action_state = ActionState::Success;
                } else {
                    debug!("Moving to tree");
                    let path = job_site.0.iter().find_map(|&site| {
                        let path = pathfinding.find_path(actor_position, site);
                        path
                    });
                    if let Some(path) = path {
                        let mut walker = walker_query
                            .get_mut(actor.0)
                            .expect("Actor should have a walker");

                        debug!("Following path to tree");
                        follow_path(path, &mut walker, actor_position, &terrain);
                    } else {
                        error!(actor_position=?actor_position, job_site=?job_site, "No path found to tree");
                        *action_state = ActionState::Failure;
                    }
                }
            }
            ActionState::Cancelled => {
                info!("Cancelling move to tree");
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

pub fn fell_tree() -> StepsBuilder {
    Steps::build().label("feller").step(MoveToTree).step(Fell)
}
