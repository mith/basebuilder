use bevy::{math::Vec3Swizzles, prelude::*};

use crate::{
    cursor_position::CursorPosition,
    dude::{Dude, DudeInput},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct PlayerControllerSet;

fn keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut DudeInput, With<Dude>>,
) {
    for mut dude_input in &mut query {
        if !keyboard_input.any_pressed([KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down])
        {
            dude_input.move_direction = None;
        } else {
            let mut direction = Vec2::ZERO;
            if keyboard_input.pressed(KeyCode::Left) {
                direction += Vec2::new(-1., 0.);
            }
            if keyboard_input.pressed(KeyCode::Right) {
                direction += Vec2::new(1., 0.);
            }
            if keyboard_input.pressed(KeyCode::Up) {
                direction += Vec2::new(0., 1.);
            }
            if keyboard_input.pressed(KeyCode::Down) {
                direction += Vec2::new(0., -1.);
            }
            dude_input.move_direction = Some(direction.normalize());
        }
    }
}

fn mouse_input(
    cursor_position: Res<CursorPosition>,
    mut query: Query<(&mut DudeInput, &GlobalTransform), With<Dude>>,
) {
    for (mut dude_input, dude_transform) in &mut query {
        // calculate angle between cursor and dude
        let angle = angle_between_dude_and_position(&cursor_position, dude_transform);
        dude_input.aim_direction = Some(angle);
    }
}

fn angle_between_dude_and_position(cursor_position: &CursorPosition, dude_transform: &GlobalTransform) -> f32 {
    let direction =
        (cursor_position.0.xy() - dude_transform.translation().truncate()).normalize();
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
        app.add_system(keyboard_input.in_set(PlayerControllerSet))
            .add_system(mouse_input.in_set(PlayerControllerSet));
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
