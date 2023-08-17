use bevy::{
    math::Vec3Swizzles,
    prelude::{Entity, GlobalTransform, Query},
};

pub fn get_entity_position(
    global_transform_query: &Query<'_, '_, &GlobalTransform>,
    entity: Entity,
) -> bevy::prelude::Vec2 {
    let entity_position = global_transform_query
        .get(entity)
        .unwrap()
        .translation()
        .xy();
    entity_position
}
