use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{CollisionGroups, Group, RapierContext};

use crate::{
    cursor_position::CursorPosition,
    designation_layer::Designated,
    labor::job::{
        all_workers_eligible, job_assigned, AssignedJob, AtJobSite, Complete, Job, JobSite, Worker,
    },
    terrain::TerrainSet,
    tree::{Tree, TreeDamageEvent, TreeDestroyedEvent, TREE_COLLISION_GROUP},
};

pub struct FellingPlugin;

impl Plugin for FellingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FellingCompleteEvent>()
            .add_state::<FellingToolState>()
            .add_systems((
                mark_trees.run_if(state_exists_and_equals(FellingToolState::Designating)),
                job_assigned::<FellingJob, Feller>,
                all_workers_eligible::<FellingJob>,
                fell,
                felling_timer.before(TerrainSet),
                finish_felling,
            ));
    }
}

#[derive(Component)]
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

#[derive(Component, Default)]
struct Feller;

#[derive(Component)]
pub struct Felling(Entity);

#[derive(Component)]
pub struct FellingTimer(pub Timer);

fn fell(
    mut commands: Commands,
    feller_query: Query<(Entity, &AssignedJob), (With<Feller>, Without<Felling>, With<AtJobSite>)>,
    felling_job_query: Query<&FellingJob>,
) {
    for (feller_entity, assigned_job) in &feller_query {
        let Ok(felling_job) = felling_job_query.get(assigned_job.0) else {
            error!("Feller {:?} has no felling job", feller_entity);
            continue;
        };
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
        for (feller, felling, assigned_job) in &mut feller_query.iter() {
            if felling.0 == tree_destroyed_event.tree {
                let felling_job = assigned_job.0;
                // Mark the job as complete
                commands.entity(felling_job).insert(Complete);
                // Remove the feller and felling from worker
                commands
                    .entity(feller)
                    .remove::<Feller>()
                    .remove::<Felling>();

                // Check if the parent is a job
                let parent_job = parent_job_query.get(assigned_job.0).ok().map(|p| p.get());

                felling_complete_event_writer.send(FellingCompleteEvent {
                    job: felling_job,
                    parent_job,
                    worker: feller,
                    tree: tree_destroyed_event.tree,
                });
            }
        }
    }
}