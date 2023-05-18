use bevy::{input::common_conditions::input_toggle_active, prelude::*};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};

use crate::pan_zoom_camera2d::{PanZoomCamera2d, PanZoomCamera2dBundle};

#[derive(Resource)]
struct Inspector;

fn toggle_inspector(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    maybe_inspector: Option<Res<Inspector>>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        if maybe_inspector.is_some() {
            commands.remove_resource::<Inspector>();
        } else {
            commands.insert_resource(Inspector);
        }
    }
}

fn toggle_physics_debug(
    mut debug_render: ResMut<DebugRenderContext>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::F2) {
        debug_render.enabled = !debug_render.enabled;
    }
}

fn spawn_freelook_camera(mut commands: Commands) {
    commands.spawn(PanZoomCamera2dBundle {
        camera: Camera2dBundle {
            camera: Camera {
                order: 100,
                ..default()
            },
            ..default()
        },
        ..default()
    });
}

fn toggle_freelook_camera(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    query: Query<Entity, With<PanZoomCamera2d>>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        if let Ok(entity) = query.get_single() {
            commands.entity(entity).despawn_recursive();
        } else {
            spawn_freelook_camera(commands);
        }
    }
}

pub(crate) struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "inspector")]
        app.add_plugin(WorldInspectorPlugin::default().run_if(resource_exists::<Inspector>()));

        app.add_plugin(RapierDebugRenderPlugin {
            enabled: false,
            ..default()
        })
        .add_system(toggle_physics_debug)
        .add_system(toggle_freelook_camera)
        .add_system(toggle_inspector);
    }
}
