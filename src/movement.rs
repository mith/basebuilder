use bevy::{math::Vec3Swizzles, prelude::*, sprite::Mesh2dHandle};
use bevy_rapier2d::prelude::{
    KinematicCharacterController, KinematicCharacterControllerOutput, QueryFilter, RapierContext,
    RayIntersection,
};

#[derive(Component, Default)]
pub(crate) struct Walker {
    pub(crate) move_direction: Option<Vec2>,
}
#[derive(Component, Default)]
pub(crate) struct Jumper {
    pub(crate) jump_timer: Option<Timer>,
    pub(crate) jump: bool,
}

fn walk(
    mut dude_query: Query<(
        &mut KinematicCharacterController,
        &KinematicCharacterControllerOutput,
        &Walker,
        Option<&mut Jumper>,
    )>,
    time: Res<Time>,
) {
    for (mut controller, controller_output, walker, mut maybe_jumper) in &mut dude_query {
        if let Some(jumper) = maybe_jumper.as_mut() {
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

#[derive(Component)]
pub(crate) struct Hands;

#[derive(Component, Default)]
pub(crate) struct Aim {
    pub(crate) aim_direction: Option<f32>,
}

#[derive(Component)]
pub(crate) struct AimingAt {
    pub(crate) target: Entity,
    pub(crate) intersection: RayIntersection,
}

fn aim(
    mut commands: Commands,
    mut hands_query: Query<(&mut Transform, &Parent), (With<Hands>, Without<Aim>)>,
    aimer_query: Query<(&Aim, &Transform, Entity)>,
    rapier_context: Res<RapierContext>,
) {
    for (mut hands_transform, parent) in &mut hands_query {
        let Ok((input, player_transform, player_entity)) = aimer_query.get(parent.get()) else {
            continue;
        };

        if let Some(aim_direction) = input.aim_direction {
            let rotation = Quat::from_rotation_z(aim_direction);
            hands_transform.rotation = rotation;

            let max_distance = 1000.;
            let filter = QueryFilter::default().exclude_collider(player_entity);
            if let Some((hit_entity, intersection)) = rapier_context.cast_ray_and_get_normal(
                player_transform.translation.xy(),
                hands_transform
                    .rotation
                    .mul_vec3(Vec3::new(1., 0., 0.))
                    .xy(),
                max_distance,
                true,
                filter,
            ) {
                commands.entity(player_entity).insert(AimingAt {
                    target: hit_entity,
                    intersection,
                });
            } else {
                commands.entity(player_entity).remove::<AimingAt>();
            };
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct MovementSet;

pub(crate) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((walk, aim).in_set(MovementSet));
    }
}
