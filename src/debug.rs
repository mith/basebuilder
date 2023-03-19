use bevy::{input::common_conditions::input_toggle_active, prelude::*};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};

use crate::pan_zoom_camera2d::{PanZoomCamera2d, PanZoomCamera2dBundle};

#[derive(Resource, Default)]
pub(crate) struct Debug;

fn toggle_debug(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    debug: Option<Res<Debug>>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        if debug.is_none() {
            commands.init_resource::<Debug>();
        } else {
            commands.remove_resource::<Debug>();
        }
    }
}

fn toggle_physics_debug(
    mut debug_render: ResMut<DebugRenderContext>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    debug_render.enabled = !debug_render.enabled;
}

fn spawn_debug_camera(mut commands: Commands) {
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

fn toggle_debug_camera(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<Entity, With<PanZoomCamera2d>>,
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    } else {
        spawn_debug_camera(commands);
    }
}

pub(crate) struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "inspector")]
        app.add_plugin(WorldInspectorPlugin::default().run_if(resource_exists::<Debug>()));

        app.add_plugin(RapierDebugRenderPlugin {
            enabled: false,
            ..default()
        })
        .add_system(toggle_physics_debug.run_if(resource_added::<Debug>()))
        .add_system(toggle_debug_camera.run_if(resource_added::<Debug>()))
        .add_system(toggle_debug);
    }
}
