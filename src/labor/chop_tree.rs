use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{CollisionGroups, Group, RapierContext};

use crate::{
    cursor_position::CursorPosition,
    designation_layer::Designated,
    labor::job::{all_workers_eligible, AssignedJob, AtJobSite, Complete, Job, JobSite, Worker},
    terrain::TerrainSet,
    tree::{Tree, TreeDamageEvent, TreeDestroyedEvent, TREE_COLLISION_GROUP},
};

use super::job::{register_job, JobAssignmentSet};

pub struct FellingPlugin;

impl Plugin for FellingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FellingCompleteEvent>()
            .add_state::<FellingToolState>()
            .register_type::<FellingJob>()
            .register_type::<FellingTimer>()
            .register_type::<Felling>()
            .add_systems(
                (
                    mark_trees.run_if(state_exists_and_equals(FellingToolState::Designating)),
                    all_workers_eligible::<FellingJob>,
                    start_felling,
                    felling_timer.before(TerrainSet),
                    finish_felling,
                )
                    .before(JobAssignmentSet),
            );

        register_job::<FellingJob, Feller>(app);
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct FellingJob(Entity);

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
    tree_query: Query<&GlobalTransform, With<Tree>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        rapier_context.intersections_with_point(
            Vec2::new(cursor_position.0.x as f32, cursor_position.0.y as f32),
            CollisionGroups::new(PICKER_COLLISION_GROUP, TREE_COLLISION_GROUP).into(),
            |tree_entity| {
                let Ok(tree_transform) = tree_query.get(tree_entity) else {
                    return true;
                };
                let tree_translation = tree_transform.translation().xy();
                let tree_size = 180.;
                info!(tree=?tree_entity, "Marking tree for felling");
                commands.entity(tree_entity).insert(Designated);
                commands.spawn((
                    Job,
                    FellingJob(tree_entity),
                    JobSite(vec![
                        Vec2::new(
                            tree_translation.x - 16.,
                            tree_translation.y - (tree_size / 2.) + 8.,
                        ),
                        Vec2::new(
                            tree_translation.x + 16.,
                            tree_translation.y - (tree_size / 2.) + 8.,
                        ),
                    ]),
                ));
                false
            },
        );
    }
}

#[derive(Component, Default, Debug)]
struct Feller;

#[derive(Component, Debug, Reflect)]
pub struct Felling(Entity);

#[derive(Component, Debug, Reflect)]
pub struct FellingTimer(pub Timer);

fn start_felling(
    mut commands: Commands,
    feller_query: Query<(Entity, &AssignedJob), (With<Feller>, Without<Felling>, With<AtJobSite>)>,
    felling_job_query: Query<&FellingJob>,
) {
    for (feller_entity, assigned_job) in &feller_query {
        let Ok(felling_job) = felling_job_query.get(assigned_job.0) else {
            error!("Feller {:?} has no felling job", feller_entity);
            continue;
        };
        info!(feller=?feller_entity, felling_job=?felling_job, "Starting felling");
        commands.entity(feller_entity).insert((
            Felling(felling_job.0),
            FellingTimer(Timer::from_seconds(1.0, TimerMode::Repeating)),
        ));
    }
}

fn felling_timer(
    time: Res<Time>,
    mut feller_query: Query<(&mut FellingTimer, &mut Felling)>,
    mut tree_damage_event_writer: EventWriter<TreeDamageEvent>,
) {
    for (mut timer, felling) in &mut feller_query {
        if timer.0.tick(time.delta()).just_finished() {
            info!(felling=?felling, "Felling tick");
            tree_damage_event_writer.send(TreeDamageEvent {
                tree: felling.0,
                damage: 20,
            });
        }
    }
}

pub struct FellingCompleteEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub tree: Entity,
}

fn finish_felling(
    mut commands: Commands,
    mut tree_destroyed_event_reader: EventReader<TreeDestroyedEvent>,
    feller_query: Query<(Entity, &Felling, &AssignedJob), With<Worker>>,
    parent_job_query: Query<&Parent, With<Job>>,
    mut felling_complete_event_writer: EventWriter<FellingCompleteEvent>,
) {
    for tree_destroyed_event in tree_destroyed_event_reader.iter() {
        for (feller, felling, AssignedJob(felling_job)) in &mut feller_query.iter() {
            if felling.0 == tree_destroyed_event.tree {
                // Mark the job as complete
                commands.entity(*felling_job).insert(Complete);
                // Remove the feller and felling from worker
                commands
                    .entity(feller)
                    .remove::<Feller>()
                    .remove::<Felling>()
                    .remove::<FellingTimer>();

                // Check if the parent is a job
                let parent_job = parent_job_query.get(*felling_job).ok().map(|p| p.get());

                info!(feller=?feller, felling_job=?felling_job, "Felling complete");

                felling_complete_event_writer.send(FellingCompleteEvent {
                    job: *felling_job,
                    parent_job,
                    worker: feller,
                    tree: tree_destroyed_event.tree,
                });
            }
        }
    }
}
