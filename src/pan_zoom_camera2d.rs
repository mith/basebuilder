use bevy::{input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};

pub struct PanZoomCamera2dPlugin;

impl Plugin for PanZoomCamera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((camera_zoom, drag_camera));
    }
}

#[derive(Component)]
pub struct PanZoomCamera2d {
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for PanZoomCamera2d {
    fn default() -> Self {
        Self {
            zoom_speed: 0.1,
            min_zoom: 0.01,
            max_zoom: 10.0,
        }
    }
}

#[derive(Bundle, Default)]
pub struct PanZoomCamera2dBundle {
    pub camera: Camera2dBundle,
    pub pan_zoom: PanZoomCamera2d,
}

fn camera_zoom(
    mut camera_query: Query<(
        &mut Transform,
        &mut OrthographicProjection,
        &PanZoomCamera2d,
    )>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(primary_window) = window_query.get_single() else {
        return;
    };
    let Some(cursor_position) = primary_window.cursor_position() else {
        return;
    };

    for event in mouse_wheel_events.iter() {
        for (mut transform, mut ortho, camera) in camera_query.iter_mut() {
            let old_scale = ortho.scale;
            let mut zoom_change = ortho.scale * event.y.clamp(-1., 1.) * camera.zoom_speed;
            ortho.scale -= zoom_change;

            if ortho.scale < camera.min_zoom {
                ortho.scale = camera.min_zoom;
                zoom_change = old_scale - ortho.scale;
            } else if ortho.scale > camera.max_zoom {
                ortho.scale = camera.max_zoom;
                zoom_change = old_scale - ortho.scale;
            }

            // Move the camera toward the cursor position to keep the current object
            // underneath it.
            let from_center = cursor_position
                - Vec2::new(primary_window.width() / 2., primary_window.height() / 2.);

            let scaled_move = from_center * event.y.clamp(-1., 1.) * zoom_change.abs();
            transform.translation += Vec3::new(scaled_move.x, scaled_move.y, 0.);
        }
    }
}

#[derive(Component)]
struct DragStart {
    cursor_position: Vec2,
    camera_position: Vec3,
}

fn drag_camera(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    undragged_cameras_query: Query<
        (Entity, &Transform),
        (With<PanZoomCamera2d>, Without<DragStart>),
    >,
    mut dragged_cameras_query: Query<
        (Entity, &mut Transform, &OrthographicProjection, &DragStart),
        (With<PanZoomCamera2d>, With<DragStart>),
    >,
) {
    let window = window_query.single();

    let Some(current_cursor_position) = window.cursor_position() else {
        return;
    };

    if mouse_button_input.pressed(MouseButton::Middle) {
        for (undragged_camera_entity, undragged_camera_transform) in &undragged_cameras_query {
            commands.entity(undragged_camera_entity).insert(DragStart {
                cursor_position: current_cursor_position,
                camera_position: undragged_camera_transform.translation,
            });
        }
    }

    for (camera_entity, mut transform, ortho, drag_start) in &mut dragged_cameras_query {
        if !mouse_button_input.pressed(MouseButton::Middle) {
            commands.entity(camera_entity).remove::<DragStart>();
        }

        let diff = current_cursor_position - drag_start.cursor_position;
        let z = transform.translation.z;
        transform.translation =
            drag_start.camera_position - Vec3::new(diff.x, diff.y, 0.) * ortho.scale;
        transform.translation.z = z;
    }
}
