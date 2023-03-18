use bevy::{math::Vec3Swizzles, prelude::*};

use crate::{
    cursor_position::CursorPosition,
    movement::{Aim, Jumper, MovementSet, Walker},
    shoot::Shooter,
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct PlayerControllerSet;

#[derive(Component)]
pub(crate) struct PlayerControlled;

fn keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Walker, Option<&mut Jumper>), With<PlayerControlled>>,
) {
    for (mut dude_input, mut maybe_jumper) in &mut query {
        if !keyboard_input.any_pressed([KeyCode::Left, KeyCode::Right, KeyCode::A, KeyCode::E]) {
            dude_input.move_direction = None;
        } else {
            let mut direction = Vec2::ZERO;
            if keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A) {
                direction += Vec2::new(-1., 0.);
            }
            if keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::E) {
                direction += Vec2::new(1., 0.);
            }

            dude_input.move_direction = Some(direction.normalize());
        }

        if let Some(jumper) = maybe_jumper.as_mut() {
            jumper.jump = keyboard_input.just_pressed(KeyCode::Space);
        }
    }
}

fn mouse_input(
    cursor_position: Res<CursorPosition>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Aim, Option<&mut Shooter>, &GlobalTransform), With<PlayerControlled>>,
) {
    for (mut aim, mut shooter, dude_transform) in &mut query {
        // calculate angle between cursor and dude
        let angle = angle_between_dude_and_position(&cursor_position, dude_transform);
        aim.aim_direction = Some(angle);

        if let Some(shooter) = shooter.as_mut() {
            if mouse_button_input.pressed(MouseButton::Left) {
                shooter.shoot = true;
            } else {
                shooter.shoot = false;
            }
        }
    }
}

fn angle_between_dude_and_position(
    cursor_position: &CursorPosition,
    dude_transform: &GlobalTransform,
) -> f32 {
    let direction = (cursor_position.0.xy() - dude_transform.translation().truncate()).normalize();
    let angle = Vec2::new(1., 0.).angle_between(direction);

    if angle < 0. {
        2. * std::f32::consts::PI + angle
    } else {
        angle
    }
}

pub(crate) struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (keyboard_input, mouse_input)
                .in_set(PlayerControllerSet)
                .before(MovementSet),
        );
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use super::*;

    #[test]
    fn angle_between_dude_and_position_up() {
        let cursor_position = CursorPosition(Vec3::new(0., 1., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = PI / 2.;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }

    #[test]
    fn angle_between_dude_and_position_down() {
        let cursor_position = CursorPosition(Vec3::new(0., -1., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = 3. * PI / 2.;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }

    #[test]
    fn angle_between_dude_and_position_left() {
        let cursor_position = CursorPosition(Vec3::new(-1., 0., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = PI;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }

    #[test]
    fn angle_between_dude_and_position_right() {
        let cursor_position = CursorPosition(Vec3::new(1., 0., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = 0.;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }

    #[test]
    fn angle_between_dude_and_position_up_far() {
        let cursor_position = CursorPosition(Vec3::new(0., 100., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(0., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = PI / 2.;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }

    #[test]
    fn angle_between_dude_and_position_up_dude_translated() {
        let cursor_position = CursorPosition(Vec3::new(10., 1., 0.));
        let dude_transform = GlobalTransform::from_translation(Vec3::new(10., 0., 0.));
        let angle = angle_between_dude_and_position(&cursor_position, &dude_transform);
        let expected_angle = PI / 2.;
        let diff = (angle - expected_angle).abs();
        assert!(diff < 0.0001);
    }
}
