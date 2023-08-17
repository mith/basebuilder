use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{CollisionGroups, Group, RapierContext};

use crate::{
    actions::work::FellJob,
    cursor_position::LastCursorPosition,
    designation_layer::Designated,
    labor::job::{all_workers_eligible, Complete, Job, JobSite},
    tree::{Tree, TreeDestroyedEvent, TREE_COLLISION_GROUP},
};

use super::job::{AssignedWorker, JobAssignmentSet};

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
                    finish_felling_job,
                )
                    .before(JobAssignmentSet),
            );
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
                        FellJob { tree: tree_entity },
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

fn finish_felling_job(
    mut commands: Commands,
    mut tree_destroyed_event_reader: EventReader<TreeDestroyedEvent>,
    felling_job_query: Query<(Entity, &FellJob, &AssignedWorker, Option<&Parent>)>,
    mut felling_complete_event_writer: EventWriter<FellingCompleteEvent>,
) {
    for tree_destroyed_event in tree_destroyed_event_reader.iter() {
        if let Some((fell_job_entity, fell_job, AssignedWorker(feller_entity), parent)) =
            felling_job_query
                .iter()
                .find(|(_, fell_job, _, _)| fell_job.tree == tree_destroyed_event.tree)
        {
            debug_assert!(tree_destroyed_event.tree == fell_job.tree, "Tree mismatch");

            // Mark the job as complete
            commands.entity(fell_job_entity).insert(Complete);

            info!(feller=?feller_entity, felling_job=?fell_job_entity, "Felling complete");

            felling_complete_event_writer.send(FellingCompleteEvent {
                job: fell_job_entity,
                parent_job: parent.map(|p| p.get()),
                worker: *feller_entity,
                tree: tree_destroyed_event.tree,
            });
        }
    }
}
