use bevy::{math::Vec3Swizzles, prelude::*};

use crate::movement::{MovementSet, Walker};

#[derive(Component)]
pub(crate) struct AiControlled;

#[derive(Component, Reflect)]
pub(crate) struct MoveTo {
    pub(crate) entity: Option<Entity>,
    pub(crate) position: Vec2,
}

fn move_to_target(mut target_query: Query<(&MoveTo, &mut Walker, &Transform), With<AiControlled>>) {
    for (target, mut walker, transform) in &mut target_query {
        let distance = target.position - transform.translation.xy();
        walker.move_direction = Some(distance.normalize());
    }
}

fn update_target(mut target_query: Query<&mut MoveTo>, entity_query: Query<&Transform>) {
    for mut target in &mut target_query {
        if let Some(entity) = target.entity {
            if let Ok(entity_transform) = entity_query.get(entity) {
                target.position = entity_transform.translation.xy();
            }
        }
    }
}

fn move_to_removed(mut removed: RemovedComponents<MoveTo>, mut target_query: Query<&mut Walker>) {
    for entity in &mut removed {
        if let Ok(mut target) = target_query.get_mut(entity) {
            target.move_direction = None;
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct AiControllerSet;

pub(crate) struct AiControllerPlugin;

impl Plugin for AiControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveTo>().add_systems(
            (update_target, move_to_target, move_to_removed)
                .chain()
                .in_set(AiControllerSet)
                .before(MovementSet),
        );
    }
}
