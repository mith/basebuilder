use bevy::prelude::*;

pub(crate) fn cursor_position_in_world(
    windows: &Windows,
    cursor_position: Vec2,
    camera_transform: &GlobalTransform,
    camera: &Camera,
) -> Vec3 {
    let window = windows.primary();

    let window_size = Vec2::new(window.width(), window.height());

    // Convert screen position [0..resolution] to ndc [-1..1]
    // (ndc = normalized device coordinates)
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
    let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;
    ndc_to_world.project_point3(ndc.extend(0.0))
}

#[derive(Default, Resource)]
pub(crate) struct CursorPosition(pub(crate) Vec3);

fn update_cursor_pos(
    windows: Res<Windows>,
    camera_query: Query<(&GlobalTransform, &Camera)>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_position: ResMut<CursorPosition>,
) {
    if let Some(cursor_moved) = cursor_moved_events.iter().last() {
        for (camera_transform, camera) in &camera_query {
            *cursor_position = CursorPosition(cursor_position_in_world(
                &windows,
                cursor_moved.position,
                camera_transform,
                camera,
            ))
        }
    }
}

pub(crate) struct CursorPositionPlugin;

impl Plugin for CursorPositionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorPosition(Vec3::ZERO))
            .add_system(update_cursor_pos);
    }
}