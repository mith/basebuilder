use bevy::prelude::*;

pub(crate) struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Health>()
            .add_event::<HealthDamageEvent>()
            .add_systems(
                (update_health, despawn_dead_entities)
                    .chain()
                    .in_set(HealthSet),
            );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HealthSet;
#[derive(Component, Reflect)]
pub(crate) struct Health(pub(crate) u32);

pub(crate) struct HealthDamageEvent {
    pub(crate) entity: Entity,
    pub(crate) damage: u32,
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

fn despawn_dead_entities(mut commands: Commands, mut health_query: Query<(Entity, &Health)>) {
    for (entity, health) in &mut health_query {
        if health.0 == 0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
