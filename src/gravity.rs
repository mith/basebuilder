use bevy::prelude::*;
use bevy_rapier2d::prelude::KinematicCharacterController;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, gravity.in_set(GravitySet));
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct GravitySet;

#[derive(Component)]
pub struct Gravity;

fn gravity(mut query: Query<&mut KinematicCharacterController, With<Gravity>>) {
    for mut controller in &mut query {
        controller.translation = Some(Vec2::new(0., -1.));
    }
}
