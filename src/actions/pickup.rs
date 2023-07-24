use bevy::prelude::*;

use crate::{actions::action::CompletedAction, labor::job::all_workers_eligible};

use super::{
    action::{register_action, ActionSet},
    move_to::{travel_to_entity, TravelAction},
};

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Pickup>().add_systems(
            (
                travel_to_entity::<Pickup>,
                all_workers_eligible::<Pickup>,
                pickup,
            )
                .before(ActionSet),
        );

        register_action::<Pickup, PickingUp>(app);
    }
}

#[derive(Component, Default, Debug)]
pub struct PickingUp;

#[derive(Component, Debug, Clone, Reflect)]
pub struct Pickup {
    pub amount: u32,
    pub from: Entity,
}

impl TravelAction for Pickup {
    type TravelingToTarget = TravellingToPickupSite;
    type AtTarget = AtPickupSite;

    fn target_entity(&self) -> Entity {
        self.from
    }
}

#[derive(Component, Debug, Default, Reflect)]
pub struct TravellingToPickupSite;

#[derive(Component, Default, Debug)]
pub struct AtPickupSite;

fn pickup(
    mut commands: Commands,
    pickup_job_query: Query<(Entity, &Pickup, &Parent), With<AtPickupSite>>,
    mut transform_query: Query<&mut Transform>,
) {
    for (pickup_action_entity, pickup_action, action_parent) in &pickup_job_query {
        let mut pickup_transform = transform_query.get_mut(pickup_action.from).unwrap();
        let load_entity = pickup_action.from;

        // Move item to worker inventory

        commands.entity(**action_parent).add_child(load_entity);

        *pickup_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 1.0));

        commands
            .entity(pickup_action_entity)
            .insert(CompletedAction);

        info!(performer=?**action_parent, action=?pickup_action_entity, item = ?load_entity, "Pickup complete");
    }
}
