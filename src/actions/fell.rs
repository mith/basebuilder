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
    labor::job::{self, AssignedJob, JobSite},
    movement::Walker,
    pathfinding::Pathfinding,
    terrain::TerrainParams,
    tree::{Tree, TreeDestroyedEvent},
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
    tree_query: Query<&Tree>,
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

                if let Some(_tree_destroyed_event) = tree_destroyed_event_reader
                    .iter()
                    .find(|event| event.tree == fell_target.0)
                {
                    finish_felling(&mut commands, actor, &mut action_state);
                    continue;
                }

                if tree_query.get(fell_target.0).is_err() {
                    info!("Tree no longer exists");
                    *action_state = ActionState::Failure;
                    continue;
                }

                let job_site = fell_job_site(fell_target.0, &global_transform_query);

                if at_job_site(actor_position, &job_site) {
                    if fell_timer_query.get(actor.0).is_err() {
                        info!("Felling started");
                        commands.entity(actor.0).insert(FellingTimer {
                            tree_entity: fell_target.0,
                            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                        });
                    }
                } else {
                    info!("Too far away to fell");
                    commands
                        .entity(actor.0)
                        .remove::<FellingTimer>()
                        .remove::<FellTarget>();
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

fn finish_felling(commands: &mut Commands, actor: &Actor, action_state: &mut ActionState) {
    info!("Felling finished");
    commands
        .entity(actor.0)
        .remove::<FellingTimer>()
        .remove::<FellTarget>();
    *action_state = ActionState::Success;
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
    job_site
        .0
        .iter()
        .any(|&site| Vec2::new(site.x, 0.).distance(Vec2::new(actor_position.x, 0.)) < 5.)
}

#[derive(Component, Clone, Debug, ActionBuilder)]
struct MoveToTree;

fn fell_job_site(target: Entity, global_transform_query: &Query<&GlobalTransform>) -> JobSite {
    let target_position = global_transform_query
        .get(target)
        .unwrap()
        .translation()
        .xy();
    JobSite(vec![
        target_position - Vec2::new(16., 0.),
        target_position + Vec2::new(16., 0.),
    ])
}

fn move_to_tree(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<MoveToTree>>,
    global_transform_query: Query<&GlobalTransform>,
    fell_target_query: Query<&FellTarget>,
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
                let Some(fell_target) = fell_target_query.get(actor.0).ok() else {
                    error!("No fell target");
                    *action_state = ActionState::Failure;
                    continue;
                };
                let job_site = fell_job_site(fell_target.0, &global_transform_query);
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
                    let path = job_site
                        .0
                        .iter()
                        .find_map(|&site| pathfinding.find_path(actor_position, site));
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
