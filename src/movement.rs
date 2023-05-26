use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{KinematicCharacterController, KinematicCharacterControllerOutput};

use crate::{climbable::ClimbableMap, gravity::Gravity, terrain::TerrainParams};

pub(crate) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((walk, climb).in_set(MovementSet));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct MovementSet;

#[derive(Component, Default)]
pub(crate) struct Walker {
    pub(crate) move_direction: Option<Vec2>,
}
#[derive(Component, Default)]
pub(crate) struct Jumper {
    pub(crate) jump_timer: Option<Timer>,
    pub(crate) jump: bool,
}

#[derive(Component)]
pub(crate) struct Climber;

#[derive(Component)]
pub(crate) struct Climbing;

fn walk(
    mut dude_query: Query<(
        &mut KinematicCharacterController,
        &Walker,
        Option<(&mut Jumper, &KinematicCharacterControllerOutput)>,
    )>,
    time: Res<Time>,
) {
    for (mut controller, walker, mut maybe_jumper) in &mut dude_query {
        if let Some((jumper, controller_output)) = maybe_jumper.as_mut() {
            if jumper.jump && controller_output.grounded {
                jumper.jump_timer = Some(Timer::from_seconds(0.5, TimerMode::Once));
            }

            if let Some(timer) = jumper.jump_timer.as_mut() {
                if timer.tick(time.delta()).just_finished() {
                    jumper.jump_timer = None;
                } else {
                    controller.translation = Some(Vec2::new(0., 2.));
                }
            }
        }

        let move_direction = walker.move_direction.map(|dir| Vec2::new(dir.x, 0.));

        controller.translation = controller.translation.map_or(move_direction, |t| {
            Some(t + move_direction.unwrap_or_default())
        });
    }
}

fn climb(
    mut commands: Commands,
    mut climber_query: Query<(Entity, &GlobalTransform), With<Climber>>,
    climbable_map_query: Query<&ClimbableMap>,
    terrain: TerrainParams,
) {
    for (climber_entity, climber_transform) in &mut climber_query {
        for climbable_map in &climbable_map_query {
            let climber_tile_pos = terrain
                .global_to_tile_pos(climber_transform.translation().xy())
                .unwrap();

            if climbable_map.is_climbable(climber_tile_pos.into()) {
                commands
                    .entity(climber_entity)
                    .insert(Climbing)
                    .remove::<Gravity>();
            } else {
                commands
                    .entity(climber_entity)
                    .remove::<Climbing>()
                    .insert(Gravity);
            }
        }
    }
}
