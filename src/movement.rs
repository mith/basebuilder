use bevy::{math::Vec3Swizzles, prelude::*};

use bevy_rapier2d::prelude::{
    Collider, CollisionGroups, KinematicCharacterController, KinematicCharacterControllerOutput,
    QueryFilter, RapierContext,
};

use crate::{
    climbable::ClimbableMap,
    dwarf::DWARF_COLLISION_GROUP,
    terrain::{TerrainParams, TERRAIN_COLLISION_GROUP},
};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((walk, fall).in_set(MovementSet));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub struct MovementSet;

#[derive(Component, Default)]
pub struct Walker {
    pub move_direction: Option<Vec2>,
}
#[derive(Component, Default)]
pub struct Jumper {
    pub jump_timer: Option<Timer>,
    pub jump: bool,
}

#[derive(Component)]
pub struct Climber;

#[derive(Component)]
pub struct Climbing;

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

        let move_direction = walker.move_direction.map(|dir| Vec2::new(dir.x, dir.y));

        controller.translation = controller
            .translation
            .map_or(move_direction, |translation| {
                move_direction.map(|dir| translation + dir)
            });
    }
}

#[derive(Component)]
pub struct Falling;

fn fall(
    mut commands: Commands,
    rapier_context: Res<RapierContext>,
    mut climber_query: Query<
        (Entity, &mut KinematicCharacterController, &GlobalTransform),
        With<Climber>,
    >,
    climbable_map_query: Query<&ClimbableMap>,
    terrain: TerrainParams,
) {
    for (climber_entity, mut controller, climber_transform) in &mut climber_query {
        for climbable_map in &climbable_map_query {
            let climber_tile_pos = terrain
                .global_to_tile_pos(climber_transform.translation().xy())
                .unwrap();

            let shape_pos = climber_transform.translation().xy() - Vec2::new(0., 6.);
            let shape_rot = 0.;
            let shape = Collider::cuboid(6., 0.2);
            let filter = QueryFilter::default().groups(CollisionGroups::new(
                DWARF_COLLISION_GROUP,
                TERRAIN_COLLISION_GROUP,
            ));
            let is_grounded = rapier_context
                .intersection_with_shape(shape_pos, shape_rot, &shape, filter)
                .is_some();

            if !climbable_map.is_climbable(climber_tile_pos.into()) && !is_grounded {
                commands.entity(climber_entity).insert(Falling);
                controller.translation = Some(Vec2::new(0., -1.));
            } else {
                commands.entity(climber_entity).remove::<Falling>();
            }
        }
    }
}
