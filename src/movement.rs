use bevy::{math::Vec3Swizzles, prelude::*, sprite::Mesh2dHandle};
use bevy_rapier2d::prelude::{
    KinematicCharacterController, KinematicCharacterControllerOutput, QueryFilter, RapierContext,
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
pub(crate) struct AimingLaser;

#[derive(Component, Default)]
pub(crate) struct Aim {
    pub(crate) aim_direction: Option<f32>,
}

#[derive(Component)]
pub(crate) struct AimingAt(pub(crate) Entity);

fn aim(
    mut commands: Commands,
    mut laser_query: Query<
        (&mut Transform, &Mesh2dHandle, &Parent),
        (With<AimingLaser>, Without<Aim>),
    >,
    aimer_query: Query<(&Aim, &Transform, Entity)>,
    rapier_context: Res<RapierContext>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut laser_transform, laser_mesh_handle, parent) in &mut laser_query {
        let Ok((input, dude_transform, dude_entity)) = aimer_query.get(parent.get()) else {
            continue;
        };

        if let Some(aim_direction) = input.aim_direction {
            let rotation = Quat::from_rotation_z(aim_direction);
            laser_transform.rotation = rotation;

            let max_distance = 1000.;
            let filter = QueryFilter::default().exclude_collider(dude_entity);
            if let Some((hit_entity, toi)) = rapier_context.cast_ray(
                dude_transform.translation.xy(),
                laser_transform
                    .rotation
                    .mul_vec3(Vec3::new(1., 0., 0.))
                    .xy(),
                max_distance,
                true,
                filter,
            ) {
                laser_transform.translation = rotation.mul_vec3(Vec3::new(toi / 2., 0., 1.));
                if let Some(mesh) = meshes.get_mut(&laser_mesh_handle.0) {
                    *mesh = Mesh::from(shape::Quad::new(Vec2::new(toi, 0.2)));
                }
                commands.entity(dude_entity).insert(AimingAt(hit_entity));
            } else {
                commands.entity(dude_entity).remove::<AimingAt>();
                laser_transform.translation =
                    rotation.mul_vec3(Vec3::new(max_distance / 2., 0., 1.));
                if let Some(mesh) = meshes.get_mut(&laser_mesh_handle.0) {
                    *mesh = Mesh::from(shape::Quad::new(Vec2::new(max_distance, 0.2)));
                }
            }
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
