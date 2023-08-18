use bevy::prelude::*;

use crate::labor::job::all_workers_eligible;

use super::job::JobAssignmentSet;

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (all_workers_eligible::<HaulRequest>).before(JobAssignmentSet),
        );
    }
}

pub enum HaulItem {
    Entity(Entity),
    ObjectType(Name),
}

#[derive(Component)]
pub struct HaulRequest {
    pub load: HaulItem,
    pub to: Entity,
}

impl HaulRequest {
    pub fn request_entity(load: Entity, to: Entity) -> Self {
        Self {
            load: HaulItem::Entity(load),
            to,
        }
    }

    pub fn request_object_type(load: Name, to: Entity) -> Self {
        Self {
            load: HaulItem::ObjectType(load),
            to,
        }
    }
}
