use bevy::{prelude::*, window::PrimaryWindow};
pub struct CursorPositionPlugin;

impl Plugin for CursorPositionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LastCursorPosition(Vec2::ZERO))
            .add_systems(Update, update_cursor_pos.in_set(CursorPositionSet));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CursorPositionSet;

pub fn cursor_position_in_world(
    window: &Window,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Option<Vec2> {
    window
        .cursor_position()
        .and_then(|pos| camera.viewport_to_world_2d(camera_transform, pos))
}

#[derive(Default, Resource)]
pub struct LastCursorPosition(pub Vec2);

fn update_cursor_pos(
    window: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut cursor_position: ResMut<LastCursorPosition>,
) {
    for (camera_transform, camera) in &camera_query {
        if let Some(cursor_position_in_world) =
            cursor_position_in_world(&window.single(), camera_transform, camera)
        {
            cursor_position.0 = cursor_position_in_world;
        }
    }
}
