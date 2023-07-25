use bevy::prelude::*;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Health>()
            .add_event::<HealthDamageEvent>()
            .add_systems(Update, update_health.in_set(HealthSet));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HealthSet;
#[derive(Component, Reflect)]
pub struct Health(pub u32);

#[derive(Event)]
pub struct HealthDamageEvent {
    pub entity: Entity,
    pub damage: u32,
}

fn update_health(
    mut health_damage_events: EventReader<HealthDamageEvent>,
    mut health_query: Query<&mut Health>,
) {
    for damage_event in health_damage_events.iter() {
        if let Ok(mut health) = health_query.get_mut(damage_event.entity) {
            health.0 = health.0.saturating_sub(damage_event.damage);
        }
    }
}
