use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{CollisionGroups, Group, RapierContext};

use crate::{
    cursor_position::LastCursorPosition,
    designation_layer::Designated,
    labor::job::{all_workers_eligible, CanceledJob, Job, JobSite},
    tree::{Tree, TreeDestroyedEvent, TREE_COLLISION_GROUP},
};

use super::job::{AssignedWorker, JobAssignmentSet, JobManagerParams};

pub struct ChopTreePlugin;

impl Plugin for ChopTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FellingCompleteEvent>()
            .add_state::<FellingToolState>()
            .register_type::<FellingJob>()
            .add_systems(
                Update,
                (
                    mark_trees.run_if(state_exists_and_equals(FellingToolState::Designating)),
                    all_workers_eligible::<FellingJob>,
                    cancel_felling_jobs,
                )
                    .before(JobAssignmentSet),
            );
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct FellingJob(pub Entity);

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
    cursor_position: Res<LastCursorPosition>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    tree_query: Query<&GlobalTransform, With<Tree>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        rapier_context.intersections_with_point(
            Vec2::new(cursor_position.0.x, cursor_position.0.y),
            CollisionGroups::new(PICKER_COLLISION_GROUP, TREE_COLLISION_GROUP).into(),
            |hit_entity| {
                let Ok((tree_entity, tree_transform)) =
                    parent_query.get(hit_entity).and_then(|parent| {
                        tree_query
                            .get(**parent)
                            .map(|tree_transform| (**parent, tree_transform))
                    }) else {
                    error!("Tree entity not found");
                    return true;
                };
                let tree_translation = tree_transform.translation().xy();
                commands.entity(tree_entity).insert(Designated);
                let job_site = JobSite(vec![
                            Vec2::new(tree_translation.x - 16., tree_translation.y),
                            Vec2::new(tree_translation.x + 16., tree_translation.y),
                        ]);
                let job_entity = commands
                    .spawn((
                        Job,
                        FellingJob(tree_entity),
                        job_site.clone(),
                    ))
                    .id();
                info!(job = ?job_entity, tree=?tree_entity, job_site=?job_site, "Marked tree for felling");
                false
            },
        );
    }
}

#[derive(Event)]
pub struct FellingCompleteEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub tree: Entity,
}

fn cancel_felling_jobs(
    mut job_manager_params: JobManagerParams,
    mut job_query: Query<(Entity, &FellingJob), With<Job>>,
    tree_query: Query<&Tree>,
) {
    for (job_entity, felling_job) in &mut job_query {
        if tree_query.get(felling_job.0).is_err() {
            info!(job = ?job_entity, tree = ?felling_job.0, "Cancelling felling job because tree does not exist");
            job_manager_params.cancel_job(job_entity);
        }
    }
}
