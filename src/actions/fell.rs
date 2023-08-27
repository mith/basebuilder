use bevy::{
    math::Vec3Swizzles,
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform,
        IntoSystemConfigs, Plugin, PreUpdate, Query, Res, Update, Vec2, With, World,
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
    actions::action_area::ActionArea,
    health::HealthDamageEvent,
    tree::{Tree, TreeDestroyedEvent},
    util::get_entity_position,
};

use super::{
    action_area::{HasActionArea, HasActionPosition},
    move_to::{move_to_action_area, MoveToActionArea},
};

pub struct FellPlugin;

impl Plugin for FellPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Fell>()
            .register_type::<FellingTimer>()
            .register_type::<FellTarget>()
            .add_systems(
                PreUpdate,
                (fell, move_to_action_area::<Fell>).in_set(BigBrainSet::Actions),
            )
            .add_systems(Update, felling_timer);
    }
}

#[derive(Component, Debug, Reflect)]
pub struct Fell(pub Entity);

#[derive(Debug, Reflect)]
pub struct FellActionBuilder;

impl ActionBuilder for FellActionBuilder {
    fn build(&self, cmd: &mut Commands, action: Entity, actor: Entity) {
        cmd.entity(actor).add(move |id: Entity, world: &mut World| {
            let FellTarget(tree) = world
                .get::<FellTarget>(id)
                .expect("Actor should have a fell target")
                .to_owned();

            world.entity_mut(action).insert(Fell(tree));
        });
    }
}

impl HasActionArea for Fell {
    fn action_area(&self, action_pos_query: &Self::PositionQuery<'_, '_>) -> Option<ActionArea> {
        let action_pos = self.action_pos(action_pos_query)?;
        Some(ActionArea(vec![
            action_pos - Vec2::new(16., 0.),
            action_pos + Vec2::new(16., 0.),
        ]))
    }
}

impl HasActionPosition for Fell {
    type PositionQuery<'w, 's> = Query<'w, 's, &'static GlobalTransform>;
    fn action_pos(&self, global_transform_query: &Self::PositionQuery<'_, '_>) -> Option<Vec2> {
        global_transform_query
            .get(self.0)
            .map(|tree_transform| tree_transform.translation().xy())
            .ok()
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct FellTarget(pub Entity);

#[derive(Debug, Reflect)]
pub struct MoveToFellActionBuilder;

impl ActionBuilder for MoveToFellActionBuilder {
    fn build(&self, cmd: &mut Commands, action: Entity, actor: Entity) {
        cmd.entity(actor).add(move |id: Entity, world: &mut World| {
            let FellTarget(tree) = world
                .get::<FellTarget>(id)
                .expect("Actor should have a fell target")
                .to_owned();

            world
                .entity_mut(action)
                .insert(MoveToActionArea(Fell(tree)));
        });
    }
}

fn at_action_area(actor_position: Vec2, action_area: &ActionArea) -> bool {
    // if we're close to a action area, we're done
    action_area
        .0
        .iter()
        .any(|&tile| Vec2::new(tile.x, 0.).distance(Vec2::new(actor_position.x, 0.)) < 5.)
}

pub fn fell_action_area(target_position: Vec2) -> ActionArea {
    ActionArea(vec![
        target_position - Vec2::new(16., 0.),
        target_position + Vec2::new(16., 0.),
    ])
}

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
    fell_timer_query: Query<&mut FellingTimer>,
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

                let tree_global_pos = global_transform_query
                    .get(fell_target.0)
                    .expect("Tree should have a global transform")
                    .translation()
                    .xy();

                let action_area = fell_action_area(tree_global_pos);

                if at_action_area(actor_position, &action_area) {
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

pub fn fell_tree() -> StepsBuilder {
    Steps::build()
        .label("feller")
        .step(MoveToFellActionBuilder)
        .step(FellActionBuilder)
}
