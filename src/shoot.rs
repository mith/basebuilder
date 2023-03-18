use bevy::prelude::*;

use crate::{
    health::{Health, HealthDamageEvent, HealthSet},
    movement::{AimingAt, MovementSet},
};

#[derive(Component, Default)]
pub(crate) struct Shooter {
    pub(crate) shoot: bool,
}

#[derive(Resource)]
struct ShootTimer(Timer);

impl Default for ShootTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.2, TimerMode::Repeating))
    }
}

fn shoot(
    dude_query: Query<(&Shooter, &AimingAt)>,
    health_query: Query<Entity, With<Health>>,
    mut health_damage_events: EventWriter<HealthDamageEvent>,
    mut timer: ResMut<ShootTimer>,
    time: Res<Time>,
) {
    for (dude_input, aiming_at) in &dude_query {
        if dude_input.shoot && health_query.contains(aiming_at.0) {
            if timer.0.tick(time.delta()).just_finished() {
                health_damage_events.send(HealthDamageEvent {
                    entity: aiming_at.0,
                    damage: 25,
                });
            }
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct ShootSet;

pub(crate) struct ShootPlugin;

impl Plugin for ShootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShootTimer>()
            .add_system(shoot.in_set(ShootSet).after(MovementSet).before(HealthSet));
    }
}
