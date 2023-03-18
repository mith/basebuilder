use bevy::{math::Vec3Swizzles, prelude::*};

use crate::movement::{MovementSet, Walker};

#[derive(Component)]
pub(crate) struct AiControlled;

#[derive(Component, Reflect)]
pub(crate) struct Target {
    pub(crate) entity: Option<Entity>,
    pub(crate) position: Vec2,
}

fn move_to_target(mut target_query: Query<(&Target, &mut Walker, &Transform)>) {
    for (target, mut controller, transform) in &mut target_query {
        let distance = target.position - transform.translation.xy();
        controller.move_direction = Some(distance.normalize());
    }
}

fn update_target(mut target_query: Query<&mut Target>, entity_query: Query<&Transform>) {
    for mut target in &mut target_query {
        if let Some(entity) = target.entity {
            if let Ok(entity_transform) = entity_query.get(entity) {
                target.position = entity_transform.translation.xy();
            }
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct AiControllerSet;

pub(crate) struct AiControllerPlugin;

impl Plugin for AiControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Target>().add_systems(
            (update_target, move_to_target)
                .chain()
                .in_set(AiControllerSet)
                .before(MovementSet),
        );
    }
}
