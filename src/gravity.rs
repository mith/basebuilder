use bevy::prelude::*;
use bevy_rapier2d::prelude::KinematicCharacterController;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct GravitySet;

#[derive(Component)]
pub(crate) struct Gravity;

fn gravity(mut query: Query<&mut KinematicCharacterController, With<Gravity>>) {
    for mut controller in &mut query {
        controller.translation = Some(
            controller
                .translation
                .map_or(Vec2::new(0., -1.), |t| t + Vec2::new(0., -1.)),
        );
    }
}
pub(crate) struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(gravity.in_set(GravitySet));
    }
}
