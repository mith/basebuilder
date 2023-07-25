use bevy::{
    prelude::{
        App, BuildChildren, Commands, Component, Entity, IntoSystemConfigs, Parent, Plugin, Query,
        Update, With,
    },
    reflect::Reflect,
};
use tracing::info;

use crate::actions::action::CompletedAction;

use super::{
    action::{register_action, ActionSet},
    move_to::{travel_to_entity, TravelAction},
};

pub struct DeliverPlugin;

impl Plugin for DeliverPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Deliver>()
            .register_type::<Delivering>()
            .add_systems(
                Update,
                (travel_to_entity::<Deliver>, complete_delivery).before(ActionSet),
            );

        register_action::<Deliver, Delivering>(app);
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct Deliver {
    pub load: Entity,
    pub to: Entity,
}

impl TravelAction for Deliver {
    type TravelingToTarget = TravelingToDeliverySite;
    type AtTarget = AtDeliverySite;

    fn target_entity(&self) -> Entity {
        self.to
    }
}

#[derive(Component, Default, Debug, Reflect)]
pub struct Delivering;

#[derive(Component, Default, Debug, Reflect)]
pub struct TravelingToDeliverySite;

#[derive(Component, Default, Debug, Reflect)]
pub struct AtDeliverySite;

fn complete_delivery(
    mut commands: Commands,
    deliver_action_query: Query<(Entity, &Deliver, &Parent), With<AtDeliverySite>>,
) {
    for (deliver_action_entity, delivery, deliverer_entity) in &deliver_action_query {
        commands.entity(delivery.to).add_child(delivery.load);

        commands
            .entity(deliver_action_entity)
            .insert(CompletedAction);

        info!(worker=?deliverer_entity, delivery=?delivery, "Delivery complete");
    }
}
